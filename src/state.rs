use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use sqlx::SqlitePool;
use tera::Tera;

use crate::config::EnvConfig;
use crate::models::SiteConfig;
use crate::services::theme::ThemeManifest;

#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub env: EnvConfig,
    pub templates: Arc<RwLock<Tera>>,
    pub active_theme: Arc<RwLock<String>>,
    pub admin_templates: Tera,
    pub login_rate_limiter:
        Arc<tokio::sync::Mutex<HashMap<String, (u32, std::time::Instant)>>>,
    pub ai_rate_limiter:
        Arc<tokio::sync::Mutex<HashMap<String, (u32, std::time::Instant)>>>,
    pub site_config_cache: Arc<RwLock<Option<SiteConfig>>>,
    pub theme_list_cache: Arc<RwLock<Option<Vec<ThemeManifest>>>>,
}
