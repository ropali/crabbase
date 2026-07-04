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
        }
    }
}
