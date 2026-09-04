use std::fmt::write;

use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;
use tracing::error;

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: &'static str,
    pub message: String,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

#[derive(Debug)]
pub enum APIError {
    Validation { message: String, details: Value },
    NotFound { resource: String },
    Unauthorized,
    Forbidden,
    Conflict { message: String },
    Internal { message: String, details: Value },
}

impl APIError {
    fn to_status_and_body(&self) -> (StatusCode, ErrorBody) {
        match self {
            APIError::Validation { message, details } => (
                StatusCode::BAD_REQUEST,
                ErrorBody {
                    code: "VALIDATION_ERROR",
                    message: message.clone(),
                    details: Some(details.clone()),
                },
            ),
            APIError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                ErrorBody {
                    code: "UNAUTHORIZED",
                    message: "Authentication required".to_string(),
                    details: None,
                },
            ),
            APIError::Forbidden => (
                StatusCode::FORBIDDEN,
                ErrorBody {
                    code: "FORBIDDEN",
                    message: "You are not allowed to perform this action".to_string(),
                    details: None,
                },
            ),
            APIError::NotFound { resource } => (
                StatusCode::NOT_FOUND,
                ErrorBody {
                    code: "NOT_FOUND",
                    message: format!("{resource} not found"),
                    details: None,
                },
            ),
            APIError::Conflict { message } => (
                StatusCode::CONFLICT,
                ErrorBody {
                    code: "CONFLICT",
                    message: message.clone(),
                    details: None,
                },
            ),
            APIError::Internal { message, details } => (
                StatusCode::INTERNAL_SERVER_ERROR,
                ErrorBody {
                    code: "INTERNAL_ERROR",
                    message: message.to_string(),
                    details: Some(details.to_owned()),
                },
            ),
        }
    }
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let (status, body) = self.to_status_and_body();
        if status.is_server_error() {
            error!(status = %status, error = ?self, "request failed with server error");
        }
        (status, Json(body)).into_response()
    }
}

#[derive(Debug)]
pub enum RepositoryError {
    NotFound(String),
    DuplicateKey(String),
    ConnectionFailed(String),
    QueryFailed {
        message: String,
        source: Option<String>,
    },
    Validation {
        message: String,
        field: Option<String>,
    },
    OtherError(String),
    Forbidden(String),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => RepositoryError::NotFound("resource".to_string()),

            sqlx::Error::Database(db_err) => {
                let message = db_err.message().to_string();

                let lower = message.to_ascii_lowercase();

                if lower.contains("unique constraint") {
                    return RepositoryError::DuplicateKey(
                        "Duplicate value violates unique constraint.".to_string(),
                    );
                }

                if lower.contains("foreign key constraint failed") {
                    return RepositoryError::Validation {
                        message: "Invalid relation reference".to_string(),
                        field: None,
                    };
                }

                RepositoryError::QueryFailed {
                    message: "Database query failed".to_string(),
                    source: Some(message),
                }
            }
            sqlx::Error::Io(io) => RepositoryError::ConnectionFailed(io.to_string()),
            other => RepositoryError::QueryFailed {
                message: "datbase query failed".to_string(),
                source: Some(other.to_string()),
            },
        }
    }
}

// Conversion from sqlx::Error to RepositoryError allows using the `?` operator
impl From<RepositoryError> for APIError {
    fn from(value: RepositoryError) -> Self {
        match value {
            RepositoryError::NotFound(resource) => APIError::NotFound { resource },
            RepositoryError::DuplicateKey(msg) => APIError::Conflict { message: msg },
            RepositoryError::Validation { message, field } => APIError::Validation {
                message,
                details: serde_json::json!({"field": field}),
            },
            RepositoryError::QueryFailed { message, source } => APIError::Internal {
                message,
                details: serde_json::json!({"source": source}),
            },
            RepositoryError::OtherError(msg) => APIError::Internal {
                message: "Something went wrong".to_string(),
                details: serde_json::Value::String(msg),
            },
            RepositoryError::ConnectionFailed(message) => APIError::Internal {
                message: "Database connection failure".to_string(),
                details: serde_json::json!({"source": message}),
            },
            RepositoryError::Forbidden(mesaage) => APIError::Forbidden,
        }
    }
}

