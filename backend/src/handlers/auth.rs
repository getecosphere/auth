use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::{
    auth_extractor::AuthUser,
    dto::{
        AuthResponse, ChangePasswordQuery, CheckUsernameRequest, CheckUsernameResponse,
        EmailVerificationStatus, LoginRequest, RegisterQuery, RegisterWithProfileQuery, UserDto,
    },
    email_verification,
    error::{require_non_blank, require_password_strength, AppError, AppResult},
    jwt, password,
    state::AppState,
    user_repo,
};

/// A precomputed, valid bcrypt hash with no corresponding real password.
/// Used to keep login's response time constant whether or not the username
/// exists -- without this, "user not found" returns immediately while
/// "wrong password" pays the full bcrypt cost, and that timing difference
/// is enough to enumerate valid usernames.
const DUMMY_PASSWORD_HASH: &str = "$2b$10$NgQ6Jvr432x5WAphKYFAHOiB/j8WX.ENwhOgv4lALaR1rszL4Xfbe";

/// One recipient of a generic transactional email. Auth resolves the
/// recipient id to an email (recipient identity is Auth's bounded context)
/// and owns the mail provider credentials, but the subject/html are opaque —
/// the calling domain (e.g. marketplace) owns its message content.
#[derive(serde::Deserialize)]
pub struct MailMessage {
    pub recipient_id: String,
    pub subject: String,
    pub html: String,
}

#[derive(serde::Deserialize)]
pub struct MailBatchRequest {
    pub messages: Vec<MailMessage>,
}

pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> AppResult<Json<AuthResponse>> {
    require_non_blank(&[("username", &req.username), ("password", &req.password)])?;

    // APINDO's login screen has historically accepted email as its primary
    // identifier. Keep the shared auth contract compatible with that UX while
    // still allowing users from other estates to sign in by username.
    let user = match user_repo::find_by_username(&state, &req.username).await? {
        Some(user) => Some(user),
        None => user_repo::find_by_email(&state, &req.username).await?,
    };
    let password_hash = user
        .as_ref()
        .map(|u| u.password_hash.as_str())
        .unwrap_or(DUMMY_PASSWORD_HASH);
    let password_ok = password::verify_password(&req.password, password_hash);

    let user = match (user, password_ok) {
        (Some(user), true) => user,
        _ => {
            tracing::warn!(username = %req.username, "login failed: invalid credentials");
            return Err(AppError::BadRequest("Invalid credentials".to_string()));
        }
    };

    Ok(Json(issue_auth_response(&state, &user)?))
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterQuery>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    require_non_blank(&[
        ("username", &req.username),
        ("email", &req.email),
        ("password", &req.password),
        ("name", &req.name),
    ])?;
    require_password_strength(&req.password)?;
    email_verification::ensure_delivery_configured(&state.config)?;

    if user_repo::find_by_username(&state, &req.username)
        .await?
        .is_some()
    {
        tracing::warn!(username = %req.username, "register rejected: username already taken");
        return Err(AppError::Conflict("Username is already taken".to_string()));
    }
    if user_repo::find_by_email(&state, &req.email)
        .await?
        .is_some()
    {
        tracing::warn!(email = %req.email, "register rejected: email already registered");
        return Err(AppError::Conflict(
            "Email is already registered".to_string(),
        ));
    }

    let role = req
        .role
        .filter(|r| !r.is_empty())
        .unwrap_or_else(|| "member".to_string());
    let hashed = password::hash_password(&req.password)?;
    let user = user_repo::insert_user(&state, &req.username, &req.email, &hashed, &req.name, &role)
        .await?;

    if let Err(error) = email_verification::send_for_user(&state, &user).await {
        let _ = email_verification::discard_for_user(&state, &user.id_string()).await;
        let _ = user_repo::discard_unverified_new_user(&state, &user.id_string()).await;
        return Err(error);
    }
    let response = issue_auth_response(&state, &user)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn register_with_profile(
    State(state): State<AppState>,
    Json(req): Json<RegisterWithProfileQuery>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    require_non_blank(&[
        ("email", &req.email),
        ("password", &req.password),
        ("name", &req.name),
    ])?;
    require_password_strength(&req.password)?;
    email_verification::ensure_delivery_configured(&state.config)?;

    if user_repo::find_by_email(&state, &req.email)
        .await?
        .is_some()
    {
        return Err(AppError::Conflict(
            "Email is already registered".to_string(),
        ));
    }

    let username = req
        .email
        .split('@')
        .next()
        .unwrap_or(&req.email)
        .to_string();
    let hashed = password::hash_password(&req.password)?;
    let user =
        user_repo::insert_user(&state, &username, &req.email, &hashed, &req.name, "member").await?;

    if let Err(error) = email_verification::send_for_user(&state, &user).await {
        let _ = email_verification::discard_for_user(&state, &user.id_string()).await;
        let _ = user_repo::discard_unverified_new_user(&state, &user.id_string()).await;
        return Err(error);
    }
    let response = issue_auth_response(&state, &user)?;
    Ok((StatusCode::CREATED, Json(response)))
}

pub async fn verify_email(
    State(state): State<AppState>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> AppResult<Json<serde_json::Value>> {
    let token = query
        .get("token")
        .ok_or_else(|| AppError::BadRequest("Verification token is required".into()))?;
    email_verification::verify(&state, token).await?;
    Ok(Json(
        serde_json::json!({ "verified": true, "message": "Email berhasil diverifikasi. Kamu sekarang dapat menggunakan Marketplace dan negosiasi." }),
    ))
}

pub async fn resend_verification(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let user = user_repo::find_by_id(&state, &auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    let message_id = email_verification::send_for_user(&state, &user).await?;
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "accepted": true,
            "messageId": message_id,
            "message": "Permintaan email verifikasi diterima oleh Brevo. Periksa inbox dan folder spam."
        })),
    ))
}

