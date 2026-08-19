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
pub struct AuthUser {
    pub user_id: String,
    pub username: String,
    pub role: String,
    /// Session id (`sid` claim) when the token carries one. Empty for legacy
    /// pre-session tokens (accepted only while `SESSION_REQUIRED=false`).
    pub sid: String,
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

        // Single-session enforcement: the token must carry the user's *current*
        // active session id. A revoked/expired/never-issued session means a
        // newer login superseded this one — reject it server-side.
        if claims.sid.is_empty() {
            if state.config.session_required {
                return Err(AuthRejection(
                    "session required — please sign in again".to_string(),
                ));
            }
        } else {
            let session = crate::session_repo::find_active_session(state, &claims.sid)
                .await
                .map_err(|_| AuthRejection("session check failed".to_string()))?
                .ok_or_else(|| {
                    AuthRejection("session no longer active — please sign in again".to_string())
                })?;
            if !session.user_id.to_hex().eq_ignore_ascii_case(&claims.sub) {
                return Err(AuthRejection("session does not match token".to_string()));
            }
        }

        Ok(AuthUser {
            user_id: claims.sub,
            username: claims.username,
            role: claims.role,
            sid: claims.sid,
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
            tracing::warn!(
                user_id = %self.user_id,
                username = %self.username,
                role = %self.role,
                required = ?allowed,
                "access denied: role not permitted"
            );
            Err(AppError::Forbidden("Access denied".to_string()))
        }
    }
}
