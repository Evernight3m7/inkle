use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{Html, IntoResponse, Response},
};
use serde::Deserialize;

use crate::models::{GlobalContext, Post, PostRow};
use crate::services;
use crate::state::AppState;

// ── Query params ──

#[derive(Debug, Deserialize, Default)]
pub struct PageQuery {
    pub page: Option<usize>,
}

#[derive(Debug, Deserialize, Default)]
pub struct SearchPageQuery {
    pub q: Option<String>,
    pub page: Option<usize>,
}

// ── Helpers ──

fn mime_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("");
    match ext {
        "css" => "text/css",
        "js" | "mjs" => "application/javascript",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "ico" => "image/x-icon",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        "ttf" => "font/ttf",
        "pdf" => "application/pdf",
        _ => "application/octet-stream",
    }
}

fn build_global_context(
    config: crate::models::SiteConfig,
    active_theme: &str,
    current_url: &str,
) -> GlobalContext {
    GlobalContext {
        config,
        current_theme: active_theme.to_string(),
        current_url: current_url.to_string(),
    }
}

fn render_template(
    tera: &tera::Tera,
    template_name: &str,
    ctx: &tera::Context,
) -> Result<Html<String>, (StatusCode, String)> {
    match tera.render(template_name, ctx) {
        Ok(html) => Ok(Html(html)),
        Err(e) => {
            tracing::error!("Template render error for '{}': {e}", template_name);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                "Template render error".into(),
            ))
        }
    }
}

// ── Static file serving ──

