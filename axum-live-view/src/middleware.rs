use crate::{live_view::EmbedLiveView, pubsub::PubSub};
use axum::http::{Request, Response};
use std::task::{Context, Poll};
use tower_service::Service;

#[derive(Debug, Clone)]
pub struct LiveViewLayer<P> {
    pubsub: P,
}

impl<P> LiveViewLayer<P> {
    pub(crate) fn new(pubsub: P) -> LiveViewLayer<P>
    where
        P: PubSub,
    {
        LiveViewLayer { pubsub }
    }
}

impl<S, P> tower_layer::Layer<S> for LiveViewLayer<P>
where
    P: PubSub + Clone,
{
    type Service = LiveViewMiddleware<S, P>;

    fn layer(&self, inner: S) -> Self::Service {
        LiveViewMiddleware {
            inner,
            pubsub: self.pubsub.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct LiveViewMiddleware<S, P> {
    inner: S,
    pubsub: P,
}

impl<S, P, ReqBody, ResBody> Service<Request<ReqBody>> for LiveViewMiddleware<S, P>
where
    P: PubSub + Clone,
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        req.extensions_mut()
            .insert(EmbedLiveView::new(self.pubsub.clone()));

        self.inner.call(req)
    }
}
