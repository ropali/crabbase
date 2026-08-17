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

pub fn to_snake_case(s: &str) -> String {
    let mut snake = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 {
                snake.push('_');
            }
            for lc in ch.to_lowercase() {
                snake.push(lc);
            }
        } else {
            snake.push(ch);
        }
    }
    snake
}

/// A generalized macro that generates a full Enum definition and
/// its associated `to_string()` implementation for any given type name.
/// Variant names are automatically converted to snake_case.
///
/// Usage: define_enum!(<EnumName>, <Variant1>, <Variant2>, ...);
#[macro_export]
macro_rules! define_enum {
    ( $enum_name:ident, $( $variant:ident ),* ) => {

        // Generate the Enum Definition using the provided name ($enum_name)
        #[derive(Debug, Clone, Copy)]
        pub enum $enum_name {
            $( $variant ),*
        }

        impl $enum_name {
            /// Maps the strongly-typed variant to a snake_case String.
            pub fn to_string(&self) -> String {
                match self {
                    $(
                        Self::$variant => $crate::utils::string_utils::to_snake_case(stringify!($variant)),
                    )*
                }
            }
        }
    };
}
