//! Shared integration-test harness. Spawns a real axum server on an
//! OS-assigned port (not `.oneshot()`) so the rate limiter's
//! `SmartIpKeyExtractor` sees a real `ConnectInfo<SocketAddr>`, exactly
//! matching how the service actually runs -- see the comment on this in
//! `main.rs`. `AppConfig` is built directly as a struct literal rather
//! than through `AppConfig::from_env()`, since `cargo test` runs test
//! functions concurrently in the same process and `std::env::set_var`
//! would race across them.
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use rwid_auth_service::{config::AppConfig, routes, state::AppState};
use serde::Serialize;
use std::net::SocketAddr;
use uuid::Uuid;

pub const TEST_JWT_SECRET: &str =
    "this-is-a-64-byte-or-longer-test-secret-for-hs512-signing-parity-check!!";

pub struct TestApp {
    pub base_url: String,
    pub http: reqwest::Client,
    pub db: mongodb::Database,
}

impl TestApp {
    pub fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }
}

impl Drop for TestApp {
    fn drop(&mut self) {
        let db = self.db.clone();
        tokio::spawn(async move {
            let _ = db.drop(None).await;
        });
    }
}

/// Spawns a fresh instance of the service against its own uniquely-named
/// local MongoDB database (dropped when the returned `TestApp` is
/// dropped), bound to a random free port.
pub async fn spawn() -> TestApp {
    let run_id = Uuid::new_v4().simple().to_string();
    let db_name = format!("auth_test_{run_id}");
    let mongodb_uri = format!("mongodb://localhost:27017/{db_name}");

    let client = mongodb::Client::with_uri_str(&mongodb_uri)
        .await
        .expect("connect to local test MongoDB (is `mongod` running on localhost:27017?)");
    let db = client.default_database().expect("db name in URI");

    let config = AppConfig {
        mongodb_uri,
        jwt_secret: TEST_JWT_SECRET.to_string(),
        jwt_expiration_ms: 3_600_000,
        server_port: 0,
        api_base_url: "http://placeholder/api".to_string(),
        cors_allowed_origins: vec!["http://localhost:3000".to_string()],
        rate_limit_general_burst: 120,
        rate_limit_general_replenish_secs: 1,
        rate_limit_auth_burst: 100,
        rate_limit_auth_replenish_secs: 1,
        email_verification_required: false,
        email_verification_ttl_hours: 24,
        password_reset_ttl_minutes: 60,
        brevo_api_key: String::new(),
        mail_from_email: String::new(),
        mail_from_name: "Test".to_string(),
        email_relay_url: String::new(),
        email_relay_token: String::new(),
        auth_public_url: "http://placeholder/api".to_string(),
        default_role: "member".to_string(),
        allowed_roles: Vec::new(),
        signup_event_url: None,
        signup_event_token: None,
        session_required: true,
    };

    let state = AppState::new(db.clone(), config);
    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .expect("bind test listener");
    let addr = listener.local_addr().expect("listener local addr");
    tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .expect("test server crashed");
    });

    TestApp {
        base_url: format!("http://{addr}/api"),
        http: reqwest::Client::new(),
        db,
    }
}

#[derive(Serialize)]
struct TestClaims {
    sub: String,
    username: String,
    role: String,
    iat: i64,
    exp: i64,
}

/// Mints a JWT with the same HS512/claims shape auth actually issues (see
/// `jwt::generate_token`), signed with `TEST_JWT_SECRET`. Every domain's
/// test harness has its own copy of this since the domains are separate
/// crates/repos by design -- keep it in sync with `jwt::Claims` if that
/// shape ever changes.
pub fn mint_token(user_id: &str, username: &str, role: &str) -> String {
    let now = chrono::Utc::now();
    let claims = TestClaims {
        sub: user_id.to_string(),
        username: username.to_string(),
        role: role.to_string(),
        iat: now.timestamp(),
        exp: (now + chrono::Duration::hours(1)).timestamp(),
    };
    encode(
        &Header::new(Algorithm::HS512),
        &claims,
        &EncodingKey::from_secret(TEST_JWT_SECRET.as_bytes()),
    )
    .expect("encode test jwt")
}
