use axum::{
    extract::{Path, State},
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
};

use crate::{auth_extractor::AuthUser, error::AppResult, state::AppState, storage};

const PROFILE_ROLES: &[&str] = &["OWNER", "MENTOR", "MEMBER"];

pub async fn download_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> AppResult<Response> {
    let file = storage::read_file(&state, &file_id).await?;
    let is_image = file.content_type.starts_with("image/");

    let mut response = (
        StatusCode::OK,
        [(header::CONTENT_TYPE, file.content_type.clone())],
        file.bytes,
    )
        .into_response();

    if !is_image {
        response.headers_mut().insert(
            header::CONTENT_DISPOSITION,
            HeaderValue::from_static("attachment; filename=\"file\""),
        );
    }
    Ok(response)
}

pub async fn view_file(
    State(state): State<AppState>,
    Path(file_id): Path<String>,
) -> AppResult<Response> {
    let file = storage::read_file(&state, &file_id).await?;
    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, file.content_type)],
        file.bytes,
    )
        .into_response())
}

pub async fn delete_file(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(file_id): Path<String>,
) -> AppResult<StatusCode> {
    auth.require_role(PROFILE_ROLES)?;
    storage::delete_file(&state, &file_id).await?;
    Ok(StatusCode::NO_CONTENT)
}
