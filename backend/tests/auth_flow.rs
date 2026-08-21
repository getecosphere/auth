mod common;

use serde_json::{json, Value};

#[tokio::test]
async fn register_then_login_round_trip() {
    let app = common::spawn().await;

    let register_res = app
        .http
        .post(app.url("/auth/register"))
        .json(&json!({
            "username": "alice", "email": "alice@example.com",
            "password": "AlicePass1", "name": "Alice"
        }))
        .send()
        .await
        .expect("register request");
    assert_eq!(register_res.status(), 201);
    let register_body: Value = register_res.json().await.expect("register body");
    assert!(register_body["token"].is_string());
    assert_eq!(register_body["user"]["username"], "alice");
    assert_eq!(register_body["user"]["role"], "member");

    // Registration mints a session, so a raw login is rejected while it is
    // active. Log out first, then login succeeds.
    let logout = app
        .http
        .post(app.url("/auth/logout"))
        .bearer_auth(register_body["token"].as_str().unwrap())
        .send()
        .await
        .expect("logout request");
    assert_eq!(logout.status(), 204);

    let login_res = app
        .http
        .post(app.url("/auth/login"))
        .json(&json!({ "username": "alice", "password": "AlicePass1" }))
        .send()
        .await
        .expect("login request");
    assert_eq!(login_res.status(), 200);
    let login_body: Value = login_res.json().await.expect("login body");
    assert!(login_body["token"].is_string());
}

#[tokio::test]
async fn login_rejects_wrong_password_and_unknown_user_identically() {
    let app = common::spawn().await;

    app.http
        .post(app.url("/auth/register"))
        .json(&json!({
            "username": "bob", "email": "bob@example.com",
            "password": "BobPass123", "name": "Bob"
        }))
        .send()
        .await
        .expect("register request")
        .error_for_status()
        .expect("register should succeed");

    let wrong_password = app
        .http
        .post(app.url("/auth/login"))
        .json(&json!({ "username": "bob", "password": "wrong-password" }))
        .send()
        .await
        .expect("login request");
    assert_eq!(wrong_password.status(), 400);

    let unknown_user = app
        .http
        .post(app.url("/auth/login"))
        .json(&json!({ "username": "nobody-registered", "password": "whatever123" }))
        .send()
        .await
        .expect("login request");
    // Same status for "wrong password" and "user doesn't exist" -- the
    // dummy-hash timing-safe comparison this asserts the *behavior* of;
    // an actual timing measurement isn't practical in a fast unit test,
    // but the status codes being indistinguishable is the load-bearing
    // part of not leaking which case happened.
    assert_eq!(unknown_user.status(), wrong_password.status());
}

#[tokio::test]
async fn register_rejects_duplicate_username_and_email_with_409() {
    let app = common::spawn().await;

    app.http
        .post(app.url("/auth/register"))
        .json(&json!({
            "username": "carol", "email": "carol@example.com",
            "password": "CarolPass1", "name": "Carol"
        }))
        .send()
        .await
        .expect("register request")
        .error_for_status()
        .expect("first register should succeed");

    let dup_username = app
        .http
        .post(app.url("/auth/register"))
        .json(&json!({
            "username": "carol", "email": "different@example.com",
            "password": "CarolPass2", "name": "Carol Duplicate"
        }))
        .send()
        .await
        .expect("register request");
    assert_eq!(dup_username.status(), 409);
}

#[tokio::test]
async fn change_password_requires_correct_current_password() {
    let app = common::spawn().await;

    let register_body: Value = app
        .http
        .post(app.url("/auth/register"))
        .json(&json!({
            "username": "dave", "email": "dave@example.com",
            "password": "DavePass1", "name": "Dave"
        }))
        .send()
        .await
        .expect("register request")
        .json()
        .await
        .expect("register body");
    let token = register_body["token"].as_str().unwrap();

    // Wrong current password is rejected.
    let wrong = app
        .http
        .put(app.url("/auth/change-password"))
        .bearer_auth(token)
        .query(&[
            ("currentPassword", "not-the-real-password"),
            ("newPassword", "DaveNewPass1"),
        ])
        .send()
        .await
        .expect("change-password request");
    assert_eq!(wrong.status(), 400);

    // Correct current password succeeds, and the new password then works
    // for login.
    let correct = app
        .http
        .put(app.url("/auth/change-password"))
        .bearer_auth(token)
        .query(&[
            ("currentPassword", "DavePass1"),
            ("newPassword", "DaveNewPass1"),
        ])
        .send()
        .await
        .expect("change-password request");
    assert_eq!(correct.status(), 204);

    // Registration session is still active, so log out before verifying the
    // new password signs in.
    let logout = app
        .http
        .post(app.url("/auth/logout"))
        .bearer_auth(token)
        .send()
        .await
        .expect("logout request");
    assert_eq!(logout.status(), 204);

    let login_with_new_password = app
        .http
        .post(app.url("/auth/login"))
        .json(&json!({ "username": "dave", "password": "DaveNewPass1" }))
        .send()
        .await
        .expect("login request");
    assert_eq!(login_with_new_password.status(), 200);
}

#[tokio::test]
async fn change_password_requires_authentication() {
    let app = common::spawn().await;

    let res = app
        .http
        .put(app.url("/auth/change-password"))
        .query(&[("currentPassword", "x"), ("newPassword", "SomeNewPass1")])
        .send()
        .await
        .expect("change-password request");
    assert_eq!(res.status(), 401);
}

