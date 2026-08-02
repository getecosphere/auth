use bson::{doc, DateTime};
use mongodb::Collection;
use uuid::Uuid;

use crate::{
    config::AppConfig,
    error::{AppError, AppResult},
    models::{user::User, verification::EmailVerification},
    password,
    state::AppState,
};

fn verifications(state: &AppState) -> Collection<EmailVerification> {
    state.db.collection("email_verifications")
}

/// Fail before a user is created when this installation requires email
/// verification but cannot deliver a verification link. This keeps a typo in
/// local/prod mail configuration from leaving behind an unusable account.
pub fn ensure_delivery_configured(config: &AppConfig) -> AppResult<()> {
    if config.email_verification_required
        && (config.brevo_api_key.is_empty()
            || config.mail_from_email.is_empty()
            || config.auth_public_url.is_empty())
    {
        return Err(AppError::BadRequest(
            "Pendaftaran belum tersedia karena layanan verifikasi email belum dikonfigurasi.".into(),
        ));
    }
    Ok(())
}

/// Sends a verification request to Brevo and returns Brevo's message id when
/// available. A successful API response means Brevo accepted the message for
/// delivery; it deliberately does not claim inbox delivery.
pub async fn send_for_user(state: &AppState, user: &User) -> AppResult<Option<String>> {
    if !state.config.email_verification_required || user.email_verified_at.is_some() {
        return Ok(None);
    }
    ensure_delivery_configured(&state.config)?;
    let secret = Uuid::new_v4().to_string();
    let record = EmailVerification {
        id: Uuid::new_v4().to_string(), user_id: user.id_string(),
        token_hash: password::hash_password(&secret)?,
        expires_at: DateTime::from_chrono(chrono::Utc::now() + chrono::Duration::hours(state.config.email_verification_ttl_hours)),
        used_at: None, created_at: DateTime::now(),
    };
    verifications(state).update_many(doc! { "userId": &record.user_id, "usedAt": null }, doc! { "$set": { "usedAt": DateTime::now() } }, None).await?;
    verifications(state).insert_one(&record, None).await?;
    let url = format!("{}/auth/verify-email?token={}.{}", state.config.auth_public_url.trim_end_matches('/'), record.id, secret);
    let body = serde_json::json!({
        "sender": { "email": state.config.mail_from_email, "name": state.config.mail_from_name },
        "to": [{ "email": user.email, "name": user.name }],
        "subject": "Verifikasi email akun Anda",
        "htmlContent": format!("<p>Halo {},</p><p>Verifikasi email akun Anda dengan membuka tautan berikut:</p><p><a href=\"{}\">Verifikasi email</a></p><p>Tautan ini berlaku {} jam dan hanya dapat digunakan sekali.</p>", user.name, url, state.config.email_verification_ttl_hours),
    });
    let response = reqwest::Client::new().post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", &state.config.brevo_api_key).json(&body).send().await
        .map_err(|e| AppError::Internal(e.into()))?;
    let status = response.status();
    let response_body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        tracing::error!(%status, response = %response_body, "Brevo rejected verification email");
        return Err(AppError::Internal(anyhow::anyhow!("Brevo rejected verification email: {status}")));
    }
    let message_id = serde_json::from_str::<serde_json::Value>(&response_body)
        .ok()
        .and_then(|value| value.get("messageId").and_then(|id| id.as_str()).map(str::to_owned));
    tracing::info!(
        user_id = %user.id_string(),
        email = %user.email,
        message_id = message_id.as_deref().unwrap_or("unavailable"),
        "Brevo accepted verification email"
    );
    Ok(message_id)
}

pub async fn discard_for_user(state: &AppState, user_id: &str) -> AppResult<()> {
    verifications(state)
        .delete_many(doc! { "userId": user_id }, None)
        .await?;
    Ok(())
}

pub async fn verify(state: &AppState, token: &str) -> AppResult<()> {
    let Some((id, secret)) = token.split_once('.') else { return Err(AppError::BadRequest("Invalid verification link".into())); };
    let record = verifications(state).find_one(doc! { "id": id, "usedAt": null }, None).await?
        .ok_or_else(|| AppError::BadRequest("This verification link is invalid or was already used".into()))?;
    if record.expires_at.to_chrono() < chrono::Utc::now() || !password::verify_password(secret, &record.token_hash) {
        return Err(AppError::BadRequest("This verification link has expired. Request a new email.".into()));
    }
    crate::user_repo::mark_email_verified(state, &record.user_id).await?;
    verifications(state).update_one(doc! { "id": id }, doc! { "$set": { "usedAt": DateTime::now() } }, None).await?;
    Ok(())
}
