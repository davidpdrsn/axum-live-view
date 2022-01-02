use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};
use std::{convert::Infallible, fmt};

pub trait Encode {
    type Error: fmt::Debug + Send + Sync + 'static;

    fn encode(&self) -> Result<Bytes, Self::Error>;
}

pub trait Decode: Sized {
    type Error: fmt::Debug + Send + Sync + 'static;

    fn decode(msg: Bytes) -> Result<Self, Self::Error>;
}

impl<T> Encode for (T,)
where
    T: Encode,
{
    type Error = T::Error;

    fn encode(&self) -> Result<Bytes, Self::Error> {
        self.0.encode()
    }
}

impl<T> Decode for (T,)
where
    T: Decode,
{
    type Error = T::Error;

    fn decode(msg: Bytes) -> Result<Self, Self::Error> {
        let t = T::decode(msg)?;
        Ok((t,))
    }
}

impl Encode for Bytes {
    type Error = Infallible;

    fn encode(&self) -> Result<Bytes, Self::Error> {
        Ok(self.clone())
    }
}

impl Decode for Bytes {
    type Error = Infallible;

    fn decode(msg: Bytes) -> Result<Self, Self::Error> {
        Ok(msg)
    }
}

impl Encode for String {
    type Error = Infallible;

    fn encode(&self) -> Result<Bytes, Self::Error> {
        Ok(Bytes::copy_from_slice(self.as_bytes()))
    }
}

impl Decode for String {
    type Error = std::str::Utf8Error;

    fn decode(msg: Bytes) -> Result<Self, Self::Error> {
        Ok(std::str::from_utf8(&*msg)?.to_owned())
    }
}

impl Encode for () {
    type Error = Infallible;

    fn encode(&self) -> Result<Bytes, Self::Error> {
        Ok(Bytes::new())
    }
}

impl Decode for () {
    type Error = Infallible;

    fn decode(_msg: Bytes) -> Result<Self, Self::Error> {
        Ok(())
    }
}

impl<T> Encode for axum::Json<T>
where
    T: Serialize,
{
    type Error = serde_json::Error;

    fn encode(&self) -> Result<Bytes, Self::Error> {
        let bytes = serde_json::to_vec(&self.0)?;
        let bytes = Bytes::copy_from_slice(&bytes);
        Ok(bytes)
    }
}

impl<T> Decode for axum::Json<T>
where
    T: DeserializeOwned,
{
    type Error = serde_json::Error;

    fn decode(msg: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(serde_json::from_slice(&msg)?))
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct Bincode<T>(pub T);

impl<T> Encode for Bincode<T>
where
    T: Serialize,
{
    type Error = bincode::Error;

    fn encode(&self) -> Result<Bytes, Self::Error> {
        let bytes = bincode::serialize(&self.0)?;
        let bytes = Bytes::copy_from_slice(&bytes);
        Ok(bytes)
    }
}

impl<T> Decode for Bincode<T>
where
    T: DeserializeOwned,
{
    type Error = bincode::Error;

    fn decode(msg: Bytes) -> Result<Self, Self::Error> {
        Ok(Self(bincode::deserialize(&msg)?))
    }
}

impl<T> Encode for Option<T>
where
    T: Encode,
{
    type Error = T::Error;

    fn encode(&self) -> Result<Bytes, Self::Error> {
        if let Some(msg) = self {
            Ok(msg.encode()?)
        } else {
            Ok(Bytes::new())
        }
    }
}

impl<T> Decode for Option<T>
where
    T: Decode,
{
    type Error = Infallible;

    fn decode(msg: Bytes) -> Result<Self, Self::Error> {
        Ok(T::decode(msg).ok())
    }
}

impl<T> Decode for Result<T, T::Error>
where
    T: Decode,
{
    type Error = Infallible;

    fn decode(msg: Bytes) -> Result<Self, Self::Error> {
        Ok(T::decode(msg))
    }
}
