use bson::DateTime;
use serde::{Deserialize, Serialize};

/// One-time password reset credential. The secret is never persisted in
/// plaintext: only its bcrypt hash is stored, and the record is consumed
/// atomically after a successful reset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PasswordReset {
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
