macro_rules! builder {
    (
        #[builder_name = $builder_name:ident]
        $(#[$m:meta])*
        pub struct $name:ident {
            $(
                $field:ident : $field_ty:ty,
            )*
        }
    ) => {
        $(#[$m])*
        pub struct $name {
            $(
                $field: $field_ty,
            )*
        }

        impl $name {
            #[doc = concat!("Get a builder for `", stringify!($name), "`.")]
            #[doc = ""]
            #[doc = concat!("This allows creating `", stringify!($name), "` events for example for use in tests.")]
            pub fn builder() -> $builder_name {
                $builder_name::default()
            }
        }

        #[derive(Default, Debug, Clone)]
        #[doc = concat!("Builder for `", stringify!($name), "`")]
        #[doc = ""]
        #[doc = concat!("Created with `", stringify!($name), "::build`")]
        pub struct $builder_name {
            $(
                $field: Option<$field_ty>,
            )*
        }

        impl $builder_name {
            $(
                #[doc = concat!("Set `", stringify!($field), "`.")]
                pub fn $field(mut self, $field: impl Into<$field_ty>) -> Self {
                    self.$field = Some($field.into());
                    self
                }
            )*

            #[doc = concat!("Consume the build and construct a `", stringify!($name), "`")]
            pub fn build(self) -> $name {
                $name {
                    $(
                        $field: self.$field.unwrap_or_default(),
                    )*
                }
            }
        }
    };
}

macro_rules! impl_from {
    ($ty:ident :: $variant:ident) => {
        impl From<$variant> for $ty {
            fn from(x: $variant) -> Self {
                Self::$variant(x)
            }
        }
    };
}
