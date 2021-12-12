use bytes::Bytes;
use serde::{de::DeserializeOwned, Serialize};

pub trait Message: Sized {
    fn encode(&self) -> anyhow::Result<Bytes>;

    fn decode(msg: Bytes) -> anyhow::Result<Self>;
}

impl<T> Message for (T,)
where
    T: Message,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        self.0.encode()
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        let t = T::decode(msg)?;
        Ok((t,))
    }
}

impl Message for Bytes {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(self.clone())
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(msg)
    }
}

impl Message for String {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::copy_from_slice(self.as_bytes()))
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(std::str::from_utf8(&*msg)?.to_owned())
    }
}

impl Message for () {
    fn encode(&self) -> anyhow::Result<Bytes> {
        Ok(Bytes::new())
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        anyhow::ensure!(msg.is_empty());
        Ok(())
    }
}

impl<T> Message for axum::Json<T>
where
    T: Serialize + DeserializeOwned,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        let bytes = serde_json::to_vec(&self.0)?;
        let bytes = Bytes::copy_from_slice(&bytes);
        Ok(bytes)
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(Self(serde_json::from_slice(&msg)?))
    }
}

pub struct Bincode<T>(pub T);

impl<T> Message for Bincode<T>
where
    T: Serialize + DeserializeOwned,
{
    fn encode(&self) -> anyhow::Result<Bytes> {
        let bytes = bincode::serialize(&self.0)?;
        let bytes = Bytes::copy_from_slice(&bytes);
        Ok(bytes)
    }

    fn decode(msg: Bytes) -> anyhow::Result<Self> {
        Ok(Self(bincode::deserialize(&msg)?))
    }
}