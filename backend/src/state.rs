use mongodb::Database;
use std::sync::Arc;

use crate::config::AppConfig;

#[derive(Clone)]
pub struct AppState(pub Arc<AppStateInner>);

pub struct AppStateInner {
    pub db: Database,
    pub config: AppConfig,
}

impl AppState {
    pub fn new(db: Database, config: AppConfig) -> Self {
        AppState(Arc::new(AppStateInner { db, config }))
    }
}

impl std::ops::Deref for AppState {
    type Target = AppStateInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
