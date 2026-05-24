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
    QueryFailed(String),
    OtherError(String),
}

impl From<sqlx::Error> for RepositoryError {
    fn from(err: sqlx::Error) -> Self {
        match err {
            sqlx::Error::RowNotFound => {
                RepositoryError::NotFound("Data not found for this collection".to_string())
            }
            sqlx::Error::Database(db_err) => {
                let message = db_err.message().to_string();
                if let Some((_, table)) = message.split_once("no such table:") {
                    let table = table.trim();
                    let resource = if table.is_empty() {
                        "Collection".to_string()
                    } else {
                        format!("Collection `{table}`")
                    };
                    return RepositoryError::NotFound(resource);
                }
                RepositoryError::QueryFailed("Failed to execute the database query".to_string())
            }
            _ => RepositoryError::QueryFailed("Failed to execute the database query".to_string()),
        }
    }
}

// Conversion from sqlx::Error to RepositoryError allows using the `?` operator
impl From<RepositoryError> for APIError {
    fn from(value: RepositoryError) -> Self {
        match value {
            RepositoryError::NotFound(msg) => APIError::NotFound { resource: msg },
            RepositoryError::DuplicateKey(msg) => APIError::Conflict { message: msg },
            RepositoryError::ConnectionFailed(msg) | RepositoryError::QueryFailed(msg) => {
                APIError::Internal {
                    message: "Something went wrong!".to_string(),
                    details: serde_json::Value::String(msg),
                }
            }
            RepositoryError::OtherError(msg) => APIError::Internal {
                message: "Something went wrong".to_string(),
                details: serde_json::Value::String(msg),
            },
        }
    }
}
