use std::env;

/// Placeholder secrets that have appeared in this codebase's example/default
/// config at some point. Refusing to start on these specifically (on top of
/// the general length check) catches the case where a real deployment
/// copy-pasted a default instead of generating its own secret.
const KNOWN_PLACEHOLDER_SECRETS: &[&str] = &[
    "your-secret-key-change-in-production",
    "change-this-secret",
    "secret",
];

/// HS512 wants a key at least as long as its output (64 bytes / 512 bits) or
/// the signature offers less security than the algorithm name implies. 32 is
/// enforced as a hard minimum; anything under 64 is allowed but warned about.
const MIN_JWT_SECRET_BYTES: usize = 32;
const RECOMMENDED_JWT_SECRET_BYTES: usize = 64;

#[derive(Clone)]
pub struct AppConfig {
    pub mongodb_uri: String,
    pub jwt_secret: String,
    pub jwt_expiration_ms: i64,
    pub server_port: u16,
    pub api_base_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub storage_local_path: String,
}

impl AppConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let server_port = env::var("SERVER_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080);

        let jwt_secret = env::var("JWT_SECRET").unwrap_or_default();
        if jwt_secret.is_empty() {
            anyhow::bail!(
                "JWT_SECRET is not set. Refusing to start with no signing key -- \
                 generate one (e.g. `openssl rand -base64 48`) and set it in .env."
            );
        }
        if KNOWN_PLACEHOLDER_SECRETS.contains(&jwt_secret.as_str()) {
            anyhow::bail!(
                "JWT_SECRET is set to a known placeholder value. Refusing to start -- \
                 generate a real secret and set it in .env."
            );
        }
        if jwt_secret.len() < MIN_JWT_SECRET_BYTES {
            anyhow::bail!(
                "JWT_SECRET is only {} bytes; refusing to start with fewer than {} \
                 for HS512 signing.",
                jwt_secret.len(),
                MIN_JWT_SECRET_BYTES
            );
        }
        if jwt_secret.len() < RECOMMENDED_JWT_SECRET_BYTES {
            tracing::warn!(
                bytes = jwt_secret.len(),
                recommended = RECOMMENDED_JWT_SECRET_BYTES,
                "JWT_SECRET is shorter than recommended for HS512"
            );
        }

        Ok(Self {
            mongodb_uri: env::var("MONGODB_URI")
                .unwrap_or_else(|_| "mongodb://localhost:27017/rwid_community".to_string()),
            jwt_secret,
            jwt_expiration_ms: env::var("JWT_EXPIRATION")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(86_400_000),
            server_port,
            api_base_url: env::var("API_BASE_URL")
                .unwrap_or_else(|_| format!("http://localhost:{server_port}/api")),
            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            storage_local_path: env::var("STORAGE_LOCAL_PATH")
                .unwrap_or_else(|_| "./storage".to_string()),
        })
    }
}
