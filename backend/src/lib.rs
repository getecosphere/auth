//! Library surface so `tests/` (real integration tests, not unit tests
//! embedded in each module) can build a real `Router` and `AppState`
//! against a real MongoDB, exactly like `main.rs` does. `main.rs` is a
//! thin wrapper around this crate; nothing in it duplicates logic that
//! lives here.
pub mod auth_extractor;
pub mod config;
pub mod dto;
pub mod error;
pub mod handlers;
pub mod jwt;
pub mod models;
pub mod password;
pub mod routes;
pub mod state;
pub mod storage;
pub mod user_repo;
