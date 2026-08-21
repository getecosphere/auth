use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

/// A login session — the auth LXS owns the session boundary. Exactly one
/// active session exists per user at any time: login **rejects** a second
/// sign-in while a session is already active (409) instead of minting a
/// competing one, so the existing device stays signed in and the same
/// account cannot live on two devices at once.
///
/// The session id is carried inside the JWT as the `sid` claim. Auth's own
/// middleware and the estate gateway (via `session-status`) both reject a
/// token whose `sid` is no longer the active session, so a stale login dies
/// server-side the moment its session is revoked (logout) or expires.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    #[serde(rename = "userId")]
    pub user_id: ObjectId,
    #[serde(rename = "createdAt")]
    pub created_at: bson::DateTime,
    #[serde(rename = "expiresAt")]
    pub expires_at: bson::DateTime,
    /// Optional client label (user-agent-ish) for the session.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device: Option<String>,
}

impl Session {
    pub fn id_string(&self) -> String {
        self.id.map(|id| id.to_hex()).unwrap_or_default()
    }
}
