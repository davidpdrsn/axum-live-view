use crate::{
    pubsub::{self, PubSub},
    LiveViewManager,
};
use axum::http::{Request, Response};
use std::task::{Context, Poll};
use tower_service::Service;

pub fn layer<P>(pubsub: P) -> Layer<P>
where
    P: PubSub,
{
    Layer { pubsub }
}

pub struct Layer<P> {
    pubsub: P,
}

impl<S, P> tower_layer::Layer<S> for Layer<P>
where
    P: PubSub + Clone,
{
    type Service = Middleware<S, P>;

    fn layer(&self, inner: S) -> Self::Service {
        Middleware {
            inner,
            pubsub: self.pubsub.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Middleware<S, P> {
    inner: S,
    pubsub: P,
}

impl<S, P, ReqBody, ResBody> Service<Request<ReqBody>> for Middleware<S, P>
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
            .insert(LiveViewManager::new(pubsub::Logging::new(
                self.pubsub.clone(),
            )));

        self.inner.call(req)
    }
}
