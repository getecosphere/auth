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

/// Minimal HTML escaping so user-entered values can never break the email
/// markup or smuggle links.
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

/// Warm, corporate-but-cozy Stuff8 email layout shared by Auth's own
/// transactional mail: cream paper background, navy header/footer, brand-green
/// CTA. Inline styles only so it renders in mail clients that strip <style>.
fn stuff8_email_shell(headline: &str, message_html: &str, cta_html: &str) -> String {
    let year = chrono::Utc::now().format("%Y");
    format!(
        r#"<!DOCTYPE html>
<html lang="id">
<head>
  <meta charset="utf-8" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <title>{headline}</title>
</head>
<body style="margin:0;padding:0;background-color:#faf6ec;">
  <table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#faf6ec;padding:24px 12px;">
    <tr><td align="center">
      <table role="presentation" width="600" cellpadding="0" cellspacing="0" style="width:100%;max-width:600px;font-family:Arial,Helvetica,sans-serif;color:#1b3029;">
        <tr>
          <td style="background-color:#0f1e1a;border-radius:16px 16px 0 0;padding:26px 32px;">
            <table role="presentation" width="100%" cellpadding="0" cellspacing="0"><tr>
              <td align="left" style="color:#faf6ec;font-size:22px;font-weight:bold;letter-spacing:0.5px;">Stuff8</td>
              <td align="right" style="color:#eb9e77;font-size:11px;font-weight:bold;text-transform:uppercase;letter-spacing:1.5px;">Akun</td>
            </tr></table>
          </td>
        </tr>
        <tr>
          <td style="background-color:#ffffff;border:1px solid #e2ece7;border-top:none;border-radius:0 0 16px 16px;padding:30px 32px;">
            <h1 style="margin:0 0 12px;font-size:22px;line-height:1.3;color:#1b3029;">{headline}</h1>
            <p style="margin:0 0 22px;font-size:14px;line-height:1.7;color:#4d806d;">{message_html}</p>
            <table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 0 22px;"><tr><td>{cta_html}</td></tr></table>
          </td>
        </tr>
        <tr>
          <td style="background-color:#1b3029;border-radius:0 0 16px 16px;padding:22px 32px;text-align:center;">
            <p style="margin:0;color:#faf6ec;font-size:13px;font-weight:bold;letter-spacing:0.5px;">Stuff8</p>
            <p style="margin:6px 0 0;color:#a0bfb3;font-size:12px;">Know what you own.</p>
            <p style="margin:10px 0 0;color:#6f9d8c;font-size:11px;">&copy; {year} Stuff8</p>
          </td>
        </tr>
      </table>
    </td></tr>
  </table>
</body>
</html>"#,
        headline = html_escape(headline),
        message_html = message_html,
        cta_html = cta_html,
        year = year,
    )
}

/// Brand-green pill CTA. A bare text fallback line sits right under it so the
/// action is still reachable when a client strips link styling.
fn stuff8_cta_button(href: &str, label: &str, fallback: &str) -> String {
    format!(
        r#"<table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 auto;"><tr><td style="border-radius:999px;"><a href="{}" target="_blank" style="display:inline-block;padding:14px 30px;border-radius:999px;background-color:#059669;color:#ffffff;font-size:14px;font-weight:bold;text-decoration:none;">{}</a></td></tr></table><p style="margin:14px 0 0;font-size:12px;line-height:1.6;color:#6f9d8c;word-break:break-all;">Jika tombol di atas tidak muncul, buka tautan ini: <a href="{}" style="color:#059669;text-decoration:underline;">{}</a></p>"#,
        href, label, href, fallback
    )
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
            "Pendaftaran belum tersedia karena layanan verifikasi email belum dikonfigurasi."
                .into(),
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
        id: Uuid::new_v4().to_string(),
        user_id: user.id_string(),
        token_hash: password::hash_password(&secret)?,
        expires_at: DateTime::from_chrono(
            chrono::Utc::now() + chrono::Duration::hours(state.config.email_verification_ttl_hours),
        ),
        used_at: None,
        created_at: DateTime::now(),
    };
    verifications(state)
        .update_many(
            doc! { "userId": &record.user_id, "usedAt": null },
            doc! { "$set": { "usedAt": DateTime::now() } },
            None,
        )
        .await?;
    verifications(state).insert_one(&record, None).await?;
    // Astro emits this route as auth/verify-email/index.html. Keep the
    // trailing slash so a static production server resolves that directory
    // rather than falling back to the application's root page.
    let url = format!(
        "{}/auth/verify-email/?token={}.{}",
        state.config.auth_public_url.trim_end_matches('/'),
        record.id,
        secret
    );
    let headline = "Verifikasi email akunmu";
    let message = format!(
        r#"Halo <strong>{}</strong>,<br /><br />Satu langkah lagi — konfirmasi alamat email ini untuk mengamankan akun Stuff8-mu. Tombol di bawah berlaku <strong>{} jam</strong> dan hanya bisa digunakan sekali."#,
        html_escape(&user.name),
        state.config.email_verification_ttl_hours
    );
    let body = serde_json::json!({
        "sender": { "email": state.config.mail_from_email, "name": state.config.mail_from_name },
        "to": [{ "email": user.email, "name": user.name }],
        "subject": headline,
        "htmlContent": stuff8_email_shell(
            headline,
            &message,
            &stuff8_cta_button(&url, "Verifikasi email", &url),
        ),
    });
    let response = reqwest::Client::new()
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", &state.config.brevo_api_key)
        .json(&body)
        .send()
        .await
        .map_err(|e| AppError::Internal(e.into()))?;
    let status = response.status();
    let response_body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        tracing::error!(%status, response = %response_body, "Brevo rejected verification email");
        return Err(AppError::Internal(anyhow::anyhow!(
            "Brevo rejected verification email: {status}"
        )));
    }
    let message_id = serde_json::from_str::<serde_json::Value>(&response_body)
        .ok()
        .and_then(|value| {
            value
                .get("messageId")
                .and_then(|id| id.as_str())
                .map(str::to_owned)
        });
    tracing::info!(
        user_id = %user.id_string(),
        email = %user.email,
        message_id = message_id.as_deref().unwrap_or("unavailable"),
        "Brevo accepted verification email"
    );
    Ok(message_id)
}

