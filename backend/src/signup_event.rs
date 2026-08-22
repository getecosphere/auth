//! Outbound domain event: "a user signed up".
//!
//! Auth publishes `user.signed_up` to an *opaque, composer-configured sink*
//! (`SIGNUP_EVENT_URL` + optional `SIGNUP_EVENT_TOKEN`). This is a pure
//! outbox-style emit: auth never interprets the sink, never knows who consumes
//! the event, and never couples to a consumer domain (notifications, CRM, …).
//! The estate decides what to point the URL at and which token to authorize
//! the webhook with. Emission is fire-and-forget — a slow or failing sink must
//! never fail the register response.

use std::time::Duration;

use serde::Serialize;

use crate::{config::AppConfig, jwt, models::user::User};

const EMIT_TIMEOUT: Duration = Duration::from_secs(5);

/// The identity used when auth mints its own sink token (no
/// `SIGNUP_EVENT_TOKEN` configured). A synthetic `sub` so the sink's
/// per-user channels are never confused with a real account.
const SERVICE_SUB: &str = "system.signup-bridge";

#[derive(Serialize)]
#[allow(non_snake_case)]
struct SignupEvent<'a> {
    event: &'a str,
    userId: &'a str,
    username: &'a str,
    email: &'a str,
    name: &'a str,
    role: &'a str,
    at: String,
}

/// Fire-and-forget publish of `user.signed_up`. Never fails the caller.
pub fn emit(config: AppConfig, user: &User) {
    let Some(url) = config.signup_event_url.clone() else {
        return;
    };
    // Preferred: the composer-supplied token. Fallback: a short-lived JWT auth
    // mints itself from the estate-shared JWT_SECRET, so any sink that
    // validates estate tokens (like the notifications LXS) accepts the event.
    let token = config
        .signup_event_token
        .clone()
        .filter(|t| !t.is_empty())
        .or_else(|| {
            // Service token: no session binding (empty sid). Sinks that
            // validate estate tokens accept it; single-session checks only
            // apply to user bearer tokens.
            jwt::generate_token(
                &config.jwt_secret,
                SERVICE_SUB,
                "system",
                "service",
                "",
                60_000,
            )
            .ok()
        });
    let user_id = user.id_string();
    let username = user.username.clone();
    let email = user.email.clone();
    let name = user.name.clone();
    let role = user.role.clone();
    let at = user.created_at.to_chrono().to_rfc3339();

    tokio::spawn(async move {
        let event = SignupEvent {
            event: "user.signed_up",
            userId: &user_id,
            username: &username,
            email: &email,
            name: &name,
            role: &role,
            at,
        };
        let result = tokio::time::timeout(EMIT_TIMEOUT, post_event(&url, &token, &event)).await;
        match result {
            Ok(Ok(_)) => tracing::debug!(url = %url, user = %user_id, "signup event delivered"),
            Ok(Err(error)) => {
                tracing::warn!(url = %url, user = %user_id, %error, "signup event failed")
            }
            Err(_) => tracing::warn!(url = %url, user = %user_id, "signup event timed out"),
        }
    });
}

async fn post_event(
    url: &str,
    token: &Option<String>,
    event: &SignupEvent<'_>,
) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(EMIT_TIMEOUT)
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client.post(url).json(event);
    if let Some(token) = token {
        if !token.is_empty() {
            request = request.bearer_auth(token);
        }
    }
    let response = request.send().await.map_err(|e| e.to_string())?;
    if !response.status().is_success() {
        return Err(format!("sink returned {}", response.status()));
    }
    Ok(())
}
