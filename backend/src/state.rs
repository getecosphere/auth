use mongodb::Database;
use std::sync::Arc;

use crate::config::{AppConfig, StorageBackend};

#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub db: Database,
    pub config: AppConfig,
    /// Only Some when config.storage_backend is S3 -- building the client
    /// itself makes no network call, so this is cheap, but there's no
    /// endpoint/credentials to build one from in Local mode.
    pub s3_client: Option<aws_sdk_s3::Client>,
}

impl AppState {
    pub fn new(db: Database, config: AppConfig) -> Self {
        let s3_client = match config.storage_backend {
            StorageBackend::S3 => Some(crate::s3_storage::build_client(&config)),
            StorageBackend::Local => None,
        };
        AppState(Arc::new(AppStateInner { db, config, s3_client }))
    }
}

impl std::ops::Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
