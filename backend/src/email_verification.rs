use bson::{doc, DateTime};
use mongodb::Collection;
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{user::User, verification::EmailVerification},
    password,
    state::AppState,
};

fn verifications(state: &AppState) -> Collection<EmailVerification> {
    state.db.collection("email_verifications")
}

pub async fn send_for_user(state: &AppState, user: &User) -> AppResult<()> {
    if !state.config.email_verification_required || user.email_verified_at.is_some() {
        return Ok(());
    }
    if state.config.brevo_api_key.is_empty() || state.config.mail_from_email.is_empty() || state.config.auth_public_url.is_empty() {
        return Err(AppError::Internal(anyhow::anyhow!("Email verification is enabled but Brevo is not configured")));
    }
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
    if !response.status().is_success() {
        return Err(AppError::Internal(anyhow::anyhow!("Brevo rejected verification email: {}", response.status())));
    }
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
