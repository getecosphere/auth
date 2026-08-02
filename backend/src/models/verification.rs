use bson::DateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailVerification {
    pub id: String,
    #[serde(rename = "userId")]
    pub user_id: String,
    #[serde(rename = "tokenHash")]
    pub token_hash: String,
    #[serde(rename = "expiresAt")]
    pub expires_at: DateTime,
    #[serde(rename = "usedAt", skip_serializing_if = "Option::is_none")]
    pub used_at: Option<DateTime>,
    #[serde(rename = "createdAt")]
    pub created_at: DateTime,
}
