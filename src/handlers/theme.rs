use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};

use crate::handlers::auth::AuthUser;
use crate::models::ErrorResponse;
use crate::services::theme::{self, ActivateThemeResponse, ThemeListResponse};
use crate::state::AppState;

pub async fn list_themes(
    _auth: AuthUser,
    State(state): State<AppState>,
) -> Result<Json<ThemeListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let themes = theme::get_themes_cached(&state);
    let active = state
        .active_theme
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    Ok(Json(ThemeListResponse { themes, active }))
}

pub async fn activate_theme(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<ActivateThemeResponse>, (StatusCode, Json<ErrorResponse>)> {
    let themes = theme::get_themes_cached(&state);
    if !themes.iter().any(|t| t.name == name) {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: format!("theme '{}' not found", name),
            }),
        ));
    }

    // Persist to database
    if let Err(e) =
        sqlx::query("INSERT OR REPLACE INTO configs (key, value) VALUES ('active_theme', ?)")
            .bind(&name)
            .execute(&state.db)
            .await
    {
        tracing::error!("Failed to persist theme switch to database: {e}");
    }

    // Reload Tera with the new theme's templates
    let new_tera = theme::load_tera(&name);
    *state.templates.write().unwrap_or_else(|e| e.into_inner()) = new_tera;
    *state
        .active_theme
        .write()
        .unwrap_or_else(|e| e.into_inner()) = name.clone();

    tracing::info!("Switched active theme to '{}'", name);

    // Invalidate caches since the active theme and config may have changed
    *state.site_config_cache.write().unwrap_or_else(|e| e.into_inner()) = None;
    *state.theme_list_cache.write().unwrap_or_else(|e| e.into_inner()) = None;

    Ok(Json(ActivateThemeResponse { active: name }))
}
