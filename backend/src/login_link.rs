use bson::{doc, DateTime};
use mongodb::{options::IndexOptions, Collection, IndexModel};
use uuid::Uuid;

use crate::{
    error::{AppError, AppResult},
    models::{login_link::LoginLink, user::User},
    password,
    state::AppState,
};

fn links(state: &AppState) -> Collection<LoginLink> {
    state.db.collection("login_links")
}

/// The TTL index removes expired records eventually; expiry is also checked
/// explicitly during confirmation so MongoDB's background TTL cadence never
/// extends a link's validity.
pub async fn ensure_indexes(state: &AppState) -> AppResult<()> {
    let collection = links(state);
    collection
        .create_index(IndexModel::builder().keys(doc! { "id": 1 }).build(), None)
        .await?;
    collection
        .create_index(
            IndexModel::builder()
                .keys(doc! { "expiresAt": 1 })
                .options(
                    IndexOptions::builder()
                        .expire_after(Some(std::time::Duration::from_secs(0)))
                        .build(),
                )
                .build(),
            None,
        )
        .await?;
    Ok(())
}

/// Creates a new passwordless sign-in link and invalidates any older unused
/// link for this account. Delivery is handled by Auth because it owns account
/// email and the mail delivery configuration.
pub async fn issue(state: &AppState, user: &User) -> AppResult<()> {
    let estate_mail =
        !state.config.brevo_api_key.is_empty() && !state.config.mail_from_email.is_empty();
    let platform_relay =
        !state.config.email_relay_url.is_empty() && !state.config.email_relay_token.is_empty();
    if (!estate_mail && !platform_relay) || state.config.auth_public_url.trim().is_empty() {
        tracing::warn!(user_id = %user.id_string(), "login link requested but email delivery is not configured");
        return Ok(());
    }

    let secret = Uuid::new_v4().to_string();
    let record = LoginLink {
        id: Uuid::new_v4().to_string(),
        user_id: user.id_string(),
        token_hash: password::hash_password(&secret)?,
        expires_at: DateTime::from_chrono(
            chrono::Utc::now() + chrono::Duration::minutes(state.config.login_link_ttl_minutes),
        ),
        used_at: None,
        created_at: DateTime::now(),
    };
    links(state)
        .update_many(
            doc! { "userId": &record.user_id, "usedAt": null },
            doc! { "$set": { "usedAt": DateTime::now() } },
            None,
        )
        .await?;
    links(state).insert_one(&record, None).await?;

    let url = format!(
        "{}/login-link?token={}.{}",
        state.config.auth_public_url.trim_end_matches('/'),
        record.id,
        secret
    );
    crate::email_verification::send_login_link_email(
        state,
        user,
        &url,
        state.config.login_link_ttl_minutes,
    )
    .await?;
    Ok(())
}

/// Verifies and consumes a login link token, returning the owning user id.
/// The caller mints the new session (which revokes every older one), so a
/// locked-out device can sign back in the moment the mailbox is proven.
pub async fn consume(state: &AppState, token: &str) -> AppResult<String> {
    let Some((id, secret)) = token.split_once('.') else {
        return Err(AppError::BadRequest(
            "Tautan masuk tidak valid atau sudah kedaluwarsa.".into(),
        ));
    };
    let record = links(state)
        .find_one(doc! { "id": id, "usedAt": null }, None)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest("Tautan masuk tidak valid atau sudah kedaluwarsa.".into())
        })?;
    if record.expires_at.to_chrono() < chrono::Utc::now()
        || !password::verify_password(secret, &record.token_hash)
    {
        return Err(AppError::BadRequest(
            "Tautan masuk tidak valid atau sudah kedaluwarsa.".into(),
        ));
    }
    let result = links(state)
        .update_one(
            doc! { "id": id, "usedAt": null },
            doc! { "$set": { "usedAt": DateTime::now() } },
            None,
        )
        .await?;
    if result.modified_count != 1 {
        return Err(AppError::BadRequest(
            "Tautan masuk tidak valid atau sudah digunakan.".into(),
        ));
    }
    Ok(record.user_id)
}
