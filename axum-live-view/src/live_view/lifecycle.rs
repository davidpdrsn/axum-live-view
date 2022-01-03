use super::{
    wrap_in_live_view_container, LiveView, LiveViewId, MakeLiveView, Subscriptions, Updated,
};
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
use tokio_stream::StreamExt as _;

enum State<T, P>
where
    T: LiveView,
{
    Initial {
        live_view_id: LiveViewId,
        live_view: T,
        pubsub: P,
    },
    WaitingForFirstMount {
        live_view_id: LiveViewId,
        live_view: T,
        pubsub: P,
        mount_stream: BoxStream<'static, ()>,
    },
    Running {
        live_view_id: LiveViewId,
        pubsub: P,
        markup: Html<T::Message>,
        markup_updates_stream: BoxStream<'static, (Option<Html<T::Message>>, Vec<JsCommand>)>,
        disconnected_stream: BoxStream<'static, ()>,
    },
    WaitForReMount {
        live_view_id: LiveViewId,
        markup_updates_stream: BoxStream<'static, (Option<Html<T::Message>>, Vec<JsCommand>)>,
        pubsub: P,
        mount_stream: BoxStream<'static, ()>,
        markup: Html<T::Message>,
    },
    Close {
        live_view_id: LiveViewId,
    },
}

enum MessageForLiveView<T>
where
    T: LiveView,
{
    Rendered(Option<Html<T::Message>>, Vec<JsCommand>),
    WebSocketDisconnected,
}

pub(super) async fn run_live_view<M, P>(
    live_view_id: LiveViewId,
    live_view: M::LiveView,
    make_live_view: M,
    pubsub: P,
) where
    M: MakeLiveView,
    P: PubSub + Clone,
{
    if let Err(err) = try_run_live_view(live_view_id, live_view, make_live_view, pubsub).await {
        tracing::error!(?err, "live task finished with error");
    }
}

