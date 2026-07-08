use axum::{
    routing::{delete, get, post, put},
    Router,
};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};

use crate::{handlers, state::AppState};

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
        .max_age(std::time::Duration::from_secs(3600));

    let api_routes = Router::new()
        .route("/health", get(handlers::health::health))
        .route("/auth/login", post(handlers::auth::login))
        .route("/auth/register", post(handlers::auth::register))
        .route(
            "/auth/register-with-profile",
            post(handlers::auth::register_with_profile),
        )
        .route("/auth/change-password", put(handlers::auth::change_password))
        .route(
            "/auth/users/check-existence",
            post(handlers::auth::check_existence),
        )
        .route("/auth/users/:id", get(handlers::auth::get_user_identity))
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
        .with_state(state);

    // Mirrors the Java service's `server.servlet.context-path: /api`.
    Router::new().nest("/api", api_routes).layer(cors)
}
