use std::{sync::Arc, time::Duration};

use axum::{
    body::Body,
    http::{header::CONTENT_TYPE, HeaderValue, StatusCode},
    response::Response,
    routing::{delete, get, post, put},
    Router,
};
use tower::ServiceBuilder;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::SmartIpKeyExtractor, GovernorError,
    GovernorLayer,
};
use tower_http::{
    cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer},
    limit::RequestBodyLimitLayer,
};

use crate::{handlers, state::AppState};

/// Multipart avatar/cover uploads decode+re-encode a whole image in memory;
/// this is generous for a profile photo but bounds worst-case allocation
/// from the raw upload size (the decompression-bomb guard in storage.rs
/// bounds the *decoded* size separately).
const MAX_BODY_BYTES: usize = 10 * 1024 * 1024;

pub fn build_router(state: AppState) -> Router {
    let origins: Vec<_> = state
        .config
        .cors_allowed_origins
        .iter()
        .filter_map(|o| o.parse().ok())
        .collect();

    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods(AllowMethods::list([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::PATCH,
            axum::http::Method::OPTIONS,
            axum::http::Method::HEAD,
        ]))
        .allow_headers(AllowHeaders::mirror_request())
        .expose_headers([
            axum::http::header::CONTENT_TYPE,
            axum::http::header::AUTHORIZATION,
        ])
        .allow_credentials(true)
        .max_age(Duration::from_secs(3600));

    // auth-sensitive endpoints get their own, much stricter limiter
    let auth_governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(state.config.rate_limit_auth_replenish_secs)
            .burst_size(state.config.rate_limit_auth_burst)
            .error_handler(rate_limit_error)
            .finish()
            .expect("valid governor config"),
    );
    spawn_governor_cleanup(auth_governor_config.clone());

    let general_governor_config = Arc::new(
        GovernorConfigBuilder::default()
            .key_extractor(SmartIpKeyExtractor)
            .per_second(state.config.rate_limit_general_replenish_secs)
            .burst_size(state.config.rate_limit_general_burst)
            .error_handler(rate_limit_error)
            .finish()
            .expect("valid governor config"),
    );
    spawn_governor_cleanup(general_governor_config.clone());

    let credential_routes = Router::new()
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/register", post(handlers::auth::register))
        .route("/auth/verify-email", get(handlers::auth::verify_email))
        .route("/auth/resend-verification", post(handlers::auth::resend_verification))
        .route(
            "/auth/register-with-profile",
            post(handlers::auth::register_with_profile),
        )
        .route(
            "/auth/change-password",
            put(handlers::auth::change_password),
        )
        .route("/auth/me", put(handlers::auth::update_identity))
        .layer(GovernorLayer {
            config: auth_governor_config,
        });

    let general_routes = Router::new()
        .route("/health", get(handlers::health::health))
        // This is polled by the shared frontend layout. It authenticates the
        // caller itself, but must not consume the small credential-stuffing
        // budget that protects login and registration.
        .route("/auth/verification-status", get(handlers::auth::verification_status))
        .route(
            "/auth/users/check-existence",
            post(handlers::auth::check_existence),
        )
        .route("/auth/users/:id", get(handlers::auth::get_user_identity))
        .route(
            "/auth/users/username/:username",
            get(handlers::auth::get_user_identity_by_username),
        )
        .route("/users/:id", delete(handlers::users::deactivate_user))
        .route("/users/:id/avatar", post(handlers::users::upload_avatar))
        .route(
            "/users/:id/upload-cover-photo",
            post(handlers::users::upload_cover_photo),
        )
        .route(
            "/files/:id",
            get(handlers::files::download_file).delete(handlers::files::delete_file),
        )
        .route("/files/view/:id", get(handlers::files::view_file))
        .layer(GovernorLayer {
            config: general_governor_config,
        });

    let api_routes = credential_routes
        .merge(general_routes)
        .layer(
            ServiceBuilder::new()
                .layer(RequestBodyLimitLayer::new(MAX_BODY_BYTES))
                .layer(axum::middleware::map_response(security_headers)),
        )
        .with_state(state);

    // Mirrors the Java service's `server.servlet.context-path: /api`.
    Router::new()
        .nest("/api", api_routes)
        .layer(cors)
        .layer(axum::middleware::from_fn(crate::request_id::propagate))
}

/// `tower_governor` defaults to a plain-text 429. Browser clients use JSON
/// for every Auth response, so keep throttling machine-readable and useful
/// instead of surfacing a misleading "invalid response" message.
fn rate_limit_error(error: GovernorError) -> Response {
    let (status, headers, message) = match error {
        GovernorError::TooManyRequests { headers, .. } => (
            StatusCode::TOO_MANY_REQUESTS,
            headers,
            "Terlalu banyak percobaan. Tunggu sebentar lalu coba lagi.",
        ),
        GovernorError::UnableToExtractKey => (
            StatusCode::INTERNAL_SERVER_ERROR,
            None,
            "Permintaan tidak dapat diproses. Coba lagi.",
        ),
        GovernorError::Other { code, headers, .. } => (
            code,
            headers,
            "Permintaan tidak dapat diproses. Coba lagi.",
        ),
    };
    let mut response = Response::new(Body::from(format!(r#"{{"error":"{message}"}}"#)));
    *response.status_mut() = status;
    if let Some(headers) = headers {
        response.headers_mut().extend(headers);
    }
    response.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/json; charset=utf-8"),
    );
    response
}

/// Restores the response headers Spring Security was adding by default in
/// the Java version (confirmed via a live boot log), since axum sends none
/// of these unless explicitly set.
async fn security_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));
    headers.insert("x-xss-protection", HeaderValue::from_static("0"));
    headers.insert("referrer-policy", HeaderValue::from_static("no-referrer"));
    headers.insert(
        "cache-control",
        HeaderValue::from_static("no-cache, no-store, max-age=0, must-revalidate"),
    );
    headers.insert("pragma", HeaderValue::from_static("no-cache"));
    response
}

/// The keyed rate-limit store grows one entry per distinct client key seen;
/// without periodic cleanup that's unbounded memory growth from an attacker
/// cycling source IPs. Mirrors the cleanup pattern from tower_governor's own
/// example, just as a tokio task instead of a raw thread.
fn spawn_governor_cleanup(
    config: Arc<
        tower_governor::governor::GovernorConfig<
            SmartIpKeyExtractor,
            governor::middleware::NoOpMiddleware,
        >,
    >,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            config.limiter().retain_recent();
        }
    });
}
