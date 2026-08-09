use rand::distributions::{Alphanumeric, DistString};

pub fn quote_ident(identifier: &str) -> String {
    format!("\"{}\"", identifier.replace('"', "\"\""))
}

pub fn random_str(length: Option<usize>) -> String {
    let len = length.unwrap_or(32);

    Alphanumeric.sample_string(&mut rand::thread_rng(), len)
}

/// Helper macro utility to get the string literal representation of an enum variant.
#[macro_export]
macro_rules! stringify {
    ($x:ident) => {
        stringify!($x)
    };
}

/// A generalized macro that generates a full Enum definition and
/// its associated `to_string()` implementation for any given type name.
///
/// Usage: define_enum!(<EnumName>, <Variant1>, <Variant2>, ...);
#[macro_export]
macro_rules! define_enum {
    ( $enum_name:ident, $( $variant:ident ),* ) => {

        // Generate the Enum Definition using the provided name ($enum_name)
        #[derive(Debug, Clone, Copy)]
        pub enum $enum_name {
            $( $variant ),* // The variants use the list of inputs
        }

        impl $enum_name {
            /// Maps the strongly-typed variant to a string value.
            pub fn to_string(&self) -> &'static str {
                match self {
                    $(
                        Self::$variant => stringify!($variant), // Match uses the list of inputs
                    )*
                }
            }
        }
    };
}
