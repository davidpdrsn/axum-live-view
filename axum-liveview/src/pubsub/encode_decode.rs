use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

pub trait Encode {
    fn encode(&self) -> anyhow::Result<Bytes>;
}

pub trait Decode: Sized {
    fn decode(msg: Bytes) -> anyhow::Result<Self>;
}

impl<T> Encode for (T,)
where
    T: Encode,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        self.0.encode()
    }
}

impl<T> Decode for (T,)
where
    T: Decode,
{
    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        let t = T::decode(msg)?;
        Ok((t,))
    }
}

impl Encode for Bytes {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(self.clone())
    }
}

impl Decode for Bytes {
    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(msg)
    }
}

impl Encode for String {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::copy_from_slice(self.as_bytes()))
    }
}

impl Decode for String {
    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(std::str::from_utf8(&*msg)?.to_owned())
    }
}

impl Encode for () {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::new())
    }
}

impl Decode for () {
    fn decode(_msg: Bytes) -> anyhow::Result<Self> {
        Ok(())
    }
}

impl<T> Encode for axum::Json<T>
where
    T: Serialize,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        let bytes = serde_json::to_vec(&self.0)?;
        let bytes = Bytes::copy_from_slice(&bytes);
        Ok(bytes)
    }
}

impl<T> Decode for axum::Json<T>
where
    T: DeserializeOwned,
{
    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(Self(serde_json::from_slice(&msg)?))
    }
}

pub struct Bincode<T>(pub T);

impl<T> Encode for Bincode<T>
where
    T: Serialize,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        let bytes = bincode::serialize(&self.0)?;
        let bytes = Bytes::copy_from_slice(&bytes);
        Ok(bytes)
    }
}

impl<T> Decode for Bincode<T>
where
    T: DeserializeOwned,
{
    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(Self(bincode::deserialize(&msg)?))
    }
}

impl<T> Encode for Option<T>
where
    T: Encode,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
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
    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(T::decode(msg).ok())
    }
}

impl<T> Encode for anyhow::Result<T>
where
    T: Encode,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        if let Ok(msg) = self {
            Ok(msg.encode()?)
        } else {
            Ok(Bytes::new())
        }
    }
}

impl<T> Decode for anyhow::Result<T>
where
    T: Decode,
{
    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(T::decode(msg))
    }
}