pub async fn serve_static(
    State(state): State<AppState>,
    Path(path): Path<String>,
) -> Result<impl IntoResponse, StatusCode> {
    // Basic path traversal protection
    if path.contains("..") {
        return Err(StatusCode::NOT_FOUND);
    }

    let theme = state
        .active_theme
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .clone();

    let theme_static_dir = format!("themes/{}/static", theme);
    let file_path = std::path::Path::new(&theme_static_dir).join(&path);

    // Canonicalize and verify the resolved path stays within the theme's static directory
    if let Ok(canon) = tokio::fs::canonicalize(&file_path).await {
        if let Ok(theme_static_canon) = tokio::fs::canonicalize(&theme_static_dir).await {
            if !canon.starts_with(&theme_static_canon) {
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

// ── Page: index ──

pub async fn index(
    State(state): State<AppState>,
    Query(pq): Query<PageQuery>,
) -> Result<Response, (StatusCode, String)> {
    let config = services::theme::load_site_config(&state).await;
    let per_page = config.posts_per_page as usize;
    let page = pq.page.unwrap_or(1).max(1);

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE status = 'published'",
    )
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let offset = (page - 1) * per_page;
    let rows: Vec<PostRow> = sqlx::query_as(
        "SELECT * FROM posts WHERE status = 'published' ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let posts: Vec<_> = rows
        .into_iter()
        .map(services::theme::post_row_to_frontend)
        .collect();

    let pagination = services::theme::calculate_pagination(page, total as usize, per_page);
    let active_theme = state.active_theme.read().unwrap_or_else(|e| e.into_inner()).clone();
    let global = build_global_context(config, &active_theme, "/");

    let mut ctx = tera::Context::new();
    ctx.insert("global", &global);
    ctx.insert("posts", &posts);
    ctx.insert("pagination", &pagination);

    let tera = state.templates.read().unwrap_or_else(|e| e.into_inner());
    match render_template(&tera, "index.html", &ctx) {
        Ok(html) => Ok(html.into_response()),
        Err((status, msg)) => Ok((status, msg).into_response()),
    }
}

// ── Page: post by slug ──

pub async fn post_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Response, (StatusCode, String)> {
    let config = services::theme::load_site_config(&state).await;

    let row: Option<PostRow> = sqlx::query_as(
        "SELECT * FROM posts WHERE slug = ? AND status = 'published'",
    )
    .bind(&slug)
    .fetch_optional(&state.db)
    .await
    .unwrap_or(None);

    let post_row = match row {
        Some(r) => r,
        None => return not_found(&state).await,
    };

    let post = services::theme::post_row_to_frontend(post_row);
    let active_theme = state.active_theme.read().unwrap_or_else(|e| e.into_inner()).clone();
    let global = build_global_context(config, &active_theme, &format!("/post/{}", slug));

    let mut ctx = tera::Context::new();
    ctx.insert("global", &global);
    ctx.insert("post", &post);

    let tera = state.templates.read().unwrap_or_else(|e| e.into_inner());
    match render_template(&tera, "post.html", &ctx) {
        Ok(html) => Ok(html.into_response()),
        Err((status, msg)) => Ok((status, msg).into_response()),
    }
}

// ── Page: category ──

pub async fn category_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(pq): Query<PageQuery>,
) -> Result<Response, (StatusCode, String)> {
    let config = services::theme::load_site_config(&state).await;
    let per_page = config.posts_per_page as usize;
    let page = pq.page.unwrap_or(1).max(1);

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM posts WHERE category = ? AND status = 'published'",
    )
    .bind(&name)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let offset = (page - 1) * per_page;
    let rows: Vec<PostRow> = sqlx::query_as(
        "SELECT * FROM posts WHERE category = ? AND status = 'published' ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&name)
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let posts: Vec<_> = rows
        .into_iter()
        .map(services::theme::post_row_to_frontend)
        .collect();

    let pagination = services::theme::calculate_pagination(page, total as usize, per_page);
    let active_theme = state.active_theme.read().unwrap_or_else(|e| e.into_inner()).clone();
    let global =
        build_global_context(config, &active_theme, &format!("/category/{}", name));

    let mut ctx = tera::Context::new();
    ctx.insert("global", &global);
    ctx.insert("current_category", &name);
    ctx.insert("posts", &posts);
    ctx.insert("pagination", &pagination);
    ctx.insert("total_posts", &total);

    let tera = state.templates.read().unwrap_or_else(|e| e.into_inner());
    match render_template(&tera, "category.html", &ctx) {
        Ok(html) => Ok(html.into_response()),
        Err((status, msg)) => Ok((status, msg).into_response()),
    }
}

// ── Page: tag ──

pub async fn tag_page(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Query(pq): Query<PageQuery>,
) -> Result<Response, (StatusCode, String)> {
    let config = services::theme::load_site_config(&state).await;
    let per_page = config.posts_per_page as usize;
    let page = pq.page.unwrap_or(1).max(1);

    let total: i64 = sqlx::query_scalar(
        "SELECT COUNT(DISTINCT p.id) FROM posts p
         JOIN post_tags pt ON p.id = pt.post_id
         WHERE pt.tag = ? AND p.status = 'published'",
    )
    .bind(&name)
    .fetch_one(&state.db)
    .await
    .unwrap_or(0);

    let offset = (page - 1) * per_page;
    let rows: Vec<PostRow> = sqlx::query_as(
        "SELECT p.* FROM posts p
         JOIN post_tags pt ON p.id = pt.post_id
         WHERE pt.tag = ? AND p.status = 'published'
         ORDER BY p.created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(&name)
    .bind(per_page as i64)
    .bind(offset as i64)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let posts: Vec<_> = rows
        .into_iter()
        .map(services::theme::post_row_to_frontend)
        .collect();

    let pagination = services::theme::calculate_pagination(page, total as usize, per_page);
    let active_theme = state.active_theme.read().unwrap_or_else(|e| e.into_inner()).clone();
    let global = build_global_context(config, &active_theme, &format!("/tag/{}", name));

    let mut ctx = tera::Context::new();
    ctx.insert("global", &global);
    ctx.insert("current_tag", &name);
    ctx.insert("posts", &posts);
    ctx.insert("pagination", &pagination);
    ctx.insert("total_posts", &total);

    let tera = state.templates.read().unwrap_or_else(|e| e.into_inner());
    match render_template(&tera, "tag.html", &ctx) {
        Ok(html) => Ok(html.into_response()),
        Err((status, msg)) => Ok((status, msg).into_response()),
    }
}

// ── Page: search ──

pub async fn search_page(
    State(state): State<AppState>,
    Query(params): Query<SearchPageQuery>,
) -> Result<Response, (StatusCode, String)> {
    let config = services::theme::load_site_config(&state).await;
    let per_page = config.posts_per_page as usize;
    let page = params.page.unwrap_or(1).max(1);
    let query = params.q.unwrap_or_default().trim().to_string();

    let (posts, total) = if query.is_empty() {
        (vec![], 0)
    } else {
        let (rows, total) =
            services::execute_search(&state.db, &query, page, per_page, true).await;
        let posts: Vec<Post> = rows
            .into_iter()
            .map(|r| {
                let tags: Vec<String> =
                    serde_json::from_str(&r.tags).unwrap_or_default();
                Post {
                    title: r.title,
                    slug: r.slug,
                    category: r.category,
                    tags,
                    content: services::markdown::render_markdown(&r.content),
                    created_at: r.created_at,
                    updated_at: r.updated_at,
                    snippet: r.snippet,
                }
            })
            .collect();
        (posts, total)
    };

    let pagination =
        services::theme::calculate_pagination(page, total as usize, per_page);

    let active_theme = state.active_theme.read().unwrap_or_else(|e| e.into_inner()).clone();
    let query_encoded = query
        .replace('%', "%25")
        .replace('&', "%26")
        .replace('#', "%23")
        .replace('+', "%2B")
        .replace(' ', "+");
    let global =
        build_global_context(config, &active_theme, &format!("/search?q={}", query_encoded));

    let mut ctx = tera::Context::new();
    ctx.insert("global", &global);
    ctx.insert("query", &query);
    ctx.insert("posts", &posts);
    ctx.insert("pagination", &pagination);
    ctx.insert("total_results", &total);

    let tera = state.templates.read().unwrap_or_else(|e| e.into_inner());
    match render_template(&tera, "search.html", &ctx) {
        Ok(html) => Ok(html.into_response()),
        Err((status, msg)) => Ok((status, msg).into_response()),
    }
}

// ── 404 ──

async fn not_found(state: &AppState) -> Result<Response, (StatusCode, String)> {
    let config = services::theme::load_site_config(&state).await;
    let active_theme = state.active_theme.read().unwrap_or_else(|e| e.into_inner()).clone();
    let global = build_global_context(config, &active_theme, "");

    let mut ctx = tera::Context::new();
    ctx.insert("global", &global);

    let tera = state.templates.read().unwrap_or_else(|e| e.into_inner());
    match tera.render("404.html", &ctx) {
        Ok(html) => Ok((StatusCode::NOT_FOUND, Html(html)).into_response()),
        Err(_) => {
            // Fallback to built-in 404 page
            Ok((
                StatusCode::NOT_FOUND,
                Html(
                    "<!DOCTYPE html><html><head><meta charset=\"UTF-8\"><title>404 - Not Found</title></head>\
                     <body style=\"display:flex;align-items:center;justify-content:center;min-height:100vh;font-family:sans-serif;\">\
                     <div style=\"text-align:center;\"><h1 style=\"font-size:4rem;margin:0;\">404</h1>\
                     <p>Page Not Found</p><p><a href=\"/\">Go home</a></p></div></body></html>"
                        .to_string(),
                ),
            )
                .into_response())
        }
    }
}

pub async fn handle_404(State(state): State<AppState>) -> Response {
    match not_found(&state).await {
        Ok(resp) => resp,
        Err((status, msg)) => (status, msg).into_response(),
    }
}
