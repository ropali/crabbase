use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::json;

#[derive(Debug)]
pub enum APIError {
    NotFound(String),
    Unauthorised(String),
    BadRequest(String),
    Internal(String),
}

impl IntoResponse for APIError {
    fn into_response(self) -> Response {
        let (status, msg) = match self {
            APIError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            APIError::Unauthorised(msg) => (StatusCode::FORBIDDEN, msg),
            APIError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            APIError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "status": status.as_u16(),
            "message": msg,
        }));

        (status, body).into_response()
    }
}
