use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};

use crate::{
    auth_extractor::AuthUser,
    dto::{
        AuthResponse, ChangePasswordQuery, CheckUsernameRequest, CheckUsernameResponse,
        EmailVerificationStatus, LoginLinkConfirmRequest, LoginLinkRequest, LoginRequest,
        PasswordResetConfirmRequest, PasswordResetRequest, RegisterQuery,
        RegisterWithProfileQuery, UserDto, VerifyPasswordRequest, VerifyPasswordResponse,
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
#[serde(rename_all = "camelCase")]
pub struct AdminRolesRequest {
    pub roles: Vec<String>,
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

    // Mailbox ownership is an authentication prerequisite, not merely a
    // capability flag after a session has already been minted.
    if state.config.email_verification_required && user.email_verified_at.is_none() {
        return Err(AppError::Forbidden(
            "Verify your email before signing in. Check your inbox for the verification link."
                .to_string(),
        ));
    }

    // Single active session: a second sign-in while one device is already
    // signed in is rejected rather than silently revoking the existing
    // session — the current device stays logged in and the caller gets an
    // accurate reason instead of a generic failure. Sign out first (or wait
    // for the session to expire) to sign in again.
    if crate::session_repo::has_active_session(&state, &user.id_string()).await? {
        tracing::warn!(
            user_id = %user.id_string(),
            username = %req.username,
            "login rejected: account already has an active session"
        );
        return Err(AppError::Conflict(
            "Already signed in on another device. Sign out from that device first.".to_string(),
        ));
    }

    Ok(Json(issue_auth_response(&state, &user).await?))
}

pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterQuery>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    register_user(&state, req, true).await
}

/// Superadmin-only provisioning lane. It is intentionally outside the public
/// credential-stuffing limiter: an estate administrator may legitimately
/// create a classroom of accounts in one request. The bearer token is still
/// validated (including its active session) before this code can run.
pub async fn admin_register(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<RegisterQuery>,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    auth.require_role(&["superadmin"])?;
    register_user(&state, req, false).await
}

/// Changes roles only after a verified superadmin request. This is an Auth
/// ownership boundary: Assessment decides approval, Auth owns the claims.
pub async fn admin_update_roles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(req): Json<AdminRolesRequest>,
) -> AppResult<Json<UserDto>> {
    auth.require_role(&["superadmin"])?;
    let roles: Vec<String> = req.roles.into_iter().map(|r| r.trim().to_lowercase()).filter(|r| !r.is_empty()).collect();
    if roles.is_empty() || roles.iter().any(|role| !state.config.allowed_roles.is_empty() && !state.config.allowed_roles.iter().any(|allowed| allowed == role)) {
        return Err(AppError::BadRequest("Role tidak diizinkan untuk estate ini".into()));
    }
    user_repo::replace_roles(&state, &user_id, &roles).await?;
    crate::session_repo::revoke_all_for_user(&state, &user_id).await?;
    let user = user_repo::find_by_id(&state, &user_id).await?.ok_or_else(|| AppError::NotFound("User tidak ditemukan".into()))?;
    Ok(Json(UserDto::from(&user)))
}

pub async fn admin_add_roles(State(state): State<AppState>, auth: AuthUser, Path(user_id): Path<String>, Json(req): Json<AdminRolesRequest>) -> AppResult<Json<UserDto>> {
    auth.require_role(&["superadmin"])?;
    let roles: Vec<String> = req.roles.into_iter().map(|r| r.trim().to_lowercase()).filter(|r| !r.is_empty()).collect();
    if roles.is_empty() || roles.iter().any(|role| !state.config.allowed_roles.is_empty() && !state.config.allowed_roles.iter().any(|allowed| allowed == role)) { return Err(AppError::BadRequest("Role tidak diizinkan untuk estate ini".into())); }
    user_repo::add_roles(&state, &user_id, &roles).await?;
    crate::session_repo::revoke_all_for_user(&state, &user_id).await?;
    let user = user_repo::find_by_id(&state, &user_id).await?.ok_or_else(|| AppError::NotFound("User tidak ditemukan".into()))?;
    Ok(Json(UserDto::from(&user)))
}

async fn register_user(
    state: &AppState,
    req: RegisterQuery,
    issue_session: bool,
) -> AppResult<(StatusCode, Json<AuthResponse>)> {
    require_non_blank(&[
        ("username", &req.username),
        ("email", &req.email),
        ("password", &req.password),
        ("name", &req.name),
    ])?;
    require_password_strength(&req.password)?;
    email_verification::ensure_delivery_configured(&state.config)?;

    if user_repo::find_by_username(state, &req.username)
        .await?
        .is_some()
    {
        tracing::warn!(username = %req.username, "register rejected: username already taken");
        return Err(AppError::Conflict("Username is already taken".to_string()));
    }
    if user_repo::find_by_email(state, &req.email)
        .await?
        .is_some()
    {
        tracing::warn!(email = %req.email, "register rejected: email already registered");
        return Err(AppError::Conflict(
            "Email is already registered".to_string(),
        ));
    }

    let role = resolve_registration_role(state, req.role.as_deref())?;
    let hashed = password::hash_password(&req.password)?;
    let user = user_repo::insert_user(state, &req.username, &req.email, &hashed, &req.name, &role)
        .await?;

    if let Err(error) = email_verification::send_for_user(state, &user).await {
        let _ = email_verification::discard_for_user(state, &user.id_string()).await;
        let _ = user_repo::discard_unverified_new_user(state, &user.id_string()).await;
        return Err(error);
    }
    crate::signup_event::emit(state.config.clone(), &user);
    let response = if state.config.email_verification_required || !issue_session {
        pending_verification_response(&user)
    } else {
        issue_auth_response(state, &user).await?
    };
    Ok((StatusCode::CREATED, Json(response)))
}

