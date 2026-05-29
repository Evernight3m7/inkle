use std::collections::HashMap;

use axum::{extract::State, http::StatusCode, Json};

use crate::handlers::auth::AuthUser;
use crate::models::{
    CategoryStat, ErrorResponse, SiteConfig, StatsResponse, TagStat, UpdateSettingsRequest,
};
use crate::services;
use crate::state::AppState;

pub async fn get_settings(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<SiteConfig>, (StatusCode, Json<ErrorResponse>)> {
    let config = services::theme::load_site_config(&state).await;
    Ok(Json(config))
}

pub async fn update_settings(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<UpdateSettingsRequest>,
) -> Result<Json<SiteConfig>, (StatusCode, Json<ErrorResponse>)> {
    if let Some(v) = &body.site_title {
        let _ = sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('site_title', ?)")
            .bind(v)
            .execute(&state.db)
            .await;
    }
    if let Some(v) = &body.site_subtitle {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO configs (key, value) VALUES ('site_subtitle', ?)",
        )
        .bind(v)
        .execute(&state.db)
        .await;
    }
    if let Some(v) = &body.active_theme {
        let _ = sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('active_theme', ?)")
            .bind(v)
            .execute(&state.db)
            .await;

        // Reload Tera to reflect the theme change immediately
        let new_tera = services::theme::load_tera(v);
        *state.templates.write().unwrap_or_else(|e| e.into_inner()) = new_tera;
        *state.active_theme.write().unwrap_or_else(|e| e.into_inner()) = v.clone();

        tracing::info!("Switched active theme to '{}' via settings", v);
    }
    if let Some(v) = body.posts_per_page {
        let _ = sqlx::query(
            "INSERT OR REPLACE INTO configs (key, value) VALUES ('posts_per_page', ?)",
        )
        .bind(v.to_string())
        .execute(&state.db)
        .await;
    }
    if let Some(v) = &body.social_links {
        let normalized: HashMap<String, String> = v
            .iter()
            .map(|(k, url)| (k.clone(), services::theme::normalize_url(url)))
            .collect();
        let json = serde_json::to_string(&normalized).unwrap_or_default();
        let _ = sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('social_links', ?)")
            .bind(&json)
            .execute(&state.db)
            .await;
    }

    if let Some(v) = &body.ai_base_url {
        let _ = sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('ai_base_url', ?)")
            .bind(v.trim())
            .execute(&state.db)
            .await;
    }
    if let Some(v) = &body.ai_api_key {
        let _ = sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('ai_api_key', ?)")
            .bind(v.trim())
            .execute(&state.db)
            .await;
    }
    if let Some(v) = &body.ai_model {
        let _ = sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('ai_model', ?)")
            .bind(v.trim())
            .execute(&state.db)
            .await;
    }
    if let Some(v) = &body.ai_prompt {
        let _ = sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('ai_prompt', ?)")
            .bind(v)
            .execute(&state.db)
            .await;
    }

    // Invalidate site config cache so the next load_site_config re-fetches from DB
    *state.site_config_cache.write().unwrap_or_else(|e| e.into_inner()) = None;
    // If active_theme was changed, also invalidate theme list cache
    if body.active_theme.is_some() {
        *state.theme_list_cache.write().unwrap_or_else(|e| e.into_inner()) = None;
    }

    get_settings(_auth, State(state)).await
}

pub async fn get_stats(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<StatsResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Category stats
    let categories: Vec<CategoryStat> = sqlx::query_as(
        r#"SELECT category as name, COUNT(*) as count
           FROM posts
           WHERE category != '' AND status = 'published'
           GROUP BY category
           ORDER BY count DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    // Tag stats from junction table
    let tags: Vec<TagStat> = sqlx::query_as(
        r#"SELECT pt.tag as name, COUNT(*) as count
           FROM post_tags pt
           JOIN posts p ON pt.post_id = p.id
           WHERE p.status = 'published'
           GROUP BY pt.tag
           ORDER BY count DESC"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    Ok(Json(StatsResponse { categories, tags }))
}
