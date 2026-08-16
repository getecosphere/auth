use axum::{extract::{Path, State}, http::StatusCode};

use crate::{
    auth_extractor::AuthUser,
    error::{AppError, AppResult},
    state::AppState,
    user_repo,
};

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
    tracing::warn!(
        target_user_id = %user_id,
        deactivated_by = %auth.user_id,
        "account deactivated"
    );
    Ok(StatusCode::NO_CONTENT)
}
