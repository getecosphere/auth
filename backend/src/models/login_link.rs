use bson::DateTime;
use serde::{Deserialize, Serialize};

/// One-time passwordless sign-in credential, minted when a user is locked out
/// of the single active session (e.g. the old device is lost). The secret is
/// never persisted in plaintext: only its bcrypt hash is stored, and the
/// record is consumed atomically after a successful login-link sign-in.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginLink {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "tokenHash")]
    pub token_hash: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime,
    #[serde(rename = "usedAt", default, skip_serializing_if = "Option::is_none")]
    pub used_at: Option<DateTime>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime,
}
