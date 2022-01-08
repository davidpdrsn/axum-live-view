macro_rules! builder {
    (
        #[builder_name = $builder_name:ident]
        #[$($m:meta)*]
        pub struct $name:ident {
            $(
                $field:ident : $field_ty:ty,
            )*
        }
    ) => {
        #[$($m)*]
        pub struct $name {
            $(
                $field: $field_ty,
            )*
        }

        impl $name {
            pub fn builder() -> $builder_name {
                $builder_name::default()
            }
        }

        #[derive(Default, Debug, Clone)]
        pub struct $builder_name {
            $(
                $field: Option<$field_ty>,
            )*
        }

        impl $builder_name {
            pub fn new() -> Self {
                Self::default()
            }

            $(
                pub fn $field(mut self, $field: impl Into<$field_ty>) -> Self {
                    self.$field = Some($field.into());
                    self
                }
            )*

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
