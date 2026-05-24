use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tera::Tera;

use crate::models::{Pagination, Post, PostRow, SiteConfig};
use crate::services::markdown;
use crate::state::AppState;

// ── Theme manifest (theme.json) ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeManifest {
    pub name: String,
    pub title: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub thumbnail: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ThemeListResponse {
    pub themes: Vec<ThemeManifest>,
    pub active: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ActivateThemeResponse {
    pub active: String,
}

// ── Theme scanning ──

pub fn scan_themes() -> Vec<ThemeManifest> {
    let themes_dir = Path::new("themes");
    if !themes_dir.is_dir() {
        tracing::warn!("themes/ directory not found or not a directory");
        return vec![];
    }

    let mut manifests: Vec<ThemeManifest> = Vec::new();

    let entries = match fs::read_dir(themes_dir) {
        Ok(e) => e,
        Err(e) => {
            tracing::warn!("Failed to read themes/ directory: {e}");
            return vec![];
        }
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let folder_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        let theme_json_path = path.join("theme.json");

        if !theme_json_path.is_file() {
            continue;
        }

        let content = match fs::read_to_string(&theme_json_path) {
            Ok(c) => c,
            Err(e) => {
                tracing::warn!("Failed to read {}/theme.json: {e}", folder_name);
                continue;
            }
        };

        let manifest: ThemeManifest = match serde_json::from_str(&content) {
            Ok(m) => m,
            Err(e) => {
                tracing::warn!("Invalid theme.json in '{folder_name}': {e}");
                continue;
            }
        };

        if manifest.name != folder_name {
            tracing::warn!(
                "Theme name '{}' does not match folder name '{}', skipping",
                manifest.name,
                folder_name
            );
            continue;
        }

        manifests.push(manifest);
    }

    manifests.sort_by(|a, b| a.name.cmp(&b.name));
    manifests
}

/// Returns the list of theme manifests, using a cached copy in AppState
/// when available to avoid re-scanning the filesystem on every request.
pub fn get_themes_cached(state: &AppState) -> Vec<ThemeManifest> {
    if let Some(cached) = &*state.theme_list_cache.read().unwrap_or_else(|e| e.into_inner()) {
        return cached.clone();
    }

    let themes = scan_themes();
    *state.theme_list_cache.write().unwrap_or_else(|e| e.into_inner()) = Some(themes.clone());
    themes
}

// ── Tera loading ──

/// Load Tera templates from the given theme's templates/ directory.
/// Returns an empty Tera instance if the directory doesn't exist or has no templates.
pub fn load_tera(theme_name: &str) -> Tera {
    let pattern = format!("themes/{}/templates/**/*", theme_name);
    match Tera::new(&pattern) {
        Ok(t) => {
            tracing::info!("Loaded templates for theme '{}'", theme_name);
            t
        }
        Err(e) => {
            tracing::warn!("Failed to load templates for theme '{}': {e}", theme_name);
            Tera::default()
        }
    }
}

// ── Post conversion ──

/// Convert a DB row (raw markdown) to a frontend-ready Post (rendered HTML).
pub fn post_row_to_frontend(row: PostRow) -> Post {
    let tags: Vec<String> = serde_json::from_str(&row.tags).unwrap_or_default();
    let content = markdown::render_markdown(&row.content);
    let r#abstract = markdown::render_markdown(&row.r#abstract);

    Post {
        title: row.title,
        slug: row.slug,
        category: row.category,
        tags,
        content,
        r#abstract,
        created_at: row.created_at,
        updated_at: row.updated_at,
        snippet: None,
    }
}

// ── Config loading ──

/// Load site configuration from the configs K-V table.
/// Results are cached in AppState to avoid a DB query on every request.
pub async fn load_site_config(state: &AppState) -> SiteConfig {
    // Check cache first
    if let Some(cached) = &*state.site_config_cache.read().unwrap_or_else(|e| e.into_inner()) {
        return cached.clone();
    }

    let rows: Vec<(String, String)> = sqlx::query_as("SELECT key, value FROM configs")
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

    let mut site_title = String::new();
    let mut site_subtitle = String::new();
    let mut active_theme = String::new();
    let mut posts_per_page: u32 = 10;
    let mut social_links = HashMap::new();

    for (key, value) in rows {
        match key.as_str() {
            "site_title" => site_title = value,
            "site_subtitle" => site_subtitle = value,
            "active_theme" => active_theme = value,
            "posts_per_page" => posts_per_page = value.parse().unwrap_or(10),
            "social_links" => {
                let raw: HashMap<String, String> = serde_json::from_str(&value).unwrap_or_default();
                social_links = raw
                    .into_iter()
                    .map(|(k, v)| (k, normalize_url(&v)))
                    .collect();
            }
            _ => {}
        }
    }

    let config = SiteConfig {
        site_title,
        site_subtitle,
        active_theme,
        posts_per_page,
        social_links,
    };

    // Populate cache
    *state.site_config_cache.write().unwrap_or_else(|e| e.into_inner()) = Some(config.clone());

    config
}

// ── Pagination ──

pub fn calculate_pagination(
    current: usize,
    total_items: usize,
    per_page: usize,
) -> Pagination {
    let total = if total_items == 0 {
        1
    } else {
        (total_items + per_page - 1) / per_page
    };

    let prev = if current > 1 { Some(current - 1) } else { None };
    let next = if current < total { Some(current + 1) } else { None };

    Pagination {
        current,
        total,
        prev,
        next,
    }
}

pub fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return String::new();
    }
    if url.starts_with("https://") || url.starts_with("http://") || url.starts_with("//") {
        return url.to_string();
    }
    // Contains :// with a non-http scheme (e.g. javascript://) — strip the scheme.
    if let Some(pos) = url.find("://") {
        let host = &url[pos + 3..];
        if host.is_empty() {
            return String::new();
        }
        return format!("https://{}", host);
    }
    // No scheme found, prepend https://
    format!("https://{}", url)
}
