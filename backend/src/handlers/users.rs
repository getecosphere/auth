use axum::{
    extract::{Multipart, Path, State},
    http::StatusCode,
    Json,
};
use serde_json::{json, Value};

use crate::{
    auth_extractor::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
    storage, user_repo,
};

const PROFILE_ROLES: &[&str] = &["OWNER", "MENTOR", "MEMBER"];

async fn read_single_file(mut multipart: Multipart) -> AppResult<(Option<String>, Vec<u8>)> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(format!("Invalid multipart body: {e}")))?
    {
        if field.name() == Some("file") {
            let content_type = field.content_type().map(|s| s.to_string());
            let bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(format!("Failed to read upload: {e}")))?;
            return Ok((content_type, bytes.to_vec()));
        }
    }
    Err(AppError::BadRequest("Missing 'file' field".to_string()))
}

pub async fn upload_avatar(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<Value>)> {
    auth.require_role(PROFILE_ROLES)?;
    user_repo::find_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {user_id}")))?;

    let (content_type, bytes) = read_single_file(multipart).await?;
    let uploaded =
        storage::upload_identity_image(&state, "avatars", &user_id, content_type.as_deref(), bytes)
            .await?;
    user_repo::update_avatar_url(&state, &user_id, &uploaded.file_url).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "avatarUrl": uploaded.file_url })),
    ))
}

pub async fn upload_cover_photo(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    multipart: Multipart,
) -> AppResult<(StatusCode, Json<Value>)> {
    auth.require_role(PROFILE_ROLES)?;
    user_repo::find_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {user_id}")))?;

    let (content_type, bytes) = read_single_file(multipart).await?;
    let uploaded = storage::upload_identity_image(
        &state,
        "cover-photos",
        &user_id,
        content_type.as_deref(),
        bytes,
    )
    .await?;
    user_repo::update_cover_photo_url(&state, &user_id, &uploaded.file_url).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({ "coverPhotoUrl": uploaded.file_url })),
    ))
}

pub async fn deactivate_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
) -> AppResult<StatusCode> {
    auth.require_role(&["OWNER"])?;
    user_repo::find_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {user_id}")))?;
    user_repo::soft_delete(&state, &user_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
