use std::env;

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
    pub fn from_env() -> Self {
        let server_port = env::var("SERVER_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(8080);

        Self {
            mongodb_uri: env::var("MONGODB_URI")
                .unwrap_or_else(|_| "mongodb://localhost:27017/rwid_community".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "your-secret-key-change-in-production".to_string()),
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
        }
    }
}
