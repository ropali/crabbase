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

#[cfg(test)]
mod tests {
    use super::*;

    // ── quote_ident ───────────────────────────────────────────────────────────

    #[test]
    fn test_quote_ident_simple() {
        assert_eq!(quote_ident("users"), "\"users\"");
    }

    #[test]
    fn test_quote_ident_with_embedded_quotes() {
        // A double-quote inside an identifier must be escaped as ""
        assert_eq!(quote_ident("col\"name"), "\"col\"\"name\"");
    }

    #[test]
    fn test_quote_ident_empty_string() {
        assert_eq!(quote_ident(""), "\"\"");
    }

    #[test]
    fn test_quote_ident_underscore_and_numbers() {
        assert_eq!(quote_ident("_my_col_1"), "\"_my_col_1\"");
    }

    // ── random_str ────────────────────────────────────────────────────────────

    #[test]
    fn test_random_str_default_length() {
        let s = random_str(None);
        assert_eq!(s.len(), 32);
    }

    #[test]
    fn test_random_str_custom_length() {
        let s = random_str(Some(16));
        assert_eq!(s.len(), 16);
    }

    #[test]
    fn test_random_str_zero_length() {
        let s = random_str(Some(0));
        assert_eq!(s.len(), 0);
    }

    #[test]
    fn test_random_str_is_alphanumeric() {
        let s = random_str(Some(64));
        assert!(
            s.chars().all(|c| c.is_ascii_alphanumeric()),
            "random_str output should only contain ASCII alphanumeric characters"
        );
    }

    #[test]
    fn test_random_str_differs_between_calls() {
        // Two independent calls should virtually never produce identical strings
        let a = random_str(None);
        let b = random_str(None);
        assert_ne!(a, b, "two random strings should differ (probabilistic)");
    }

    // ── to_snake_case ─────────────────────────────────────────────────────────

    #[test]
    fn test_to_snake_case_already_snake() {
        assert_eq!(to_snake_case("hello_world"), "hello_world");
    }

    #[test]
    fn test_to_snake_case_single_word_lowercase() {
        assert_eq!(to_snake_case("hello"), "hello");
    }

    #[test]
    fn test_to_snake_case_camel_case() {
        assert_eq!(to_snake_case("camelCase"), "camel_case");
    }

    #[test]
    fn test_to_snake_case_pascal_case() {
        assert_eq!(to_snake_case("PascalCase"), "pascal_case");
    }

    #[test]
    fn test_to_snake_case_multiple_uppercase() {
        assert_eq!(to_snake_case("MyVariableName"), "my_variable_name");
    }

    #[test]
    fn test_to_snake_case_single_uppercase_char() {
        assert_eq!(to_snake_case("A"), "a");
    }

    #[test]
    fn test_to_snake_case_empty_string() {
        assert_eq!(to_snake_case(""), "");
    }
}
