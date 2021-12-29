macro_rules! axm {
    (
        $(#[$meta:meta])*
        pub(crate) enum $name:ident {
            $(
                #[attr = $attr:literal]
                $(#[$variant_meta:meta])*
                $variant:ident,
            )*
        }
    ) => {
        $(#[$meta])*
        pub(crate) enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )*
        }

        impl $name {
            pub(crate) fn from_str(s: &str) -> anyhow::Result<Self> {
                match s {
                    $(
                        concat!("axm-", $attr) => Ok(Self::$variant),
                    )*
                    other => anyhow::bail!("unknown message topic: {:?}", other),
                }
            }
        }
    };
}