/// Resolve the role for a new account. The estate declares its roles in
/// ecompose.yml's `auth.roles` block; this binary never hardcodes one. A
/// requested role must be in the declared set (`ECO_AUTH_ROLES`) or the account
/// silently gets the declared default (`ECO_AUTH_DEFAULT_ROLE`). When no roles
/// are declared the request is trusted as-is (legacy behavior).
fn resolve_registration_role(
    state: &AppState,
    requested: Option<&str>,
) -> Result<String, AppError> {
    let requested = requested.map(str::trim).filter(|r| !r.is_empty());
    if state.config.allowed_roles.is_empty() {
        return Ok(requested.unwrap_or(&state.config.default_role).to_string());
    }
    match requested {
        Some(role) if state.config.allowed_roles.iter().any(|r| r == role) => Ok(role.to_string()),
        Some(role) => Err(AppError::BadRequest(format!(
            "Role \"{role}\" is not allowed on this estate. Allowed roles: {}",
            state.config.allowed_roles.join(", ")
        ))),
        None => Ok(state.config.default_role.clone()),
    }
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
    crate::signup_event::emit(state.config.clone(), &user);
    let response = if state.config.email_verification_required {
        pending_verification_response(&user)
    } else {
        issue_auth_response(&state, &user).await?
    };
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

/// Verifies the authenticated user's password without changing anything.
/// Used for sensitive confirmations (e.g. deleting a member account).
pub async fn verify_password(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<VerifyPasswordRequest>,
) -> AppResult<Json<VerifyPasswordResponse>> {
    let user = user_repo::find_by_id(&state, &auth.user_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {}", auth.user_id)))?;

    let valid = password::verify_password(&req.password, &user.password_hash);
    Ok(Json(VerifyPasswordResponse { valid }))
}

/// Starts the password-recovery flow. This response never says whether the
/// email exists; doing so would turn the endpoint into an account directory.
pub async fn forgot_password(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let email = req.email.trim();
    if !email.is_empty() {
        match user_repo::find_by_email(&state, email).await? {
            Some(user) => {
                if let Err(error) = crate::password_reset::issue(&state, &user).await {
                    tracing::error!(user_id = %user.id_string(), ?error, "password reset delivery failed");
                }
            }
            None => {
                // Pay the bcrypt cost even for an unknown address so the
                // observable timing stays closer to the known-account path.
                let _ = password::hash_password("password-reset-timing-padding");
            }
        }
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "accepted": true,
            "message": "Jika alamat email terdaftar, kami telah mengirim tautan untuk mengatur ulang password."
        })),
    ))
}

/// Completes a password recovery without requiring the old password. The
/// one-time token proves mailbox control; all prior sessions are revoked.
pub async fn reset_password(
    State(state): State<AppState>,
    Json(req): Json<PasswordResetConfirmRequest>,
) -> AppResult<StatusCode> {
    require_non_blank(&[("token", &req.token), ("newPassword", &req.new_password)])?;
    require_password_strength(&req.new_password)?;
    let user_id = crate::password_reset::consume(&state, req.token.trim()).await?;
    let user = user_repo::find_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("Tautan reset password tidak valid atau sudah kedaluwarsa.".into())
        })?;
    let hash = password::hash_password(&req.new_password)?;
    user_repo::update_password(&state, &user.id_string(), &hash).await?;
    crate::session_repo::revoke_all_for_user(&state, &user.id_string()).await?;
    tracing::info!(user_id = %user.id_string(), "password reset completed and sessions revoked");
    Ok(StatusCode::NO_CONTENT)
}

/// Starts the passwordless sign-in flow — the recovery path for a user locked
/// out of the single active session (e.g. the old device is lost or broken).
/// Like password recovery this never says whether the email exists; doing so
/// would turn the endpoint into an account directory.
pub async fn request_login_link(
    State(state): State<AppState>,
    Json(req): Json<LoginLinkRequest>,
) -> AppResult<(StatusCode, Json<serde_json::Value>)> {
    let email = req.email.trim();
    if !email.is_empty() {
        match user_repo::find_by_email(&state, email).await? {
            Some(user) => {
                if let Err(error) = crate::login_link::issue(&state, &user).await {
                    tracing::error!(user_id = %user.id_string(), ?error, "login link delivery failed");
                }
            }
            None => {
                // Pay the bcrypt cost even for an unknown address so the
                // observable timing stays closer to the known-account path.
                let _ = password::hash_password("login-link-timing-padding");
            }
        }
    }
    Ok((
        StatusCode::ACCEPTED,
        Json(serde_json::json!({
            "accepted": true,
            "message": "Jika alamat email terdaftar, kami telah mengirimkan tautan masuk."
        })),
    ))
}