/// Sends a fully-rendered transactional email on behalf of another domain.
/// Auth owns the mail provider credentials and recipient identity; the caller
/// owns the subject/html (its bounded context). Content is opaque here on
/// purpose — no business template or domain data ever lives in Auth. Delivery
/// is best-effort so a transient provider failure never rolls back the
/// caller's own transactional state change.
pub async fn send_transactional_mail(
    state: &AppState,
    user: &User,
    subject: &str,
    html: &str,
) -> AppResult<Option<String>> {
    if state.config.brevo_api_key.is_empty() || state.config.mail_from_email.is_empty() {
        tracing::warn!(user_id = %user.id_string(), "Transactional email skipped: mail delivery is not configured");
        return Ok(None);
    }

    let payload = serde_json::json!({
        "sender": { "email": state.config.mail_from_email, "name": state.config.mail_from_name },
        "to": [{ "email": user.email, "name": user.name }],
        "subject": subject,
        "htmlContent": html,
    });
    let response = reqwest::Client::new()
        .post("https://api.brevo.com/v3/smtp/email")
        .header("api-key", &state.config.brevo_api_key)
        .json(&payload)
        .send()
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    let status = response.status();
    let response_body = response.text().await.unwrap_or_default();
    if !status.is_success() {
        tracing::error!(%status, response = %response_body, user_id = %user.id_string(), "Brevo rejected transactional email");
        return Err(AppError::Internal(anyhow::anyhow!(
            "Brevo rejected transactional email: {status}"
        )));
    }
    let message_id = serde_json::from_str::<serde_json::Value>(&response_body)
        .ok()
        .and_then(|value| {
            value
                .get("messageId")
                .and_then(|id| id.as_str())
                .map(str::to_owned)
        });
    tracing::info!(
        user_id = %user.id_string(),
        email = %user.email,
        message_id = message_id.as_deref().unwrap_or("unavailable"),
        "Brevo accepted transactional email"
    );
    Ok(message_id)
}

/// Password reset is an Auth-owned email because it contains a credential
/// recovery link. Other domains only use `send_transactional_mail` with their
/// own opaque content.
pub async fn send_password_reset_email(
    state: &AppState,
    user: &User,
    url: &str,
    ttl_minutes: i64,
) -> AppResult<Option<String>> {
    // An estate's own provider configuration always wins. The platform relay
    // is intentionally a recovery-only fallback for a temporary `eco serve`
    // lease, never a general outbound-mail API.
    if !state.config.brevo_api_key.is_empty() && !state.config.mail_from_email.is_empty() {
        let headline = "Atur ulang password akunmu";
        let message = format!(
            "Halo <strong>{}</strong>,<br /><br />Kami menerima permintaan untuk mengatur ulang password akunmu. Tautan ini berlaku <strong>{} menit</strong> dan hanya dapat digunakan sekali. Jika kamu tidak memintanya, abaikan email ini.",
            html_escape(&user.name),
            ttl_minutes,
        );
        let html = stuff8_email_shell(
            headline,
            &message,
            &stuff8_cta_button(url, "Atur ulang password", url),
        );
        return send_transactional_mail(state, user, headline, &html).await;
    }

    if state.config.email_relay_url.trim().is_empty() || state.config.email_relay_token.is_empty() {
        tracing::warn!(user_id = %user.id_string(), "password reset requested but no direct email provider or platform relay is configured");
        return Ok(None);
    }
    let response = reqwest::Client::new()
        .post(state.config.email_relay_url.trim())
        .bearer_auth(&state.config.email_relay_token)
        .json(&serde_json::json!({
            "to": &user.email,
            "name": &user.name,
            "reset_url": url,
            "ttl_minutes": ttl_minutes,
        }))
        .send()
        .await
        .map_err(|error| AppError::Internal(error.into()))?;
    if !response.status().is_success() {
        let status = response.status();
        tracing::error!(%status, user_id = %user.id_string(), "platform password recovery relay rejected email");
        return Err(AppError::Internal(anyhow::anyhow!(
            "platform password recovery relay rejected email: {status}"
        )));
    }
    tracing::info!(user_id = %user.id_string(), "platform password recovery relay accepted email");
    Ok(None)
}

pub async fn discard_for_user(state: &AppState, user_id: &str) -> AppResult<()> {
    verifications(state)
        .delete_many(doc! { "userId": user_id }, None)
        .await?;
    Ok(())
}

pub async fn verify(state: &AppState, token: &str) -> AppResult<()> {
    let Some((id, secret)) = token.split_once('.') else {
        return Err(AppError::BadRequest("Invalid verification link".into()));
    };
    let record = verifications(state)
        .find_one(doc! { "id": id, "usedAt": null }, None)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("This verification link is invalid or was already used".into())
        })?;
    if record.expires_at.to_chrono() < chrono::Utc::now()
        || !password::verify_password(secret, &record.token_hash)
    {
        return Err(AppError::BadRequest(
            "This verification link has expired. Request a new email.".into(),
        ));
    }
    crate::user_repo::mark_email_verified(state, &record.user_id).await?;
    verifications(state)
        .update_one(
            doc! { "id": id },
            doc! { "$set": { "usedAt": DateTime::now() } },
            None,
        )
        .await?;
    Ok(())
}
