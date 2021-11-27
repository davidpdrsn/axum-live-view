use axum::{
    body::{Bytes, HttpBody},
    http::{self, Request, Response},
    Json,
};
use axum_liveview::{pubsub::PubSub, PubSubExt};
use futures::ready;
use parking_lot::Mutex;
use pin_project_lite::pin_project;
use serde::{Deserialize, Serialize};
use std::future::Future;
use std::{
    pin::Pin,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::Duration,
};
use tokio::time::interval;
use tower::{Layer, Service};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HttpStats {
    pub requests_total: u64,
    pub bytes_received: u64,
    pub bytes_sent: u64,
}

#[derive(Debug, Clone)]
pub struct HttpStatsLayer {
    stats: Arc<Mutex<HttpStats>>,
}

impl HttpStatsLayer {
    pub fn new<P>(pubsub: P) -> Self
    where
        P: PubSub,
    {
        let stats = Arc::new(Mutex::new(HttpStats::default()));

        {
            let stats = stats.clone();
            tokio::spawn(async move {
                let mut interval = interval(Duration::from_secs(1));
                loop {
                    interval.tick().await;
                    let stats = stats.lock().clone();
                    let _ = pubsub.broadcast("http-stats", Json(stats)).await;
                }
            });
        }

        HttpStatsLayer { stats }
    }
}

impl<S> Layer<S> for HttpStatsLayer {
    type Service = WithHttpStats<S>;

    fn layer(&self, inner: S) -> Self::Service {
        WithHttpStats {
            inner,
            stats: self.stats.clone(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct WithHttpStats<S> {
    inner: S,
    stats: Arc<Mutex<HttpStats>>,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for WithHttpStats<S>
where
    S: Service<Request<StatsBody<ReqBody>>, Response = Response<ResBody>>,
{
    type Response = Response<StatsBody<ResBody>>;
    type Error = S::Error;
    type Future = ResponseFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        req.extensions_mut().insert(self.stats.clone());

        ResponseFuture {
            inner: self.inner.call(req.map(|inner| StatsBody {
                inner,
                stats: self.stats.clone(),
                direction: Direction::Receive,
            })),
            stats: self.stats.clone(),
        }
    }
}

pin_project! {
    pub struct ResponseFuture<F> {
        #[pin]
        inner: F,
        stats: Arc<Mutex<HttpStats>>,
    }
}

impl<F, B, E> Future for ResponseFuture<F>
where
    F: Future<Output = Result<Response<B>, E>>,
{
    type Output = Result<Response<StatsBody<B>>, E>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.project();
        let result = ready!(this.inner.poll(cx));
        this.stats.lock().requests_total += 1;
        let result = result.map(|res| {
            res.map(|inner| StatsBody {
                inner,
                stats: this.stats.clone(),
                direction: Direction::Send,
            })
        });
        Poll::Ready(result)
    }
}

pin_project! {
    pub struct StatsBody<B> {
        #[pin]
        inner: B,
        stats: Arc<Mutex<HttpStats>>,
        direction: Direction
    }
}

#[derive(Clone, Copy)]
enum Direction {
    Receive,
    Send,
}

impl<B> HttpBody for StatsBody<B>
where
    B: HttpBody<Data = Bytes>,
{
    type Data = B::Data;
    type Error = B::Error;

    fn poll_data(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Data, Self::Error>>> {
        let this = self.project();
        match ready!(this.inner.poll_data(cx)) {
            Some(Ok(buf)) => {
                match this.direction {
                    Direction::Receive => {
                        this.stats.lock().bytes_received += buf.len() as u64;
                    }
                    Direction::Send => {
                        this.stats.lock().bytes_sent += buf.len() as u64;
                    }
                }
                Some(Ok(buf)).into()
            }
            other => other.into(),
        }
    }

    fn poll_trailers(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Result<Option<http::HeaderMap>, Self::Error>> {
        self.project().inner.poll_trailers(cx)
    }

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn size_hint(&self) -> http_body::SizeHint {
        self.inner.size_hint()
    }
}
