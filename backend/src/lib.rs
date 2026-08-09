//! Library surface so `tests/` (real integration tests, not unit tests
//! embedded in each module) can build a real `Router` and `AppState`
//! against a real MongoDB, exactly like `main.rs` does. `main.rs` is a
//! thin wrapper around this crate; nothing in it duplicates logic that
//! lives here.
pub mod auth_extractor;
pub mod config;
pub mod dto;
pub mod email_verification;
pub mod error;
pub mod handlers;
pub mod jwt;
pub mod models;
pub mod password;
pub mod request_id;
pub mod routes;
pub mod s3_storage;
pub mod state;
pub mod storage;
pub mod user_repo;

pub async fn bootstrap() -> anyhow::Result<axum::Router> {
    let _ = dotenvy::dotenv();
    let config = config::AppConfig::from_env()?;
    let client = mongodb::Client::with_uri_str(&config.mongodb_uri).await?;
    let db = client.default_database()
        .ok_or_else(|| anyhow::anyhow!("MONGODB_URI must include a database name"))?;
    let state = state::AppState::new(db, config.clone());
    if config.storage_backend == config::StorageBackend::S3 {
        if let Some(s3) = state.s3_client.as_ref() {
            s3_storage::ensure_bucket(s3, &config.s3_bucket).await?;
        }
    }
    Ok(routes::build_router(state))
}
