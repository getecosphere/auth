use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::{
    dto::{
        AuthResponse, ChangePasswordQuery, CheckUsernameRequest, CheckUsernameResponse,
        LoginRequest, RegisterQuery, RegisterWithProfileQuery, UserDto,
    },
    error::{require_non_blank, AppError, AppResult},
    jwt, password,
    state::AppState,
    user_repo,
};

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    require_non_blank(&[("username", &req.username), ("password", &req.password)])?;

    let user = user_repo::find_by_username(&state, &req.username)
        .await?
        .ok_or_else(|| AppError::BadRequest("Invalid credentials".to_string()))?;

    if !password::verify_password(&req.password, &user.password_hash) {
        return Err(AppError::BadRequest("Invalid credentials".to_string()));
    }

    Ok(Json(issue_auth_response(&state, &user)?))
}

pub async fn register(
    State(state): State<AppState>,
    Query(req): Query<RegisterQuery>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    require_non_blank(&[
        ("username", &req.username),
        ("email", &req.email),
        ("password", &req.password),
        ("name", &req.name),
    ])?;

    if let Some(existing) = user_repo::find_by_username(&state, &req.username).await? {
        return Ok((StatusCode::CREATED, Json(issue_auth_response(&state, &existing)?)));
    }
    if let Some(existing) = user_repo::find_by_email(&state, &req.email).await? {
        return Ok((StatusCode::CREATED, Json(issue_auth_response(&state, &existing)?)));
    }

    let role = req.role.filter(|r| !r.is_empty()).unwrap_or_else(|| "member".to_string());
    let hashed = password::hash_password(&req.password)?;
    let user = user_repo::insert_user(&state, &req.username, &req.email, &hashed, &req.name, &role).await?;

    let response = issue_auth_response(&state, &user)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn register_with_profile(
    State(state): State<AppState>,
    Query(req): Query<RegisterWithProfileQuery>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    require_non_blank(&[
        ("email", &req.email),
        ("password", &req.password),
        ("name", &req.name),
    ])?;

    if let Some(existing) = user_repo::find_by_email(&state, &req.email).await? {
        return Ok((StatusCode::CREATED, Json(issue_auth_response(&state, &existing)?)));
    }

    let username = req.email.split('@').next().unwrap_or(&req.email).to_string();
    let hashed = password::hash_password(&req.password)?;
    let user = user_repo::insert_user(&state, &username, &req.email, &hashed, &req.name, "member").await?;

    let response = issue_auth_response(&state, &user)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn change_password(
    State(state): State<AppState>,
    Query(req): Query<ChangePasswordQuery>,
) -> AppResult<StatusCode> {
    require_non_blank(&[
        ("userId", &req.user_id),
        ("newPassword", &req.new_password),
    ])?;

    user_repo::find_by_id(&state, &req.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {}", req.user_id)))?;

    let hashed = password::hash_password(&req.new_password)?;
    user_repo::update_password(&state, &req.user_id, &hashed).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn check_existence(
    State(state): State<AppState>,
    Json(req): Json<CheckUsernameRequest>,
) -> AppResult<Json<CheckUsernameResponse>> {
    if req.usernames.is_empty() {
        let mut details = std::collections::HashMap::new();
        details.insert(
            "usernames".to_string(),
            "Username list cannot be empty".to_string(),
        );
        return Err(AppError::Validation(details));
    }

    let existing = user_repo::usernames_in(&state, &req.usernames).await?;
    Ok(Json(CheckUsernameResponse { existing }))
}

/// Internal identity lookup used by lms-backend to hydrate a profile row the
/// first time it meets a userId, and to compose avatarUrl/coverPhotoUrl live
/// on every profile read (auth is the only writer of those two fields).
pub async fn get_user_identity(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> AppResult<Json<UserDto>> {
    let user = user_repo::find_by_id(&state, &id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {id}")))?;
    Ok(Json(UserDto::from(&user)))
}

/// Same as `get_user_identity` but keyed by username, for lms-backend to
/// hydrate a public `/users/username/{username}` profile view it has never
/// seen locally.
pub async fn get_user_identity_by_username(
    State(state): State<AppState>,
    Path(username): Path<String>,
) -> AppResult<Json<UserDto>> {
    let user = user_repo::find_by_username(&state, &username)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {username}")))?;
    Ok(Json(UserDto::from(&user)))
}

fn issue_auth_response(
    state: &AppState,
    user: &crate::models::user::User,
) -> AppResult<AuthResponse> {
    let token = jwt::generate_token(
        &state.config.jwt_secret,
        &user.id_string(),
        &user.username,
        &user.role,
        state.config.jwt_expiration_ms,
    )?;

    Ok(AuthResponse {
        token,
        user: UserDto::from(user),
        expires_in: state.config.jwt_expiration_ms / 1000,
    })
}
