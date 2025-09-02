use std::sync::Arc;

use crate::cli::{FrameworkTool, resolve_or_install};
use crate::types::Config;

#[derive(Clone)]
pub struct AppState {
    pub cli: Option<FrameworkTool>,
    pub config: Arc<tokio::sync::RwLock<Config>>,
}

impl AppState {
    pub async fn initialize() -> Self {
        let config = Arc::new(tokio::sync::RwLock::new(crate::config::load()));
        match resolve_or_install().await {
            Ok(cli) => Self { cli: Some(cli), config },
            Err(_e) => {
                Self { cli: None, config }
            }
        }
    }
}


