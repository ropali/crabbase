use axum::{
    extract::FromRequestParts,
    http::{Request, request::Parts},
    middleware::Next,
    response::Response,
};
use crabbase_auth::auth::verify_token;
use crabbase_core::errors::APIError;
use crabbase_db::repositories::auth::AuthUser;

use crate::state::AppState;

pub struct AuthenticatedUser(pub AuthUser);

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = APIError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        eprint!("{:?}", parts);
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or(APIError::Unauthorized)?;

        eprint!("{}", auth_header);
        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or(APIError::Unauthorized)?;

        eprint!("{}", token);

        let claims = verify_token(token).map_err(|_| APIError::Unauthorized)?;

        let user = state.auth_service().verify_session(&claims).await?;
        Ok(AuthenticatedUser(user))
    }
}

pub async fn require_auth(
    AuthenticatedUser(user): AuthenticatedUser,
    mut request: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, APIError> {
    request.extensions_mut().insert(user);

    Ok(next.run(request).await)
}