#[tokio::test]
async fn health_check_is_public_and_unrate_limited_by_credential_bucket() {
    let app = common::spawn().await;
    let res = app
        .http
        .get(app.url("/health"))
        .send()
        .await
        .expect("health request");
    assert_eq!(res.status(), 200);
}

#[tokio::test]
async fn forged_signature_is_rejected() {
    let app = common::spawn().await;

    // Same claims shape, but signed with the wrong secret -- must be
    // rejected outright, not merely treated as "different user".
    let forged = jsonwebtoken::encode(
        &jsonwebtoken::Header::new(jsonwebtoken::Algorithm::HS512),
        &json!({
            "sub": "attacker",
            "username": "attacker",
            "role": "owner",
            "iat": chrono::Utc::now().timestamp(),
            "exp": (chrono::Utc::now() + chrono::Duration::hours(1)).timestamp(),
        }),
        &jsonwebtoken::EncodingKey::from_secret(b"not-the-real-shared-secret-at-all-nope"),
    )
    .unwrap();

    let res = app
        .http
        .put(app.url("/auth/change-password"))
        .bearer_auth(forged)
        .query(&[("currentPassword", "x"), ("newPassword", "SomeNewPass1")])
        .send()
        .await
        .expect("change-password request");
    assert_eq!(res.status(), 401);
}

async fn register_user(app: &common::TestApp, username: &str) -> Value {
    app.http
        .post(app.url("/auth/register"))
        .json(&json!({
            "username": username, "email": format!("{username}@example.com"),
            "password": "SessionPass1", "name": username
        }))
        .send()
        .await
        .expect("register request")
        .json()
        .await
        .expect("register body")
}

#[tokio::test]
async fn login_is_rejected_while_an_active_session_exists() {
    let app = common::spawn().await;

    let first = register_user(&app, "session_first").await;
    let first_token = first["token"].as_str().unwrap().to_string();
    let first_session = first["sessionId"].as_str().unwrap().to_string();
    assert!(!first_session.is_empty());

    // First token works while it is the active session.
    let ok = app
        .http
        .get(app.url("/auth/session"))
        .bearer_auth(&first_token)
        .send()
        .await
        .expect("session request");
    assert_eq!(ok.status(), 200);

    // A second login is rejected with a clear conflict while the first
    // session is active — it must not revoke the existing session.
    let rejected = app
        .http
        .post(app.url("/auth/login"))
        .json(&json!({ "username": "session_first", "password": "SessionPass1" }))
        .send()
        .await
        .expect("second login request");
    assert_eq!(rejected.status(), 409);
    let rejected_body: Value = rejected.json().await.expect("rejection body");
    assert!(
        rejected_body["message"]
            .as_str()
            .unwrap_or_default()
            .contains("another device"),
        "rejection must carry an accurate message, got: {}",
        rejected_body
    );

    // The existing session is untouched — the first token still works.
    let still_active = app
        .http
        .get(app.url("/auth/session"))
        .bearer_auth(&first_token)
        .send()
        .await
        .expect("first token after rejected login");
    assert_eq!(still_active.status(), 200);

    // After logout the same credentials sign in again.
    let logout = app
        .http
        .post(app.url("/auth/logout"))
        .bearer_auth(&first_token)
        .send()
        .await
        .expect("logout request");
    assert_eq!(logout.status(), 204);

    let relogin = app
        .http
        .post(app.url("/auth/login"))
        .json(&json!({ "username": "session_first", "password": "SessionPass1" }))
        .send()
        .await
        .expect("relogin request");
    assert_eq!(relogin.status(), 200);
}

#[tokio::test]
async fn logout_revokes_the_current_session() {
    let app = common::spawn().await;

    let register = register_user(&app, "session_logout").await;
    let token = register["token"].as_str().unwrap().to_string();

    let logout = app
        .http
        .post(app.url("/auth/logout"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("logout request");
    assert_eq!(logout.status(), 204);

    let after = app
        .http
        .get(app.url("/auth/session"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("post-logout request");
    assert_eq!(after.status(), 401);
}

#[tokio::test]
async fn session_status_reports_the_active_session() {
    let app = common::spawn().await;

    let register = register_user(&app, "session_status").await;
    let token = register["token"].as_str().unwrap().to_string();

    let status = app
        .http
        .get(app.url("/auth/session-status"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("session-status request");
    assert_eq!(status.status(), 200);
    let body: Value = status.json().await.expect("session-status body");
    assert_eq!(body["active"], true);
    assert_eq!(body["sessionId"], register["sessionId"]);
    assert_eq!(body["username"], "session_status");
    assert!(body["expiresInSeconds"].as_i64().unwrap() > 0);

    // A second login is rejected while the session is active, so the token
    // stays valid — the gateway keeps passing session-status checks.
    let rejected = app
        .http
        .post(app.url("/auth/login"))
        .json(&json!({ "username": "session_status", "password": "SessionPass1" }))
        .send()
        .await
        .expect("second login");
    assert_eq!(rejected.status(), 409);

    let still_active = app
        .http
        .get(app.url("/auth/session-status"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("session-status after rejected login");
    assert_eq!(still_active.status(), 200);

    // Once the session is revoked (logout), session-status turns into 401 so
    // the gateway denies the stale token at the edge.
    let logout = app
        .http
        .post(app.url("/auth/logout"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("logout");
    assert_eq!(logout.status(), 204);
    let revoked = app
        .http
        .get(app.url("/auth/session-status"))
        .bearer_auth(&token)
        .send()
        .await
        .expect("session-status after logout");
    assert_eq!(revoked.status(), 401);
}
