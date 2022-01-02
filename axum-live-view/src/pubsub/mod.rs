use axum::{async_trait, BoxError};
use bytes::Bytes;
use futures_util::{
    future::BoxFuture,
    stream::{BoxStream, StreamExt},
};
use std::{fmt, future::ready};
use tokio::sync::broadcast;
use tokio_stream::wrappers::BroadcastStream;

mod encode_decode;
mod in_process;

pub use self::{
    encode_decode::{Bincode, Decode, Encode},
    in_process::InProcess,
};

pub trait Topic: Send + Sync + 'static {
    type Message: Encode + Decode + Send;

    fn topic(&self) -> &str;
}

#[async_trait]
pub trait PubSubBackend: Send + Sync + 'static {
    type Error: fmt::Debug + Into<BoxError> + Send + Sync + 'static;

    async fn broadcast_raw(&self, topic: &str, msg: Bytes) -> Result<(), Self::Error>;

    async fn subscribe_raw(&self, topic: &str) -> Result<BoxStream<'static, Bytes>, Self::Error>;
}

pub trait PubSub: PubSubBackend {
    fn broadcast<'a, T>(
        &'a self,
        topic: &T,
        msg: T::Message,
    ) -> BoxFuture<'a, Result<(), PubSubError<T, Self::Error>>>
    where
        T: Topic,
        Self: Sized,
    {
        let topic = topic.topic().to_owned();
        Box::pin(async move {
            let bytes = msg.encode().map_err(PubSubError::Encode)?;
            self.broadcast_raw(&topic, bytes)
                .await
                .map_err(PubSubError::Broadcast)?;
            Ok(())
        })
    }

    fn subscribe<'a, T>(
        &'a self,
        topic: &T,
    ) -> BoxFuture<'a, Result<BoxStream<'static, T::Message>, PubSubError<T, Self::Error>>>
    where
        T: Topic,
        Self: Sized,
    {
        let topic = topic.topic().to_owned();

        Box::pin(async move {
            let mut stream = self
                .subscribe_raw(&topic)
                .await
                .map_err(PubSubError::Subscribe)?;

            let decoded_stream = async_stream::stream! {
                while let Some(bytes) = stream.next().await {
                    match T::Message::decode(bytes) {
                        Ok(msg) => yield msg,
                        Err(err) => {
                            tracing::warn!(
                                ?topic,
                                ?err,
                                expected_type = tracing::field::display(std::any::type_name::<T>()),
                                "failed to decode message for topic stream",
                            );
                        }
                    }
                }
            };

            Ok(Box::pin(decoded_stream) as BoxStream<'static, T::Message>)
        })
    }
}

impl<T> PubSub for T where T: PubSubBackend {}

#[derive(Debug, thiserror::Error)]
#[error(transparent)]
pub struct BoxErrorWrapper {
    err: BoxError,
}

impl BoxErrorWrapper {
    pub(crate) fn new<E>(err: E) -> Self
    where
        E: Into<BoxError>,
    {
        Self { err: err.into() }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct Logging<P> {
    inner: P,
}

impl<P> Logging<P> {
    pub(crate) fn new(inner: P) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<P> PubSubBackend for Logging<P>
where
    P: PubSubBackend,
{
    type Error = P::Error;

    async fn broadcast_raw(&self, topic: &str, msg: Bytes) -> Result<(), Self::Error> {
        {
            let msg = String::from_utf8_lossy(&msg);
            tracing::trace!(?topic, %msg, "broadcast_raw");
        }
        self.inner.broadcast_raw(topic, msg).await
    }

    async fn subscribe_raw(&self, topic: &str) -> Result<BoxStream<'static, Bytes>, Self::Error> {
        tracing::trace!(?topic, "subscribing");
        self.inner.subscribe_raw(topic).await
    }
}

pub enum PubSubError<T, E>
where
    T: Topic,
{
    Encode(<T::Message as Encode>::Error),
    Decode(<T::Message as Decode>::Error),
    Broadcast(E),
    Subscribe(E),
}

impl<T, E> PubSubError<T, E>
where
    T: Topic,
    <T::Message as Encode>::Error: Into<BoxError>,
    <T::Message as Decode>::Error: Into<BoxError>,
    E: Into<BoxError>,
{
    pub(crate) fn boxed(self) -> BoxErrorWrapper {
        let err = match self {
            Self::Encode(inner) => inner.into(),
            Self::Decode(inner) => inner.into(),
            Self::Broadcast(inner) => inner.into(),
            Self::Subscribe(inner) => inner.into(),
        };
        BoxErrorWrapper { err }
    }
}

impl<T, E> fmt::Debug for PubSubError<T, E>
where
    T: Topic,
    E: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(inner) => f.debug_tuple("Encode").field(inner).finish(),
            Self::Decode(inner) => f.debug_tuple("Decode").field(inner).finish(),
            Self::Broadcast(inner) => f.debug_tuple("Broadcast").field(inner).finish(),
            Self::Subscribe(inner) => f.debug_tuple("Subscribe").field(inner).finish(),
        }
    }
}

impl<T, E> fmt::Display for PubSubError<T, E>
where
    T: Topic,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PubSubError::Encode(_) => write!(f, "Encoding a message failed"),
            PubSubError::Decode(_) => write!(f, "Decoding a message failed"),
            PubSubError::Broadcast(_) => write!(f, "PubSub backend failed to broadcast"),
            PubSubError::Subscribe(_) => {
                write!(f, "PubSub backend failed to create new subscription")
            }
        }
    }
}

impl<T, E> std::error::Error for PubSubError<T, E>
where
    T: Topic,
    <T::Message as Encode>::Error: std::error::Error,
    <T::Message as Decode>::Error: std::error::Error,
    E: std::error::Error + 'static,
{
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PubSubError::Encode(inner) => Some(inner),
            PubSubError::Decode(inner) => Some(inner),
            PubSubError::Broadcast(inner) => Some(inner),
            PubSubError::Subscribe(inner) => Some(inner),
        }
    }
}
