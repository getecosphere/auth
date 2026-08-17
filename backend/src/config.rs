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
    /// Everyday rate limit shared across all non-credential routes, per
    /// source IP. Tunable via env instead of a recompile -- dev traffic
    /// (page-load fan-out across peer services, hot reload) legitimately
    /// needs a larger burst than what's safe to hardcode as the only value.
    pub rate_limit_general_burst: u32,
    pub rate_limit_general_replenish_secs: u64,
    /// Tight limit for login/register/change-password specifically -- kept
    /// separate from the general limit and defaults conservative, since this
    /// one exists to blunt brute force / credential stuffing.
    pub rate_limit_auth_burst: u32,
    pub rate_limit_auth_replenish_secs: u64,
    pub email_verification_required: bool,
    pub email_verification_ttl_hours: i64,
    pub brevo_api_key: String,
    pub mail_from_email: String,
    pub mail_from_name: String,
    pub auth_public_url: String,
    /// Default role assigned to new accounts when the request does not carry
    /// one. Declared by the estate in ecompose.yml's `auth.roles` block (Eco
    /// writes it into this service's .env as `ECO_AUTH_DEFAULT_ROLE`) — never
    /// hardcoded in the binary, so each estate picks its own identity model.
    pub default_role: String,
    /// Roles this estate accepts on registration (comma-separated from
    /// `ECO_AUTH_ROLES`). Empty means the client-supplied role is trusted as-is
    /// (legacy behavior); non-empty means a requested role must be in the list
    /// or the account defaults to `default_role`.
    pub allowed_roles: Vec<String>,
    /// Optional outbound sink for the `user.signed_up` domain event. When set,
    /// auth fire-and-forget POSTs a JSON event after each successful
    /// registration. Auth never interprets this URL — the composer decides who
    /// consumes it (e.g. the notifications LXS ingest endpoint).
    pub signup_event_url: Option<String>,
    /// Optional bearer token attached to the signup event POST. Auth never
    /// mints or interprets it; the sink's owner supplies it.
    pub signup_event_token: Option<String>,
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
                .unwrap_or(2_592_000_000),
            server_port,
            api_base_url: env::var("API_BASE_URL")
                .unwrap_or_else(|_| format!("http://localhost:{server_port}/api")),
            cors_allowed_origins: env::var("CORS_ALLOWED_ORIGINS")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            rate_limit_general_burst: env::var("RATE_LIMIT_GENERAL_BURST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(120),
            rate_limit_general_replenish_secs: env::var("RATE_LIMIT_GENERAL_REPLENISH_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(1),
            rate_limit_auth_burst: env::var("RATE_LIMIT_AUTH_BURST")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5),
            rate_limit_auth_replenish_secs: env::var("RATE_LIMIT_AUTH_REPLENISH_SECS")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(10),
            email_verification_required: env::var("EMAIL_VERIFICATION_REQUIRED")
                .map(|v| matches!(v.to_lowercase().as_str(), "1" | "true" | "yes" | "on"))
                .unwrap_or(true),
            email_verification_ttl_hours: env::var("EMAIL_VERIFICATION_TTL_HOURS")
                .ok().and_then(|v| v.parse().ok()).unwrap_or(24),
            brevo_api_key: env::var("BREVO_API_KEY").unwrap_or_default(),
            mail_from_email: env::var("MAIL_FROM_EMAIL").unwrap_or_default(),
            mail_from_name: env::var("MAIL_FROM_NAME").unwrap_or_else(|_| "Auth".to_string()),
            // Verification links belong to the public application, not the
            // Auth API. Prefer an explicit URL; otherwise Eco's generated
            // first CORS origin is the estate frontend in both dev and prod.
            // AUTH_PUBLIC_URL remains a backwards-compatible final fallback.
            auth_public_url: env::var("EMAIL_VERIFICATION_PUBLIC_URL")
                .ok()
                .filter(|value| !value.trim().is_empty())
                .or_else(|| {
                    env::var("CORS_ALLOWED_ORIGINS").ok().and_then(|origins| {
                        origins
                            .split(',')
                            .next()
                            .map(str::trim)
                            .filter(|value| !value.is_empty())
                            .map(str::to_owned)
                    })
                })
                .or_else(|| env::var("AUTH_PUBLIC_URL").ok())
                .unwrap_or_default(),
            default_role: env::var("ECO_AUTH_DEFAULT_ROLE")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| "member".to_string()),
            allowed_roles: env::var("ECO_AUTH_ROLES")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            signup_event_url: env::var("SIGNUP_EVENT_URL")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            signup_event_token: env::var("SIGNUP_EVENT_TOKEN")
                .ok()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
        })
    }
}
