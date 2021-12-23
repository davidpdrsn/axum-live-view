use crate::pubsub::{Decode, Encode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

pub mod axm {
    macro_rules! binding {
        ($fn_name:ident, $const_name:ident, $attr:literal) => {
            pub const fn $fn_name() -> &'static str {
                $const_name
            }

            pub(crate) const $const_name: &str = concat!("axm-", $attr);
        };
    }

    binding!(blur, BLUR, "blur");
    binding!(change, CHANGE, "change");
    binding!(click, CLICK, "click");
    binding!(focus, FOCUS, "focus");
    binding!(input, INPUT, "input");
    binding!(keydown, KEYDOWN, "keydown");
    binding!(keyup, KEYUP, "keyup");
    binding!(submit, SUBMIT, "submit");
    binding!(throttle, THROTTLE, "throttle");
    binding!(debounce, DEBOUNCE, "debounce");
    binding!(key, KEY, "key");
    binding!(window_keydown, WINDOW_KEYDOWN, "window-keydown");
    binding!(window_keyup, WINDOW_KEYUP, "window-keyup");

    pub fn data(name: impl AsRef<str>) -> String {
        format!("axm-data-{}", name.as_ref())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FormEvent<V = String, D = ()> {
    pub(crate) value: V,
    pub(crate) data: D,
}

impl<V, D> FormEvent<V, D> {
    pub fn value(&self) -> &V {
        &self.value
    }

    pub fn into_value(self) -> V {
        self.value
    }

    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn into_data(self) -> D {
        self.data
    }

    pub fn into_parts(self) -> (V, D) {
        (self.value, self.data)
    }
}

impl<V, D> Encode for FormEvent<V, D>
where
    V: Serialize,
    D: Serialize,
{
    fn encode(&self) -> anyhow::Result<bytes::Bytes> {
        axum::Json(self).encode()
    }
}

impl<V, D> Decode for FormEvent<V, D>
where
    V: DeserializeOwned,
    D: DeserializeOwned,
{
    fn decode(msg: bytes::Bytes) -> anyhow::Result<Self> {
        Ok(axum::Json::<Self>::decode(msg)?.0)
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct KeyEvent<D = ()> {
    pub(crate) key: String,
    pub(crate) code: String,
    pub(crate) alt: bool,
    pub(crate) ctrl: bool,
    pub(crate) shift: bool,
    pub(crate) meta: bool,
    pub(crate) data: D,
}

impl<D> KeyEvent<D> {
    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn code(&self) -> &str {
        &self.key
    }

    pub fn alt(&self) -> bool {
        self.alt
    }

    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    pub fn shift(&self) -> bool {
        self.shift
    }

    pub fn meta(&self) -> bool {
        self.meta
    }

    pub fn data(&self) -> &D {
        &self.data
    }

    pub fn into_data(self) -> D {
        self.data
    }
}

impl<D> Encode for KeyEvent<D>
where
    D: Serialize,
{
    fn encode(&self) -> anyhow::Result<bytes::Bytes> {
        axum::Json(self).encode()
    }
}

impl<D> Decode for KeyEvent<D>
where
    D: DeserializeOwned,
{
    fn decode(msg: bytes::Bytes) -> anyhow::Result<Self> {
        Ok(axum::Json::<Self>::decode(msg)?.0)
    }
}
