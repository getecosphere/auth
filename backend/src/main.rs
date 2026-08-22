use rwid_auth_service::bootstrap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    let app = bootstrap().await?;
    let config = rwid_auth_service::config::AppConfig::from_env()?;
    tracing::info!(port = config.server_port, "starting rwid-auth-service");
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", config.server_port)).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;
    Ok(())
}
