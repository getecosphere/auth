use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::models::user::User;

/// Credential-domain view of a user. Only carries fields auth actually owns;
/// profile fields (bio, experiences, ...) live in lms-backend now and are
/// intentionally absent rather than sent as null.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDto {
    pub id: String,
    pub name: String,
    pub username: String,
    pub email: String,
    pub email_verified: bool,
    pub role: String,
    /// Access rights the user currently holds (e.g. `verified_user`). Auth
    /// reports the rights; the rules that map them to capabilities live in the
    /// composition domain.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<&User> for UserDto {
    fn from(user: &User) -> Self {
        UserDto {
            id: user.id_string(),
            name: user.name.clone(),
            username: user.username.clone(),
            email: user.email.clone(),
            email_verified: user.email_verified_at.is_some(),
            role: user.role.clone(),
            permissions: user.access_rights(),
            created_at: user.created_at.to_chrono(),
            updated_at: user.updated_at.to_chrono(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthResponse {
    pub token: String,
    pub user: UserDto,
    pub expires_in: i64,
    /// The single active session id this token is bound to (also the JWT
    /// `sid` claim). A newer login revokes it — the token then dies 401.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub session_id: String,
}

/// Single-session status for the presented bearer token. `active:false` means
/// a newer login superseded this session (or it was logged out / expired) —
/// the estate gateway returns 401 in that case and a frontend should clear its
/// local session.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStatus {
    pub session_id: String,
    pub active: bool,
    pub expires_in_seconds: i64,
    pub user_id: String,
    pub username: String,
    pub role: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailVerificationStatus {
    pub email_verified: bool,
    pub verification_expires_in_seconds: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterQuery {
    pub username: String,
    pub email: String,
    pub password: String,
    pub name: String,
    pub role: Option<String>,
}

/// whatsappNumber/province used to live on this same request in the Java
/// version, but those are profile fields owned by lms-backend now — the
/// frontend sends them to lms's profile endpoint in a follow-up call instead
/// of auth silently accepting and discarding them.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegisterWithProfileQuery {
    pub email: String,
    pub password: String,
    pub name: String,
}

/// Target user comes from the authenticated JWT, not a client-supplied id --
/// the old contract accepted an arbitrary userId with no auth at all, which
/// let anyone change anyone's password.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangePasswordQuery {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VerifyPasswordRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct VerifyPasswordResponse {
    pub valid: bool,
}

/// Email is deliberately the only field accepted by the request endpoint.
/// Its response is always generic so it cannot reveal account existence.
#[derive(Debug, Deserialize)]
pub struct PasswordResetRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PasswordResetConfirmRequest {
    pub token: String,
    pub new_password: String,
}

/// Identity fields belong to Auth. The authenticated subject is the only
/// target; callers never supply an arbitrary user id.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateIdentityRequest {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct CheckUsernameRequest {
    pub usernames: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct CheckUsernameResponse {
    pub existing: Vec<String>,
}
