use chrono::Utc;
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// Mirrors the claim shape produced by the previous Java (jjwt) implementation:
/// HS512, `sub`/`username`/`role`/`iat`/`exp`. lms-backend's JwtTokenProvider parses
/// tokens with this exact shape, so it must not change without updating both services.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub username: String,
    pub role: String,
    pub iat: i64,
    pub exp: i64,
}

pub fn generate_token(
    secret: &str,
    user_id: &str,
    username: &str,
    role: &str,
    expiration_ms: i64,
) -> anyhow::Result<String> {
    let now = Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::milliseconds(expiration_ms)).timestamp(),
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
