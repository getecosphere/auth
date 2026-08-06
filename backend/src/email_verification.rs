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

fn rupiah(value: f64) -> String {
    let digits = (value.round() as i64).to_string();
    let mut grouped = String::new();
    for (index, character) in digits.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 { grouped.push('.'); }
        grouped.push(character);
    }
    format!("Rp {}", grouped.chars().rev().collect::<String>())
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
    // Astro emits this route as auth/verify-email/index.html. Keep the
    // trailing slash so a static production server resolves that directory
    // rather than falling back to the application's root page.
    let url = format!("{}/auth/verify-email/?token={}.{}", state.config.auth_public_url.trim_end_matches('/'), record.id, secret);
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

/// Product details used by the marketplace sale email template. Kept in a
/// struct so the handler stays readable as the template grows.
pub struct MarketplaceSaleEmailItem {
    pub id: String,
    pub title: String,
    pub description: String,
    pub category: String,
    pub condition: String,
    pub photos: Vec<String>,
    pub asking_price: f64,
}

/// Minimal HTML escaping so user-entered values (names, titles,
/// descriptions) can never break the email markup or smuggle links.
fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

fn sale_hero_image(site: &str, item: &MarketplaceSaleEmailItem, target_url: &str) -> String {
    match item.photos.first() {
        Some(key) if !key.is_empty() => format!(
            r#"<a href="{}" target="_blank"><img src="{}/api/storage/content/{}" alt="{}" width="600" style="display:block;width:100%;height:auto;border-radius:14px;border:1px solid #e2ece7;" /></a>"#,
            target_url,
            site,
            key,
            html_escape(&item.title)
        ),
        _ => r#"<table role="presentation" width="100%" cellpadding="0" cellspacing="0" style="background-color:#f2f6f4;border:1px solid #e2ece7;border-radius:14px;"><tr><td align="center" style="padding:32px;color:#a0bfb3;font-size:13px;">Foto produk</td></tr></table>"#
            .to_string(),
    }
}

fn sale_cta_button(href: &str, label: &str) -> String {
    format!(
        r#"<table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 auto;"><tr><td style="border-radius:999px;"><a href="{}" target="_blank" style="display:inline-block;padding:14px 30px;border-radius:999px;background-color:#059669;color:#ffffff;font-size:14px;font-weight:bold;text-decoration:none;">{}</a></td></tr></table>"#,
        href, label
    )
}

fn sale_detail_cell(label: &str, value: &str, highlight: bool) -> String {
    let text_color = if highlight { "#b84a22" } else { "#1b3029" };
    format!(
        r#"<td style="width:50%;padding:12px 16px;background-color:#faf6ec;border-radius:12px;vertical-align:top;"><p style="margin:0;font-size:10px;font-weight:bold;text-transform:uppercase;letter-spacing:1px;color:#6f9d8c;">{}</p><p style="margin:5px 0 0;font-size:14px;font-weight:bold;color:{};">{}</p></td>"#,
        label, text_color, value
    )
}

fn marketplace_sale_html(
    site: &str,
    user_name: &str,
    headline: &str,
    message: &str,
    image_html: &str,
    item: &MarketplaceSaleEmailItem,
    asking_price: &str,
    final_price: &str,
    cta: &str,
    seller_name: &str,
) -> String {
    let year = chrono::Utc::now().format("%Y");
    let description = if item.description.trim().is_empty() {
        String::new()
    } else {
        format!(
            r#"<p style="margin:0 0 4px;font-size:13px;line-height:1.6;color:#4d806d;">{}</p>"#,
            html_escape(&item.description)
        )
    };
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
              <td align="right" style="color:#eb9e77;font-size:11px;font-weight:bold;text-transform:uppercase;letter-spacing:1.5px;">Marketplace</td>
            </tr></table>
          </td>
        </tr>
        <tr>
          <td style="background-color:#ffffff;border:1px solid #e2ece7;border-top:none;border-radius:0 0 16px 16px;padding:30px 32px;">
            <p style="margin:0 0 6px;font-size:14px;color:#1b3029;">Halo {user_name},</p>
            <h1 style="margin:0 0 12px;font-size:22px;line-height:1.3;color:#1b3029;">{headline}</h1>
            <p style="margin:0 0 22px;font-size:14px;line-height:1.7;color:#4d806d;">{message}</p>
            {image_html}
            <p style="margin:18px 0 10px;font-size:18px;font-weight:bold;color:#1b3029;">{title}</p>
            {description}
            <p style="margin:4px 0 18px;font-size:12px;color:#6f9d8c;">Pembeli: {seller_name} · {category} · {condition}</p>
            <table role="presentation" width="100%" cellpadding="0" cellspacing="6" style="margin:0 0 24px;">
              <tr>
                {asking_cell}
                {final_cell}
              </tr>
            </table>
            <table role="presentation" cellpadding="0" cellspacing="0" style="margin:0 0 6px;"><tr><td>{cta}</td></tr></table>
          </td>
        </tr>
        <tr>
          <td style="background-color:#1b3029;border-radius:0 0 16px 16px;padding:22px 32px;text-align:center;">
            <p style="margin:0;color:#faf6ec;font-size:13px;font-weight:bold;letter-spacing:0.5px;">Stuff8</p>
            <p style="margin:6px 0 0;color:#a0bfb3;font-size:12px;">Know what you own.</p>
            <p style="margin:14px 0 0;color:#6f9d8c;font-size:11px;">
              <a href="{site}/marketplace/" style="color:#a0bfb3;text-decoration:underline;">Marketplace</a>
              &nbsp;·&nbsp;
              <a href="{site}/inventory/" style="color:#a0bfb3;text-decoration:underline;">Inventaris</a>
            </p>
            <p style="margin:10px 0 0;color:#6f9d8c;font-size:11px;">&copy; {year} Stuff8</p>
          </td>
        </tr>
      </table>
    </td></tr>
  </table>
</body>
</html>"#,
        site = site,
        user_name = html_escape(user_name),
        headline = html_escape(headline),
        message = message,
        image_html = image_html,
        title = html_escape(&item.title),
        description = description,
        seller_name = html_escape(seller_name),
        category = html_escape(&item.category),
        condition = html_escape(&item.condition),
        asking_cell = sale_detail_cell("Harga buka", asking_price, false),
        final_cell = sale_detail_cell("Harga terjual", final_price, true),
        cta = cta,
        year = year,
    )
}

