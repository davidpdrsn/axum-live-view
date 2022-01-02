use super::{wrap_in_liveview_container, LiveView, LiveViewId, Subscriptions, Updated};
use crate::{
    html::Html,
    js_command::JsCommand,
    pubsub::{PubSub, PubSubError},
    topics::{self, RenderedMessage},
};
use anyhow::Context;
use async_stream::stream;
use axum::Json;
use futures_util::stream::BoxStream;
use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::StreamExt as _;

enum State<T, P>
where
    T: LiveView,
{
    Initial {
        liveview_id: LiveViewId,
        liveview: T,
        pubsub: P,
        mount_stream: BoxStream<'static, ()>,
    },
    Running {
        liveview_id: LiveViewId,
        pubsub: P,
        mounted_streams_count: usize,
        markup: Html<T::Message>,
        stream: BoxStream<'static, MessageForLiveView<T>>,
    },
    EveryoneDisconnected,
}

enum MessageForLiveView<T>
where
    T: LiveView,
{
    Mounted,
    Rendered(Option<Html<T::Message>>, Vec<JsCommand>),
    WebSocketDisconnected,
}

pub(super) async fn run_liveview<T, P>(
    liveview_id: LiveViewId,
    liveview: T,
    pubsub: P,
) -> anyhow::Result<()>
where
    T: LiveView,
    P: PubSub + Clone,
{
    let mount_stream = pubsub
        .subscribe(&topics::mounted(liveview_id))
        .await
        .map_err(PubSubError::boxed)
        .context("subscribing to mounted topic")?;

    let mut state = State::Initial {
        liveview_id,
        liveview,
        pubsub,
        mount_stream,
    };

    tracing::trace!("liveview update loop running");

    loop {
        state = next_state(state)
            .await
            .context("failed to compute next state")?;

        match state {
            State::Initial { liveview_id, .. } => {
                tracing::warn!(?liveview_id, "liveview going into `Initial` state")
            }
            State::Running {
                liveview_id,
                mounted_streams_count,
                ..
            } => tracing::trace!(
                ?liveview_id,
                ?mounted_streams_count,
                "liveview going into `Running` state"
            ),
            State::EveryoneDisconnected => {
                tracing::trace!("liveview going into `EveryoneDisconnected` state");
            }
        }

        if matches!(state, State::EveryoneDisconnected) {
            tracing::trace!(%liveview_id, "shutting down liveview task");
            break;
        }
    }

    Ok(())
}

#[allow(unreachable_code)]
async fn next_state<T, P>(state: State<T, P>) -> anyhow::Result<State<T, P>>
where
    T: LiveView,
    P: PubSub + Clone,
{
    match state {
        State::Initial {
            liveview_id,
            liveview,
            pubsub,
            mut mount_stream,
        } => {
            if timeout(Duration::from_secs(30), mount_stream.next())
                .await
                .is_err()
            {
                tracing::warn!("liveview mount timeout elapsed");
                return Ok(State::EveryoneDisconnected);
            }

            let mount_stream = mount_stream.map(|_| MessageForLiveView::Mounted);

            let markup = wrap_in_liveview_container(liveview_id, liveview.render());

            pubsub
                .broadcast(
                    &topics::initial_render(liveview_id),
                    Json(markup.serialize()),
                )
                .await
                .map_err(PubSubError::boxed)
                .context("failed to publish initial render markup")?;

            let markup_updates_stream =
                markup_updates_stream(liveview, pubsub.clone(), liveview_id)
                    .await
                    .context("failed to create markup updates stream")?
                    .map(|(markup, js_commands)| MessageForLiveView::Rendered(markup, js_commands));

            let disconnected_stream = pubsub
                .subscribe(&topics::socket_disconnected(liveview_id))
                .await
                .map_err(PubSubError::boxed)
                .context("failed to subscribe to socket disconnected")?
                .map(|_| MessageForLiveView::WebSocketDisconnected);

            let stream = futures_util::stream::pending()
                .merge(mount_stream)
                .merge(markup_updates_stream)
                .merge(disconnected_stream);
            let stream = Box::pin(stream);

            Ok(State::Running {
                liveview_id,
                pubsub,
                mounted_streams_count: 1,
                markup,
                stream,
            })
        }

        State::Running {
            liveview_id,
            pubsub,
            mut mounted_streams_count,
            mut markup,
            mut stream,
        } => {
            if mounted_streams_count == 0 {
                return Ok(State::EveryoneDisconnected);
            }

            let msg = if let Some(msg) = stream.next().await {
                msg
            } else {
                tracing::error!("internal liveview streams all ended. This is a bug");
                return Ok(State::EveryoneDisconnected);
            };

            match msg {
                MessageForLiveView::Mounted => {
                    tracing::trace!(?liveview_id, "liveview mounted on another websocket");

                    mounted_streams_count += 1;
                    let _ = pubsub
                        .broadcast(
                            &topics::initial_render(liveview_id),
                            Json(markup.serialize()),
                        )
                        .await;
                }

                MessageForLiveView::Rendered(new_markup, js_commands) => {
                    tracing::trace!(?liveview_id, "liveview re-rendered its markup");

                    let new_markup = new_markup
                        .map(|new_markup| wrap_in_liveview_container(liveview_id, new_markup));

                    let diff = new_markup
                        .as_ref()
                        .and_then(|new_markup| markup.diff(new_markup));

                    if let Some(new_markup) = new_markup {
                        markup = new_markup;
                    }

                    match diff {
                        Some(diff) if js_commands.is_empty() => {
                            let _ = pubsub
                                .broadcast(
                                    &topics::rendered(liveview_id),
                                    Json(RenderedMessage::Diff(diff)),
                                )
                                .await;
                        }
                        Some(diff) => {
                            let _ = pubsub
                                .broadcast(
                                    &topics::rendered(liveview_id),
                                    Json(RenderedMessage::DiffWithCommands(diff, js_commands)),
                                )
                                .await;
                        }
                        None if !js_commands.is_empty() => {
                            let _ = pubsub
                                .broadcast(
                                    &topics::rendered(liveview_id),
                                    Json(RenderedMessage::Commands(js_commands)),
                                )
                                .await;
                        }
                        None => {}
                    }
                }

                MessageForLiveView::WebSocketDisconnected => {
                    tracing::trace!(?liveview_id, "socket disconnected from liveview");
                    mounted_streams_count -= 1;
                }
            }

            Ok(State::Running {
                liveview_id,
                pubsub,
                mounted_streams_count,
                markup,
                stream,
            })
        }

        State::EveryoneDisconnected => Ok(State::EveryoneDisconnected),
    }
}

async fn markup_updates_stream<T, P>(
    mut liveview: T,
    pubsub: P,
    liveview_id: LiveViewId,
) -> anyhow::Result<BoxStream<'static, (Option<Html<T::Message>>, Vec<JsCommand>)>>
where
    T: LiveView,
    P: PubSub,
{
    let mut subscriptions = Subscriptions::new(liveview_id);
    liveview.init(&mut subscriptions);

    let mut stream = subscriptions.into_stream(pubsub).await?;

    Ok(Box::pin(stream! {
        while let Some((callback, msg)) = stream.next().await {
            let Updated {
                liveview: new_liveview,
                js_commands,
                skip_render,
            } = callback.call(liveview, msg).await;
            liveview = new_liveview;

            if skip_render {
                yield (None, js_commands);
            } else {
                let markup = liveview.render();
                yield (Some(markup), js_commands);
            }
        }
    }))
}
