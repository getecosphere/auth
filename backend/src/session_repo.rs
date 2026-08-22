use bson::{doc, oid::ObjectId};
use mongodb::{options::IndexOptions, Collection, IndexModel};

use crate::{error::AppError, models::session::Session, state::AppState};

fn sessions(state: &AppState) -> Collection<Session> {
    state.db.collection("sessions")
}

/// Ensure the sessions collection has the indexes auth relies on: one for
/// fast "active session per user" lookups and a TTL index that auto-expires
/// stale session rows. Idempotent.
pub async fn ensure_indexes(state: &AppState) -> Result<(), AppError> {
    let col = sessions(state);
    col.create_index(
        IndexModel::builder().keys(doc! { "userId": 1 }).build(),
        None,
    )
    .await?;
    col.create_index(
        IndexModel::builder()
            .keys(doc! { "expiresAt": 1 })
            .options(
                IndexOptions::builder()
                    .expire_after(Some(std::time::Duration::from_secs(0)))
                    .build(),
            )
            .build(),
        None,
    )
    .await?;
    Ok(())
}

/// Create a new active session for the user, invalidating every previous one.
/// Returns the session whose id becomes the token's `sid` claim.
pub async fn create_session(
    state: &AppState,
    user_id: &str,
    device: Option<&str>,
) -> Result<Session, AppError> {
    let oid = ObjectId::parse_str(user_id).map_err(|_| {
        AppError::Internal(anyhow::anyhow!("invalid user id for session: {user_id}"))
    })?;

    // Revoke all prior sessions first — one active session per account.
    sessions(state)
        .delete_many(doc! { "userId": oid }, None)
        .await?;

    let now = bson::DateTime::now();
    let session = Session {
        id: None,
        user_id: oid,
        created_at: now,
        // Sessions expire with their token so both die together.
        expires_at: bson::DateTime::from_millis(
            now.timestamp_millis() + state.config.jwt_expiration_ms,
        ),
        device: device.map(str::to_string),
    };
    let result = sessions(state).insert_one(&session, None).await?;
    let id = result.inserted_id.as_object_id().ok_or_else(|| {
        AppError::Internal(anyhow::anyhow!("session insert did not return an ObjectId"))
    })?;
    Ok(Session {
        id: Some(id),
        ..session
    })
}

/// Whether the user already holds an active (unexpired) session. Login uses
/// this to reject a second sign-in while one device is already signed in —
/// the existing session is preserved instead of being revoked.
pub async fn has_active_session(state: &AppState, user_id: &str) -> Result<bool, AppError> {
    let Ok(oid) = ObjectId::parse_str(user_id) else {
        return Ok(false);
    };
    let now = bson::DateTime::now();
    let found = sessions(state)
        .find_one(doc! { "userId": oid, "expiresAt": { "$gt": now } }, None)
        .await?;
    Ok(found.is_some())
}

/// Look up an active (unexpired) session by its id. Returns None for a
/// revoked, expired, or never-issued id.
pub async fn find_active_session(state: &AppState, sid: &str) -> Result<Option<Session>, AppError> {
    let Ok(oid) = ObjectId::parse_str(sid) else {
        return Ok(None);
    };
    let now = bson::DateTime::now();
    Ok(sessions(state)
        .find_one(doc! { "_id": oid, "expiresAt": { "$gt": now } }, None)
        .await?)
}

/// Revoke the current session (logout). Returns whether a session was revoked.
pub async fn revoke_session(state: &AppState, sid: &str) -> Result<bool, AppError> {
    let Ok(oid) = ObjectId::parse_str(sid) else {
        return Ok(false);
    };
    let result = sessions(state)
        .delete_one(doc! { "_id": oid }, None)
        .await?;
    Ok(result.deleted_count > 0)
}

/// Revoke every session after a password reset. This is intentionally broader
/// than logout: anyone holding an older JWT must authenticate again.
pub async fn revoke_all_for_user(state: &AppState, user_id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(user_id)
        .map_err(|_| AppError::BadRequest("Invalid user id for session revocation".into()))?;
    sessions(state)
        .delete_many(doc! { "userId": oid }, None)
        .await?;
    Ok(())
}
