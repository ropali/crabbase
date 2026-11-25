//! Redaction of sensitive values before they are persisted to `_logs.data`.

use serde_json::Value;

/// Keys whose values are never written to the audit log in the clear.
/// Matching is substring-based so variants like `new_password` or
/// `access_token` are covered too.
const SENSITIVE_KEYS: &[&str] = &[
    "password",
    "password_hash",
    "token",
    "secret",
    "otp",
    "authorization",
    "api_key",
];

/// Recursively replaces the values of sensitive keys in a JSON payload
/// with a `[REDACTED]` marker.
pub fn redact(mut value: Value) -> Value {
    redact_in_place(&mut value);
    value
}

fn redact_in_place(value: &mut Value) {
    match value {
        Value::Object(map) => {
            for (key, inner) in map.iter_mut() {
                if is_sensitive(key) {
                    *inner = Value::String("[REDACTED]".to_string());
                } else {
                    redact_in_place(inner);
                }
            }
        }
        Value::Array(items) => {
            for item in items.iter_mut() {
                redact_in_place(item);
            }
        }
        _ => {}
    }
}

fn is_sensitive(key: &str) -> bool {
    let key = key.to_ascii_lowercase();
    SENSITIVE_KEYS.iter().any(|s| key.contains(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn redacts_nested_sensitive_keys() {
        let input = json!({
            "user": "admin",
            "credentials": {
                "password": "hunter2",
                "api_key": "abcd1234"
            },
            "tokens": [{ "token": "xyz" }],
            "note": "password reset requested"
        });

        let out = redact(input);
        assert_eq!(out["user"], "admin");
        assert_eq!(out["credentials"]["password"], "[REDACTED]");
        assert_eq!(out["credentials"]["api_key"], "[REDACTED]");
        // "tokens" itself contains "token", so the whole array is redacted —
        // over-redaction is the safe default for an audit trail.
        assert_eq!(out["tokens"], "[REDACTED]");
        // Plain-text values containing the word are NOT treated as keys.
        assert_eq!(out["note"], "password reset requested");
    }
}
