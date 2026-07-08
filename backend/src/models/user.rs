use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

/// Credential-domain user. Historical documents in this collection may still
/// carry profile fields (headline, bio, experiences, ...) from before the
/// profile domain moved to lms-backend; those are intentionally not modeled
/// here and are left untouched since all mutations use targeted `$set`
/// updates rather than whole-document replacement.
///
/// Timestamps use `bson::DateTime` rather than `chrono::DateTime<Utc>`
/// directly: mixing the two causes a deserialize mismatch as soon as a
/// timestamp is written through a raw `doc! { "$set": ... }` update instead
/// of the typed struct serializer. Converted to `chrono` only at the DTO
/// boundary for JSON output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub username: String,
    pub email: String,
    #[serde(rename = "passwordHash")]
    pub password_hash: String,
    pub name: String,
    pub role: String,
    #[serde(rename = "avatarUrl", skip_serializing_if = "Option::is_none")]
    pub avatar_url: Option<String>,
    #[serde(rename = "coverPhotoUrl", skip_serializing_if = "Option::is_none")]
    pub cover_photo_url: Option<String>,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "updatedAt")]
    pub updated_at: bson::DateTime,
    #[serde(rename = "deletedAt", skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<bson::DateTime>,
}

impl User {
    pub fn id_string(&self) -> String {
        self.id.map(|id| id.to_hex()).unwrap_or_default()
    }
}