async fn try_run_live_view<M, P>(
    live_view_id: LiveViewId,
    live_view: M::LiveView,
    make_live_view: M,
    pubsub: P,
) -> anyhow::Result<()>
where
    M: MakeLiveView,
    P: PubSub + Clone,
{
    let mut state = State::Initial {
        live_view_id,
        live_view,
        pubsub,
    };

    tracing::trace!("live_view update loop running");

    loop {
        state = next_state(state)
            .await
            .context("failed to compute next state")?;

        match state {
            State::Initial { live_view_id, .. } => {
                tracing::trace!(?live_view_id, "live_view going into `Initial` state")
            }
            State::WaitingForFirstMount { live_view_id, .. } => {
                tracing::trace!(
                    ?live_view_id,
                    "live_view going into `WaitingForFirstMount` state"
                )
            }
            State::Running { live_view_id, .. } => {
                tracing::trace!(?live_view_id, "live_view going into `Running` state")
            }
            State::WaitForReMount { live_view_id, .. } => {
                tracing::trace!(?live_view_id, "live_view going into `WaitForReMount` state")
            }
            State::Close { live_view_id } => {
                tracing::trace!(?live_view_id, "live_view going into `Close` state");
                break;
            }
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
            live_view_id,
            live_view,
            pubsub,
        } => {
            let mount_stream = pubsub
                .subscribe(&topics::mounted(live_view_id))
                .await
                .map_err(PubSubError::boxed)
                .context("subscribing to mounted topic")?;

            Ok(State::WaitingForFirstMount {
                live_view_id,
                live_view,
                pubsub: pubsub.clone(),
                mount_stream,
            })
        }

        State::WaitingForFirstMount {
            live_view_id,
            live_view,
            pubsub,
            mut mount_stream,
        } => {
            tracing::trace!("live view is waiting to be mounted");
            mount_stream.next().await;
            tracing::trace!("live view mounted");

            let markup = wrap_in_live_view_container(live_view_id, live_view.render());

            pubsub
                .broadcast(
                    &topics::initial_render(live_view_id),
                    Json(markup.serialize()),
                )
                .await
                .map_err(PubSubError::boxed)
                .context("failed to publish initial render markup")?;

            let markup_updates_stream =
                markup_updates_stream(live_view, pubsub.clone(), live_view_id)
                    .await
                    .context("failed to create markup updates stream")?;

            let disconnected_stream = pubsub
                .subscribe(&topics::socket_disconnected(live_view_id))
                .await
                .map_err(PubSubError::boxed)
                .context("failed to subscribe to socket disconnected")?
                .map(|_| ());

            Ok(State::Running {
                live_view_id,
                pubsub,
                markup,
                markup_updates_stream: Box::pin(markup_updates_stream),
                disconnected_stream: Box::pin(disconnected_stream),
            })
        }

        State::Running {
            live_view_id,
            pubsub,
            mut markup,
            mut markup_updates_stream,
            mut disconnected_stream,
        } => {
            let msg = tokio::select! {
                Some((new_markup, js_commands)) = markup_updates_stream.next() => {
                    MessageForLiveView::<T>::Rendered(new_markup, js_commands)
                }
                Some(()) = disconnected_stream.next() => {
                    MessageForLiveView::WebSocketDisconnected
                }
            };

            match msg {
                MessageForLiveView::Rendered(new_markup, js_commands) => {
                    tracing::trace!(?live_view_id, "live_view re-rendered its markup");

                    let new_markup = new_markup
                        .map(|new_markup| wrap_in_live_view_container(live_view_id, new_markup));

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
                                    &topics::rendered(live_view_id),
                                    Json(RenderedMessage::Diff(diff)),
                                )
                                .await;
                        }
                        Some(diff) => {
                            let _ = pubsub
                                .broadcast(
                                    &topics::rendered(live_view_id),
                                    Json(RenderedMessage::DiffWithCommands(diff, js_commands)),
                                )
                                .await;
                        }
                        None if !js_commands.is_empty() => {
                            let _ = pubsub
                                .broadcast(
                                    &topics::rendered(live_view_id),
                                    Json(RenderedMessage::Commands(js_commands)),
                                )
                                .await;
                        }
                        None => {}
                    }

                    Ok(State::Running {
                        live_view_id,
                        pubsub,
                        markup,
                        markup_updates_stream,
                        disconnected_stream,
                    })
                }

                MessageForLiveView::WebSocketDisconnected => {
                    tracing::trace!(?live_view_id, "socket disconnected from live_view");

                    let mount_stream = pubsub
                        .subscribe(&topics::mounted(live_view_id))
                        .await
                        .map_err(PubSubError::boxed)
                        .context("subscribing to mounted topic")?;

                    Ok(State::WaitForReMount {
                        live_view_id,
                        pubsub,
                        markup_updates_stream,
                        mount_stream,
                        markup,
                    })
                }
            }
        }

        State::WaitForReMount {
            live_view_id,
            markup_updates_stream,
            pubsub,
            markup,
            mut mount_stream,
        } => {
            tracing::trace!("live view is waiting to be re-mounted");
            mount_stream.next().await;
            tracing::trace!("live view re-mounted");

            pubsub
                .broadcast(
                    &topics::initial_render(live_view_id),
                    Json(markup.serialize()),
                )
                .await
                .map_err(PubSubError::boxed)
                .context("failed to publish initial render markup")?;

            let disconnected_stream = pubsub
                .subscribe(&topics::socket_disconnected(live_view_id))
                .await
                .map_err(PubSubError::boxed)
                .context("failed to subscribe to socket disconnected")?
                .map(|_| ());

            Ok(State::Running {
                live_view_id,
                pubsub,
                markup,
                markup_updates_stream,
                disconnected_stream: Box::pin(disconnected_stream),
            })
        }

        State::Close { live_view_id } => Ok(State::Close { live_view_id }),
    }
}

async fn markup_updates_stream<T, P>(
    mut live_view: T,
    pubsub: P,
    live_view_id: LiveViewId,
) -> anyhow::Result<BoxStream<'static, (Option<Html<T::Message>>, Vec<JsCommand>)>>
where
    T: LiveView,
    P: PubSub,
{
    let mut subscriptions = Subscriptions::new(live_view_id);
    live_view.init(&mut subscriptions);

    let mut stream = subscriptions.into_stream(pubsub).await?;

    Ok(Box::pin(stream! {
        while let Some((callback, msg)) = stream.next().await {
            let Updated {
                live_view: new_live_view,
                js_commands,
                skip_render,
            } = callback.call(live_view, msg).await;
            live_view = new_live_view;

            if skip_render {
                yield (None, js_commands);
            } else {
                let markup = live_view.render();
                yield (Some(markup), js_commands);
            }
        }
    }))
}