/// Generic transactional mail delivery. Auth owns the mail provider
/// credentials and recipient identity (user id -> email); the caller owns the
/// message content. This is deliberately content-agnostic so no domain's
/// business data or templates ever leak into Auth — marketplace renders its
/// own sale emails and sends them through this contract. Returns accepted
/// counts so a transient provider failure never undoes the caller's own
/// transactional state change.
pub async fn send_mail(
    State(state): State<AppState>,
    _sender: AuthUser,
    Json(request): Json<MailBatchRequest>,
) -> AppResult<Json<serde_json::Value>> {
    let mut accepted = 0usize;
    let mut skipped = 0usize;
    for message in request.messages {
        let Some(user) = user_repo::find_by_id(&state, &message.recipient_id).await? else {
            skipped += 1;
            continue;
        };
        match email_verification::send_transactional_mail(
            &state,
            &user,
            &message.subject,
            &message.html,
        )
        .await
        {
            Ok(_) => accepted += 1,
            Err(_error) => {
                skipped += 1;
                tracing::warn!(user_id = %user.id_string(), "Transactional email could not be sent");
            }
        }
    }
    Ok(Json(
        serde_json::json!({ "accepted": accepted, "skipped": skipped }),
    ))
}

pub async fn verification_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<EmailVerificationStatus>> {
    let user = user_repo::find_by_id(&state, &auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    Ok(Json(EmailVerificationStatus {
        email_verified: !state.config.email_verification_required
            || user.email_verified_at.is_some(),
        verification_expires_in_seconds: state.config.email_verification_ttl_hours * 3600,
    }))
}

/// Returns the authenticated identity for sibling domains.  This keeps JWT
/// verification in Auth, instead of making every domain carry a copy of the
/// signing secret just to learn who made a request.
pub async fn session_identity(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<UserDto>> {
    let user = user_repo::find_by_id(&state, &auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    Ok(Json(UserDto::from(&user)))
}

/// Returns the access rights (permission tokens) the authenticated user
/// currently holds, e.g. `["verified_user"]`. Auth is the settings owner —
/// it reports the rights — but the rules that map rights to capabilities live
/// in the composition domain, so this endpoint deliberately exposes raw
/// tokens, not business capabilities.
pub async fn access_rights(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<serde_json::Value>> {
    let user = user_repo::find_by_id(&state, &auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound("User not found".into()))?;
    Ok(Json(serde_json::json!({
        "userId": user.id_string(),
        "emailVerified": user.email_verified_at.is_some(),
        "permissions": user.access_rights(),
    })))
}

pub async fn change_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(req): Query<ChangePasswordQuery>,
) -> AppResult<StatusCode> {
    require_non_blank(&[
        ("currentPassword", &req.current_password),
        ("newPassword", &req.new_password),
    ])?;
    require_password_strength(&req.new_password)?;

    let user = user_repo::find_by_id(&state, &auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {}", auth.user_id)))?;

    if !password::verify_password(&req.current_password, &user.password_hash) {
        tracing::warn!(user_id = %auth.user_id, "change-password rejected: wrong current password");
        return Err(AppError::BadRequest(
            "Current password is incorrect".to_string(),
        ));
    }

    let hashed = password::hash_password(&req.new_password)?;
    user_repo::update_password(&state, &auth.user_id, &hashed).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn update_identity(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<crate::dto::UpdateIdentityRequest>,
) -> AppResult<Json<UserDto>> {
    require_non_blank(&[("name", &req.name)])?;
    user_repo::update_name(&state, &auth.user_id, req.name.trim()).await?;
    let user = user_repo::find_by_id(&state, &auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {}", auth.user_id)))?;
    Ok(Json(UserDto::from(&user)))
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
