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
use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::StreamExt as _;

type OwnedStream<T = ()> = BoxStream<'static, T>;
type MarkupUpdatesStream<M> = OwnedStream<(Option<Html<M>>, Vec<JsCommand>)>;

enum State<T, M>
where
    T: LiveView,
    M: MakeLiveView<LiveView = T>,
{
    Initial {
        live_view: T,
        make_live_view: M,
    },
    WaitingForFirstMount {
        live_view: T,
        make_live_view: M,
        mount_stream: OwnedStream,
    },
    Running {
        disconnected_stream: OwnedStream,
        markup: Html<T::Message>,
        markup_updates_stream: MarkupUpdatesStream<T::Message>,
    },
    WaitForReMount {
        markup: Html<T::Message>,
        markup_updates_stream: MarkupUpdatesStream<T::Message>,
        mount_stream: OwnedStream,
    },
    Close,
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
        live_view,
        make_live_view,
    };

    tracing::trace!("live_view update loop running");

    loop {
        state = next_state(state, live_view_id, &pubsub)
            .await
            .context("failed to compute next state")?;

        match state {
            State::Initial { .. } => {
                tracing::trace!(?live_view_id, "live_view going into `Initial` state")
            }
            State::WaitingForFirstMount { .. } => {
                tracing::trace!(
                    ?live_view_id,
                    "live_view going into `WaitingForFirstMount` state"
                )
            }
            State::Running { .. } => {
                tracing::trace!(?live_view_id, "live_view going into `Running` state")
            }
            State::WaitForReMount { .. } => {
                tracing::trace!(?live_view_id, "live_view going into `WaitForReMount` state")
            }
            State::Close => {
                tracing::trace!(?live_view_id, "live_view task closing");
                break;
            }
        }
    }

    Ok(())
}

async fn next_state<T, M, P>(
    state: State<T, M>,
    live_view_id: LiveViewId,
    pubsub: &P,
) -> anyhow::Result<State<T, M>>
where
    T: LiveView,
    M: MakeLiveView<LiveView = T>,
    P: PubSub + Clone,
{
    match state {
        State::Initial {
            live_view,
            make_live_view,
        } => {
            let mount_stream = subscribe_to_mount(pubsub, live_view_id).await?;

            Ok(State::WaitingForFirstMount {
                live_view,
                mount_stream,
                make_live_view,
            })
        }

        State::WaitingForFirstMount {
            live_view,
            make_live_view,
            mut mount_stream,
        } => {
            if wait_for_mount(live_view_id, &mut mount_stream)
                .await
                .is_err()
            {
                return Ok(State::Close);
            }

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
                markup_updates_stream(live_view, make_live_view, pubsub.clone(), live_view_id)
                    .await;

            let disconnected_stream = pubsub
                .subscribe(&topics::socket_disconnected(live_view_id))
                .await
                .map_err(PubSubError::boxed)
                .context("failed to subscribe to socket disconnected")?
                .map(|_| ());

            Ok(State::Running {
                markup,
                markup_updates_stream: Box::pin(markup_updates_stream),
                disconnected_stream: Box::pin(disconnected_stream),
            })
        }

        State::Running {
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
                        markup,
                        markup_updates_stream,
                        disconnected_stream,
                    })
                }

                MessageForLiveView::WebSocketDisconnected => {
                    tracing::trace!(?live_view_id, "socket disconnected from live_view");

                    let mount_stream = subscribe_to_mount(pubsub, live_view_id).await?;

                    Ok(State::WaitForReMount {
                        markup_updates_stream,
                        mount_stream,
                        markup,
                    })
                }
            }
        }

        State::WaitForReMount {
            markup_updates_stream,
            markup,
            mut mount_stream,
        } => {
            if wait_for_mount(live_view_id, &mut mount_stream)
                .await
                .is_err()
            {
                return Ok(State::Close);
            }

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
                markup,
                markup_updates_stream,
                disconnected_stream: Box::pin(disconnected_stream),
            })
        }

        State::Close => Ok(State::Close),
    }
}

async fn subscribe_to_mount<P>(pubsub: &P, live_view_id: LiveViewId) -> anyhow::Result<OwnedStream>
where
    P: PubSub,
{
    pubsub
        .subscribe(&topics::mounted(live_view_id))
        .await
        .map_err(PubSubError::boxed)
        .context("subscribing to mounted topic")
}

const MOUNT_TIMEOUT: Duration = Duration::from_secs(60);

async fn wait_for_mount(
    live_view_id: LiveViewId,
    mount_stream: &mut OwnedStream,
) -> Result<(), ()> {
    tracing::trace!(?live_view_id, "waiting for live view to be mounted");
    match timeout(MOUNT_TIMEOUT, mount_stream.next()).await {
        Ok(Some(())) => Ok(()),
        Ok(None) | Err(_) => {
            tracing::debug!("live view task mount timeout expired");
            Err(())
        }
    }
}

async fn markup_updates_stream<M, P>(
    mut live_view: M::LiveView,
    make_live_view: M,
    pubsub: P,
    live_view_id: LiveViewId,
) -> MarkupUpdatesStream<<M::LiveView as LiveView>::Message>
where
    M: MakeLiveView,
    P: PubSub + Clone,
{
    let mut first_iteration = true;

    let stream = stream! {
        loop {
            let mut subscriptions = Subscriptions::new(live_view_id);
            live_view.init(&mut subscriptions);
            let mut stream = match subscriptions.into_stream(pubsub.clone()).await {
                Ok(stream) => stream,
                Err(err) => {
                    tracing::error!(%err, "failed to create subscription streams for live view");
                    break;
                }
            };

            if !first_iteration {
                yield (Some(live_view.render()), Vec::new());
            }
            first_iteration = false;

            'inner: while let Some((callback, msg)) = stream.next().await {
                let Updated {
                    live_view: new_live_view,
                    js_commands,
                    skip_render,
                } = callback.call(live_view, msg).await;

                if let Some(new_live_view) = new_live_view {
                    live_view = new_live_view;

                    if skip_render {
                        yield (None, js_commands);
                    } else {
                        let markup = live_view.render();
                        yield (Some(markup), js_commands);
                    }
                } else {
                    tracing::trace!("live view update panicked. Recreating it using `MakeLiveView`");
                    live_view = make_live_view.make_live_view().await;
                    break 'inner;
                }
            }
        }
    };

    Box::pin(stream)
}