/// Sends a transactional marketplace outcome email. Unlike account
/// verification, this is best-effort: a completed sale must never be rolled
/// back merely because a mail provider is temporarily unavailable.
pub async fn send_marketplace_sale_notice(
    state: &AppState,
    user: &User,
    item: &MarketplaceSaleEmailItem,
    buyer_name: &str,
    final_price: f64,
    is_buyer: bool,
    is_seller: bool,
) -> AppResult<Option<String>> {
    if state.config.brevo_api_key.is_empty() || state.config.mail_from_email.is_empty() {
        tracing::warn!(user_id = %user.id_string(), "Marketplace sale email skipped: mail delivery is not configured");
        return Ok(None);
    }

    let site = state.config.public_site_url.trim_end_matches('/').to_string();
    let final_price = if final_price.is_finite() { final_price } else { 0.0 };
    let formatted_price = rupiah(final_price);
    let formatted_asking = rupiah(if item.asking_price.is_finite() { item.asking_price } else { 0.0 });

    let (subject, headline, message, product_url, cta) = if is_buyer {
        (
            format!("{} kini ada di inventarismu", item.title),
            format!("Selamat! {} sudah jadi milikmu", item.title),
            format!(
                "Barang <strong>{}</strong> telah terjual kepadamu seharga <strong>{}</strong> dan kini sudah masuk ke inventaris pribadimu di Stuff8. Semoga cocok, ya.",
                html_escape(&item.title), formatted_price
            ),
            format!("{site}/inventory/detail/?id={}", item.id),
            sale_cta_button(&format!("{site}/inventory/detail/?id={}", item.id), "Lihat di Inventarismu"),
        )
    } else if is_seller {
        (
            format!("Barangmu {} sudah terjual", item.title),
            format!("Barangmu {} sudah terjual", item.title),
            format!(
                "Barang <strong>{}</strong> telah terjual kepada <strong>{}</strong> seharga <strong>{}</strong>. Terima kasih sudah bertransaksi dengan aman dan hangat di Stuff8.",
                html_escape(&item.title), html_escape(buyer_name), formatted_price
            ),
            format!("{site}/marketplace/detail/?id={}", item.id),
            sale_cta_button(&format!("{site}/marketplace/"), "Jual barang berikutnya"),
        )
    } else {
        (
            format!("Update negosiasi {}", item.title),
            format!("{} sudah terjual", item.title),
            format!(
                "Terima kasih sudah menawar <strong>{}</strong>. Barang ini sudah terjual kepada <strong>{}</strong> sehingga negosiasinya ditutup. Masih banyak barang menarik lain yang menantimu di Marketplace Stuff8.",
                html_escape(&item.title), html_escape(buyer_name)
            ),
            format!("{site}/marketplace/detail/?id={}", item.id),
            sale_cta_button(&format!("{site}/marketplace/"), "Jelajahi Marketplace"),
        )
    };

    let image_html = sale_hero_image(&site, item, &product_url);
    let body = marketplace_sale_html(
        &site,
        &user.name,
        &headline,
        &message,
        &image_html,
        item,
        &formatted_asking,
        &formatted_price,
        &cta,
        buyer_name,
    );

    let payload = serde_json::json!({
        "sender": { "email": state.config.mail_from_email, "name": state.config.mail_from_name },
        "to": [{ "email": user.email, "name": user.name }],
        "subject": subject,
        "htmlContent": body,
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
        tracing::error!(%status, response = %response_body, user_id = %user.id_string(), "Brevo rejected marketplace sale email");
        return Err(AppError::Internal(anyhow::anyhow!("Brevo rejected marketplace sale email: {status}")));
    }
    let message_id = serde_json::from_str::<serde_json::Value>(&response_body)
        .ok()
        .and_then(|value| value.get("messageId").and_then(|id| id.as_str()).map(str::to_owned));
    tracing::info!(
        user_id = %user.id_string(),
        email = %user.email,
        is_buyer,
        is_seller,
        message_id = message_id.as_deref().unwrap_or("unavailable"),
        "Brevo accepted marketplace sale email"
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
