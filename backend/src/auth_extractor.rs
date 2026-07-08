use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

use crate::{error::AppError, jwt, state::AppState};

/// Authenticated principal derived from a validated JWT. Mirrors what
/// `JwtAuthenticationFilter` used to populate on the Spring SecurityContext.
/// `user_id`/`username` aren't read by any handler yet (none of the ported
/// endpoints do ownership checks, only role checks) but are kept since
/// they're part of what a validated token actually asserts.
#[allow(dead_code)]
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
}

pub struct AuthRejection(String);

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "error": "Unauthorized",
                "message": format!("Unauthorized: {}", self.0),
            })),
        )
            .into_response()
    }
}

#[axum::async_trait]
impl FromRequestParts<AppState> for AuthUser {
    type Rejection = AuthRejection;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| AuthRejection("missing bearer token".to_string()))?;

        let token = header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AuthRejection("missing bearer token".to_string()))?;

        let claims = jwt::validate_token(&state.config.jwt_secret, token)
            .ok_or_else(|| AuthRejection("invalid or expired token".to_string()))?;

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
        })
    }
}

impl AuthUser {
    /// Mirrors `@PreAuthorize("hasAnyRole(...)")`, which fails with 403 (not
    /// 401 like a missing/invalid token does).
    pub fn require_role(&self, allowed: &[&str]) -> Result<(), AppError> {
        let role_upper = self.role.to_uppercase();
        if allowed.iter().any(|r| r.to_uppercase() == role_upper) {
            Ok(())
        } else {
            Err(AppError::Forbidden("Access denied".to_string()))
        }
    }
}
