use rwid_auth_service::{
    config::{AppConfig, StorageBackend},
    routes, s3_storage,
    state::AppState,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    // JSON, not the default human-readable text: every field (including
    // the request_id set by request_id::propagate on the request's span)
    // ends up as a real, queryable field instead of buried in a formatted
    // string -- what a log aggregator like Loki actually wants.
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info".into()),
        )
        .init();

    let config = AppConfig::from_env()?;

    let client = mongodb::Client::with_uri_str(&config.mongodb_uri).await?;
    let db = client
        .default_database()
        .ok_or_else(|| anyhow::anyhow!("MONGODB_URI must include a database name"))?;

    tracing::info!(port = config.server_port, "starting rwid-auth-service");

    let state = AppState::new(db, config.clone());

    // Fail fast if S3 is misconfigured -- an unreachable bucket should
    // stop startup, not surface as a mysterious 500 on the first upload.
    if config.storage_backend == StorageBackend::S3 {
        let client = state.s3_client.as_ref().expect("s3_client set when storage_backend is S3");
        s3_storage::ensure_bucket(client, &config.s3_bucket).await?;
        tracing::info!(bucket = %config.s3_bucket, endpoint = %config.s3_endpoint, "using S3 storage backend");
    }

    let app = routes::build_router(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.server_port)).await?;
    // Rate limiting's SmartIpKeyExtractor falls back to the TCP peer address
    // when no x-forwarded-for/x-real-ip/forwarded header is present, which
    // requires ConnectInfo to be available -- without this, requests with
    // none of those headers (e.g. direct local access) would be rejected
    // outright instead of just rate-limited by peer IP.
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