impl std::fmt::Display for RepositoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            RepositoryError::NotFound(msg) => write!(f, "Not Found: {}", msg),
            RepositoryError::DuplicateKey(msg) => write!(f, "Duplicate Key: {}", msg),

            RepositoryError::ConnectionFailed(msg) => write!(f, "Connection Failed: {}", msg),
            RepositoryError::QueryFailed { message, source } => {
                if let Some(src) = source {
                    write!(f, "Query Failed: {} (Source: {})", message, src)
                } else {
                    write!(f, "Query Failed: {}", message)
                }
            }
            RepositoryError::Validation { message, field } => {
                if let Some(fld) = field {
                    write!(f, "Validation Error on field '{}': {}", fld, message)
                } else {
                    write!(f, "Validation Error:{}", message)
                }
            }
            RepositoryError::OtherError(msg) => write!(f, "Error: {}", msg),
            RepositoryError::Forbidden(msg) => write!(f, "Error: {}", msg),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    // ── RepositoryError → APIError conversions ────────────────────────────────

    #[test]
    fn test_repository_not_found_converts_to_api_not_found() {
        let repo_err = RepositoryError::NotFound("widgets".to_string());
        let api_err: APIError = repo_err.into();
        let (status, body) = api_err.to_status_and_body();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "NOT_FOUND");
        assert!(body.message.contains("widgets"));
    }

    #[test]
    fn test_repository_duplicate_key_converts_to_api_conflict() {
        let repo_err = RepositoryError::DuplicateKey("email already exists".to_string());
        let api_err: APIError = repo_err.into();
        let (status, body) = api_err.to_status_and_body();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "CONFLICT");
    }

    #[test]
    fn test_repository_validation_converts_to_api_bad_request() {
        let repo_err = RepositoryError::Validation {
            message: "name is required".to_string(),
            field: Some("name".to_string()),
        };
        let api_err: APIError = repo_err.into();
        let (status, body) = api_err.to_status_and_body();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "VALIDATION_ERROR");
        assert!(body.message.contains("name is required"));
        assert!(body.details.is_some());
    }

    #[test]
    fn test_repository_query_failed_converts_to_api_internal() {
        let repo_err = RepositoryError::QueryFailed {
            message: "query failed".to_string(),
            source: Some("syntax error".to_string()),
        };
        let api_err: APIError = repo_err.into();
        let (status, body) = api_err.to_status_and_body();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "INTERNAL_ERROR");
    }

    #[test]
    fn test_repository_other_error_converts_to_api_internal() {
        let repo_err = RepositoryError::OtherError("something weird".to_string());
        let api_err: APIError = repo_err.into();
        let (status, _body) = api_err.to_status_and_body();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_repository_connection_failed_converts_to_api_internal() {
        let repo_err = RepositoryError::ConnectionFailed("connection refused".to_string());
        let api_err: APIError = repo_err.into();
        let (status, body) = api_err.to_status_and_body();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "INTERNAL_ERROR");
        assert!(
            body.details
                .as_ref()
                .and_then(|d| d.get("source"))
                .is_some()
        );
    }

    // ── APIError status codes ─────────────────────────────────────────────────

    #[test]
    fn test_api_unauthorized_status() {
        let (status, body) = APIError::Unauthorized.to_status_and_body();
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "UNAUTHORIZED");
        assert!(body.details.is_none());
    }

    #[test]
    fn test_api_forbidden_status() {
        let (status, body) = APIError::Forbidden.to_status_and_body();
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.code, "FORBIDDEN");
        assert!(body.details.is_none());
    }

    #[test]
    fn test_api_not_found_message_format() {
        let err = APIError::NotFound {
            resource: "user".to_string(),
        };
        let (status, body) = err.to_status_and_body();
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.message, "user not found");
    }

    #[test]
    fn test_api_conflict_status() {
        let err = APIError::Conflict {
            message: "already exists".to_string(),
        };
        let (status, body) = err.to_status_and_body();
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "CONFLICT");
        assert_eq!(body.message, "already exists");
    }

    #[test]
    fn test_api_internal_status() {
        let err = APIError::Internal {
            message: "db down".to_string(),
            details: serde_json::json!({"reason": "timeout"}),
        };
        let (status, body) = err.to_status_and_body();
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "INTERNAL_ERROR");
        assert!(body.details.is_some());
    }

    #[test]
    fn test_api_validation_error_includes_details() {
        let err = APIError::Validation {
            message: "bad input".to_string(),
            details: serde_json::json!({"field": "email"}),
        };
        let (status, body) = err.to_status_and_body();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(body.details.is_some());
    }

    // ── RepositoryError Display ───────────────────────────────────────────────

    #[test]
    fn test_display_not_found() {
        let e = RepositoryError::NotFound("posts".to_string());
        assert_eq!(format!("{e}"), "Not Found: posts");
    }

    #[test]
    fn test_display_duplicate_key() {
        let e = RepositoryError::DuplicateKey("collision".to_string());
        assert_eq!(format!("{e}"), "Duplicate Key: collision");
    }

    #[test]
    fn test_display_connection_failed() {
        let e = RepositoryError::ConnectionFailed("refused".to_string());
        assert_eq!(format!("{e}"), "Connection Failed: refused");
    }

    #[test]
    fn test_display_query_failed_with_source() {
        let e = RepositoryError::QueryFailed {
            message: "oops".to_string(),
            source: Some("syntax error".to_string()),
        };
        let s = format!("{e}");
        assert!(s.contains("oops"));
        assert!(s.contains("syntax error"));
    }

    #[test]
    fn test_display_query_failed_no_source() {
        let e = RepositoryError::QueryFailed {
            message: "oops".to_string(),
            source: None,
        };
        assert_eq!(format!("{e}"), "Query Failed: oops");
    }

    #[test]
    fn test_display_validation_with_field() {
        let e = RepositoryError::Validation {
            message: "required".to_string(),
            field: Some("email".to_string()),
        };
        let s = format!("{e}");
        assert!(s.contains("email"));
        assert!(s.contains("required"));
    }

    #[test]
    fn test_display_validation_no_field() {
        let e = RepositoryError::Validation {
            message: "required".to_string(),
            field: None,
        };
        let s = format!("{e}");
        assert!(s.contains("required"));
    }

    #[test]
    fn test_display_other_error() {
        let e = RepositoryError::OtherError("unknown".to_string());
        assert_eq!(format!("{e}"), "Error: unknown");
    }

    // ── sqlx::Error → RepositoryError conversions ─────────────────────────────

    #[test]
    fn test_sqlx_row_not_found_maps_to_repository_not_found() {
        let err: RepositoryError = sqlx::Error::RowNotFound.into();
        assert!(matches!(err, RepositoryError::NotFound(_)));
    }
}
