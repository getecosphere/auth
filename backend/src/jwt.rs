use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Mirrors the claim shape produced by the previous Java (jjwt) implementation:
/// HS512, `sub`/`username`/`role`/`iat`/`exp`. lms-backend's JwtTokenProvider parses
/// tokens with this exact shape, so it must not change without updating both services.
///
/// `sid` (session id) is the single-session binding: auth mints one active
/// session per user on login/register and rejects any token whose `sid` is no
/// longer the active one. A missing `sid` marks a legacy pre-session token —
/// accepted only while `SESSION_REQUIRED=false` (see auth_extractor).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub roles: Vec<String>,
    pub iat: i64,
    pub exp: i64,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub sid: String,
}

pub fn generate_token(
    secret: &str,
    user_id: &str,
    username: &str,
    role: &str,
    roles: &[String],
    session_id: &str,
    expiration_ms: i64,
) -> anyhow::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        roles: roles.to_vec(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::milliseconds(expiration_ms)).timestamp(),
        sid: session_id.to_string(),
    };

    let token = encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )?;
    Ok(token)
}

pub fn validate_token(secret: &str, token: &str) -> Option<Claims> {
    let validation = Validation::new(Algorithm::HS512);
    decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()
    .map(|data| data.claims)
}
