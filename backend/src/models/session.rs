use bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

/// A login session — the auth LXS owns the session boundary. Exactly one
/// active session exists per user at any time: issuing a new session revokes
/// every older one, so the same account cannot stay signed in on two devices.
///
/// The session id is carried inside the JWT as the `sid` claim. Auth's own
/// middleware and the estate gateway (via `session-status`) both reject a
/// token whose `sid` is no longer the active session, so a stale login dies
/// server-side the moment a newer login happens.
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
