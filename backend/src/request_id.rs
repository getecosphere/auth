//! Per-request correlation id: reused from an incoming `x-request-id`
//! header when present, otherwise generated fresh. Recorded on the
//! request's tracing span (so JSON log lines carry it and can be filtered
//! by it in Loki/whatever), echoed back on the response, and readable via
//! `current()` from anywhere else in the same request's async call tree
//! -- most importantly from `*_client.rs` peer HTTP calls, so it can be
//! forwarded downstream without threading it through every handler
//! signature.
use axum::{
    extract::Request,
    http::{HeaderName, HeaderValue},
    middleware::Next,
    response::Response,
};
use tracing::Instrument;

pub const HEADER_NAME: &str = "x-request-id";

tokio::task_local! {
    static REQUEST_ID: String;
}

/// The current request's correlation id, if called from within a
/// request's async call tree. `None` outside of one (e.g. a background
/// task not spawned from inside `propagate`).
pub fn current() -> Option<String> {
    REQUEST_ID.try_with(|id| id.clone()).ok()
}

pub async fn propagate(request: Request, next: Next) -> Response {
    let id = request
        .headers()
        .get(HEADER_NAME)
        .and_then(|value| value.to_str().ok())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());

    let span = tracing::info_span!("request", request_id = %id);
    let response_id = id.clone();

    let mut response = REQUEST_ID
        .scope(id, next.run(request).instrument(span))
        .await;

    if let Ok(value) = HeaderValue::from_str(&response_id) {
        response
            .headers_mut()
            .insert(HeaderName::from_static(HEADER_NAME), value);
    }
    response
}
