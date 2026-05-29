use axum::{
    extract::{Path, Query, State},
    http::{header, request::Parts, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
};
use serde::Deserialize;

use crate::handlers::auth::{get_cookie, AuthPage};
use crate::models::{AdminPost, PostRow, SiteConfig};
use crate::services;
use crate::state::AppState;

// ── Query params ──

#[derive(Deserialize, Default)]
pub struct DashboardQuery {
    pub page: Option<usize>,
    pub q: Option<String>,
}

// ── Helpers ──

async fn load_config(state: &AppState) -> SiteConfig {
    services::theme::load_site_config(state).await
}

fn ai_config_partial(config: &SiteConfig) -> bool {
    let has_url = !config.ai_base_url.is_empty();
    let has_key = !config.ai_api_key.is_empty();
    has_url != has_key
}

fn render_admin(
    state: &AppState,
    template: &str,
    ctx: &tera::Context,
) -> Result<Html<String>, (StatusCode, String)> {
    match state.admin_templates.render(template, ctx) {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            tracing::error!("Admin template render error for '{}': {e}", template);
            Err((StatusCode::INTERNAL_SERVER_ERROR, "Template error".into()))
        }
    }
}

fn mime_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn make_empty_post() -> AdminPost {
    AdminPost {
        id: 0,
        title: String::new(),
        slug: String::new(),
        category: String::new(),
        tags: vec![],
        content: String::new(),
        r#abstract: String::new(),
        status: "draft".into(),
        created_at: String::new(),
        updated_at: String::new(),
    }
}

fn extract_token_from_parts(parts: &Parts) -> Option<String> {
    get_cookie(&parts.headers, "token")
}

fn validate_token(parts: &Parts, state: &AppState) -> bool {
    if state.env.admin_password.is_empty() {
        return true;
    }
    if let Some(token) = extract_token_from_parts(parts) {
        let decoding =
            jsonwebtoken::DecodingKey::from_secret(state.env.jwt_secret.as_bytes());
        let mut validation = jsonwebtoken::Validation::default();
        validation.sub = Some("admin".into());
        jsonwebtoken::decode::<serde_json::Value>(&token, &decoding, &validation).is_ok()
    } else {
        false
    }
}

// ── GET /admin/login ──

