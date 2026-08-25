use bson::{doc, oid::ObjectId};
use mongodb::Collection;

use crate::{error::AppError, models::user::User, state::AppState};

fn users(state: &AppState) -> Collection<User> {
    state.db.collection("users")
}

pub async fn find_by_username(state: &AppState, username: &str) -> Result<Option<User>, AppError> {
    Ok(users(state)
        .find_one(doc! { "username": username, "deletedAt": null }, None)
        .await?)
}

pub async fn find_by_email(state: &AppState, email: &str) -> Result<Option<User>, AppError> {
    Ok(users(state)
        .find_one(doc! { "email": email, "deletedAt": null }, None)
        .await?)
}

pub async fn find_by_id(state: &AppState, id: &str) -> Result<Option<User>, AppError> {
    let Ok(oid) = ObjectId::parse_str(id) else {
        return Ok(None);
    };
    Ok(users(state)
        .find_one(doc! { "_id": oid, "deletedAt": null }, None)
        .await?)
}

pub async fn usernames_in(state: &AppState, usernames: &[String]) -> Result<Vec<String>, AppError> {
    let mut cursor = users(state)
        .find(doc! { "username": { "$in": usernames } }, None)
        .await?;
    let mut found = Vec::new();
    use futures_util::StreamExt;
    while let Some(user) = cursor.next().await {
        found.push(user?.username);
    }
    Ok(found)
}

pub async fn insert_user(
    state: &AppState,
    username: &str,
    email: &str,
    password_hash: &str,
    name: &str,
    role: &str,
) -> Result<User, AppError> {
    let now = bson::DateTime::now();
    let user = User {
        id: None,
        username: username.to_string(),
        email: email.to_string(),
        password_hash: password_hash.to_string(),
        name: name.to_string(),
        role: role.to_string(),
        roles: vec![role.to_string()],
        permissions: Vec::new(),
        email_verified_at: None,
        created_at: now,
        updated_at: now,
        deleted_at: None,
    };
    let result = users(state).insert_one(&user, None).await?;
    let id = result
        .inserted_id
        .as_object_id()
        .ok_or_else(|| AppError::Internal(anyhow::anyhow!("insert did not return an ObjectId")))?;
    Ok(User {
        id: Some(id),
        ..user
    })
}

pub async fn mark_email_verified(state: &AppState, id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(id)
        .map_err(|_| AppError::BadRequest(format!("Invalid user id: {id}")))?;
    users(state).update_one(
        doc! { "_id": oid, "deletedAt": null },
        doc! { "$set": { "emailVerifiedAt": bson::DateTime::now(), "updatedAt": bson::DateTime::now() } },
        None,
    ).await?;
    Ok(())
}

pub async fn update_password(
    state: &AppState,
    id: &str,
    password_hash: &str,
) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(id)
        .map_err(|_| AppError::BadRequest(format!("Invalid user id: {id}")))?;
    users(state)
        .update_one(
            doc! { "_id": oid },
            doc! { "$set": { "passwordHash": password_hash, "updatedAt": bson::DateTime::now() } },
            None,
        )
        .await?;
    Ok(())
}

pub async fn update_name(state: &AppState, id: &str, name: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(id)
        .map_err(|_| AppError::BadRequest(format!("Invalid user id: {id}")))?;
    users(state)
        .update_one(
            doc! { "_id": oid, "deletedAt": null },
            doc! { "$set": { "name": name, "updatedAt": bson::DateTime::now() } },
            None,
        )
        .await?;
    Ok(())
}

pub async fn soft_delete(state: &AppState, id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(id)
        .map_err(|_| AppError::BadRequest(format!("Invalid user id: {id}")))?;
    users(state)
        .update_one(
            doc! { "_id": oid },
            doc! { "$set": { "deletedAt": bson::DateTime::now() } },
            None,
        )
        .await?;
    Ok(())
}

/// Used only to roll back a newly-created, unverified account when its first
/// verification email cannot be sent. Normal user deletion remains soft.
pub async fn discard_unverified_new_user(state: &AppState, id: &str) -> Result<(), AppError> {
    let oid = ObjectId::parse_str(id)
        .map_err(|_| AppError::BadRequest(format!("Invalid user id: {id}")))?;
    users(state)
        .delete_one(doc! { "_id": oid, "emailVerifiedAt": null }, None)
        .await?;
    Ok(())
}
