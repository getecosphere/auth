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
    /// Explicit access-right grants (opaque tokens, e.g. `moderator`). Auth
    /// stores and returns these but never interprets their meaning; the
    /// business rules that decide what each grant *allows* live in the
    /// composition (core) domain, not here.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub permissions: Vec<String>,
    #[serde(rename = "emailVerifiedAt", skip_serializing_if = "Option::is_none")]
    pub email_verified_at: Option<bson::DateTime>,
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

    /// The effective access-right set: identity-derived grants (a verified
    /// email always grants `verified_user`) combined with any explicit grants.
    /// Auth computes which rights a user *holds* but never what those rights
    /// *allow* — capability mapping is the composition's rule.
    pub fn access_rights(&self) -> Vec<String> {
        let mut rights = self.permissions.clone();
        if self.email_verified_at.is_some() && !rights.iter().any(|r| r == "verified_user") {
            rights.push("verified_user".to_string());
        }
        rights
    }
}