pub async fn login_page(
    State(state): State<AppState>,
    parts: Parts,
) -> Response {
    // If already logged in, redirect to dashboard
    if validate_token(&parts, &state) {
        return Redirect::to("/admin").into_response();
    }

    let config = load_config(&state).await;
    let mut ctx = tera::Context::new();
    ctx.insert("site_title", &config.site_title);

    match render_admin(&state, "login.html", &ctx) {
        Ok(html) => html.into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

// ── GET /admin ──

pub async fn dashboard(
    _auth: AuthPage,
    State(state): State<AppState>,
    Query(params): Query<DashboardQuery>,
) -> Response {
    let config = load_config(&state).await;
    let page = params.page.unwrap_or(1).max(1);
    let per_page: usize = 10;
    let query = params.q.as_deref().unwrap_or("").trim().to_string();

    let (posts, total): (Vec<AdminPost>, i64) = if query.is_empty() {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);
        let offset = (page - 1) * per_page;
        let rows: Vec<PostRow> = sqlx::query_as(
            "SELECT * FROM posts ORDER BY created_at DESC LIMIT ? OFFSET ?",
        )
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();
        (rows.into_iter().map(AdminPost::from).collect(), total)
    } else {
        let (rows, total) =
            services::execute_search(&state.db, &query, page, per_page, false).await;
        let posts: Vec<AdminPost> = rows
            .into_iter()
            .map(|r| {
                let tags: Vec<String> =
                    serde_json::from_str(&r.tags).unwrap_or_default();
                AdminPost {
                    id: r.id,
                    title: r.title,
                    slug: r.slug,
                    category: r.category,
                    tags,
                    content: r.content,
                    r#abstract: r.r#abstract,
                    status: r.status,
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                }
            })
            .collect();
        (posts, total)
    };

    let pagination = services::theme::calculate_pagination(page, total as usize, per_page);

    let mut ctx = tera::Context::new();
    ctx.insert("posts", &posts);
    ctx.insert("total", &total);
    ctx.insert("page", &page);
    ctx.insert("per_page", &per_page);
    ctx.insert("pagination", &pagination);
    ctx.insert("query", &query);
    ctx.insert("site_title", &config.site_title);
    ctx.insert("current_path", "dashboard");
    ctx.insert("ai_warning", &ai_config_partial(&config));

    match render_admin(&state, "dashboard.html", &ctx) {
        Ok(html) => html.into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

// ── GET /admin/posts/new ──

pub async fn new_post(
    _auth: AuthPage,
    State(state): State<AppState>,
) -> Response {
    let config = load_config(&state).await;
    let empty_post = make_empty_post();

    let mut ctx = tera::Context::new();
    ctx.insert("is_new", &true);
    ctx.insert("post", &empty_post);
    ctx.insert("site_title", &config.site_title);
    ctx.insert("current_path", "editor");
    ctx.insert("ai_warning", &ai_config_partial(&config));

    match render_admin(&state, "editor.html", &ctx) {
        Ok(html) => html.into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

// ── GET /admin/posts/{id}/edit ──

pub async fn edit_post(
    _auth: AuthPage,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Response {
    let config = load_config(&state).await;

    let row: Option<PostRow> = sqlx::query_as("SELECT * FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .unwrap_or(None);

    match row {
        Some(r) => {
            let post = AdminPost::from(r);
            let mut ctx = tera::Context::new();
            ctx.insert("is_new", &false);
            ctx.insert("post", &post);
            ctx.insert("site_title", &config.site_title);
            ctx.insert("current_path", "editor");
            ctx.insert("ai_warning", &ai_config_partial(&config));

            match render_admin(&state, "editor.html", &ctx) {
                Ok(html) => html.into_response(),
                Err((status, msg)) => (status, msg).into_response(),
            }
        }
        None => {
            let mut ctx = tera::Context::new();
            ctx.insert("site_title", &config.site_title);
            ctx.insert("current_path", "404");
            match render_admin(&state, "404.html", &ctx) {
                Ok(html) => (StatusCode::NOT_FOUND, html).into_response(),
                Err((status, msg)) => (status, msg).into_response(),
            }
        }
    }
}

// ── GET /admin/settings ──

pub async fn settings_page(
    _auth: AuthPage,
    State(state): State<AppState>,
) -> Response {
    let config = load_config(&state).await;

    let mut ctx = tera::Context::new();
    ctx.insert("config", &config);
    ctx.insert("site_title", &config.site_title);
    ctx.insert("current_path", "settings");
    ctx.insert("ai_warning", &ai_config_partial(&config));

    match render_admin(&state, "settings.html", &ctx) {
        Ok(html) => html.into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

// ── GET /admin/themes ──

pub async fn themes_page(
    _auth: AuthPage,
    State(state): State<AppState>,
) -> Response {
    let config = load_config(&state).await;
    let theme_list = services::theme::get_themes_cached(&state);
    let active = state
        .active_theme
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    let mut ctx = tera::Context::new();
    ctx.insert("themes", &theme_list);
    ctx.insert("active", &active);
    ctx.insert("site_title", &config.site_title);
    ctx.insert("current_path", "themes");
    ctx.insert("ai_warning", &ai_config_partial(&config));

    match render_admin(&state, "themes.html", &ctx) {
        Ok(html) => html.into_response(),
        Err((status, msg)) => (status, msg).into_response(),
    }
}

// ── GET /static/admin/{*path} ──

pub async fn serve_admin_static(
    Path(path): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    if path.contains("..") {
        return Err(StatusCode::NOT_FOUND);
    }

    let file_path = std::path::Path::new("static").join(&path);

    // Canonicalize and verify the resolved path stays within static/
    if let Ok(canon) = tokio::fs::canonicalize(&file_path).await {
        if let Ok(static_canon) = tokio::fs::canonicalize("static").await {
            if !canon.starts_with(&static_canon) {
                return Err(StatusCode::NOT_FOUND);
            }
        }
    } else {
        return Err(StatusCode::NOT_FOUND);
    }

    match tokio::fs::read(&file_path).await {
        Ok(content) => {
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_static(mime_for_path(&path)),
            );
            Ok((headers, content))
        }
        Err(_) => Err(StatusCode::NOT_FOUND),
    }
}