/// Completes a passwordless sign-in. The one-time token proves mailbox
/// control; confirming it mints a fresh session (revoking every older one) so
/// the account can move to the current device immediately.
pub async fn confirm_login_link(
    State(state): State<AppState>,
    Json(req): Json<LoginLinkConfirmRequest>,
) -> AppResult<Json<AuthResponse>> {
    require_non_blank(&[("token", &req.token)])?;
    let user_id = crate::login_link::consume(&state, req.token.trim()).await?;
    let user = user_repo::find_by_id(&state, &user_id)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("Tautan masuk tidak valid atau sudah kedaluwarsa.".into())
        })?;

    // Proving mailbox control is itself proof the address is owned, so a
    // login-link sign-in clears the verification gate instead of dead-ending
    // an unverified account that had lost its only session.
    if state.config.email_verification_required && user.email_verified_at.is_none() {
        user_repo::mark_email_verified(&state, &user_id).await?;
    }

    tracing::info!(user_id = %user.id_string(), "login-link sign-in confirmed; old sessions revoked");
    Ok(Json(issue_auth_response(&state, &user).await?))
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

/// Internal identity lookup used by peer domains to hydrate a profile row the
/// first time they meet a userId, and to refresh identity fields live on every
/// profile read (auth is the only writer of these credential fields).
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

/// Same as `get_user_identity` but keyed by email, for the chat domain to
/// resolve invite-by-email to a user id without exposing any password data.
/// Auth owns the email → user mapping, so other domains never re-implement it.
pub async fn get_user_identity_by_email(
    State(state): State<AppState>,
    Path(email): Path<String>,
) -> AppResult<Json<UserDto>> {
    let email = email.trim().to_lowercase();
    let user = user_repo::find_by_email(&state, &email)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("User not found: {email}")))?;
    Ok(Json(UserDto::from(&user)))
}

async fn issue_auth_response(
    state: &AppState,
    user: &crate::models::user::User,
) -> AppResult<AuthResponse> {
    // Single active session per account: minting a new session revokes every
    // older one, so the same login cannot live on two devices at once.
    let session = crate::session_repo::create_session(state, &user.id_string(), None).await?;
    let token = jwt::generate_token(
        &state.config.jwt_secret,
        &user.id_string(),
        &user.username,
        &user.role,
        &user.effective_roles(),
        &session.id_string(),
        state.config.jwt_expiration_ms,
    )?;

    Ok(AuthResponse {
        token,
        user: UserDto::from(user),
        expires_in: state.config.jwt_expiration_ms / 1000,
        session_id: session.id_string(),
    })
}

/// Registration succeeds only after the verification email is accepted, but
/// it must never mint a usable session before the mailbox is proven. Keeping
/// the familiar response shape lets white-label Auth UI show a clear next step
/// without exposing a bearer token.
fn pending_verification_response(user: &crate::models::user::User) -> AuthResponse {
    AuthResponse {
        token: String::new(),
        user: UserDto::from(user),
        expires_in: 0,
        session_id: String::new(),
    }
}

/// Revoke the caller's current session — signing out the device that called.
pub async fn logout(State(state): State<AppState>, auth: AuthUser) -> AppResult<StatusCode> {
    if !auth.sid.is_empty() {
        crate::session_repo::revoke_session(&state, &auth.sid).await?;
    }
    tracing::info!(user_id = %auth.user_id, session_id = %auth.sid, "session revoked (logout)");
    Ok(StatusCode::NO_CONTENT)
}

/// Report whether the presented token's session is still the account's active
/// one. The estate gateway calls this per protected request to enforce
/// single-session at the edge; the frontend can poll it to notice when a newer
/// login superseded the current session.
pub async fn session_status(
    State(state): State<AppState>,
    auth: AuthUser,
) -> AppResult<Json<crate::dto::SessionStatus>> {
    let (active, session_id, expires_in_seconds) = if !auth.sid.is_empty() {
        let session = crate::session_repo::find_active_session(&state, &auth.sid).await?;
        match session {
            Some(session) => (
                true,
                session.id_string(),
                (session.expires_at.timestamp_millis() - bson::DateTime::now().timestamp_millis())
                    .max(0)
                    / 1000,
            ),
            None => (false, auth.sid, 0),
        }
    } else {
        (
            !state.config.session_required,
            String::new(),
            state.config.jwt_expiration_ms / 1000,
        )
    };
    Ok(Json(crate::dto::SessionStatus {
        session_id,
        active,
        expires_in_seconds,
        user_id: auth.user_id,
        username: auth.username,
        role: auth.role,
    }))
}
