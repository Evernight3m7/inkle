mod config;
mod db;
mod handlers;
mod models;
mod services;
mod state;

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, RwLock};

use axum::{routing::get, Router};

use state::AppState;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let env = config::EnvConfig::from_env();

    let pool = db::init_db(&env.database_url)
        .await
        .expect("Failed to initialize database");

    // Load active theme from database
    let active_theme: String = sqlx::query_scalar(
        "SELECT value FROM configs WHERE key = 'active_theme'",
    )
    .fetch_optional(&pool)
    .await
    .unwrap_or(None)
    .unwrap_or_else(|| "default".to_string());

    // Load Tera for the active theme
    let tera = services::theme::load_tera(&active_theme);

    // Load admin Tera (separate instance, never changes at runtime)
    let admin_tera = match tera::Tera::new("admin_templates/**/*") {
        Ok(t) => {
            tracing::info!("Loaded admin templates");
            t
        }
        Err(e) => {
            tracing::warn!("Failed to load admin templates: {e}");
            tera::Tera::default()
        }
    };

    let state = AppState {
        db: pool,
        env,
        templates: Arc::new(RwLock::new(tera)),
        active_theme: Arc::new(RwLock::new(active_theme)),
        admin_templates: admin_tera,
        login_rate_limiter: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        site_config_cache: Arc::new(RwLock::new(None)),
        theme_list_cache: Arc::new(RwLock::new(None)),
    };

    // ── Public API routes ──

    let public_routes = Router::new()
        .route("/api/health", get(health))
        .route("/api/login", axum::routing::post(handlers::auth::login))
        .route("/api/logout", axum::routing::post(handlers::auth::logout))
        .route("/api/preview", axum::routing::post(handlers::preview::preview))
        .route("/api/search", get(handlers::search::search));

    // ── Admin API routes ──

    let admin_routes = Router::new()
        .route(
            "/api/admin/posts",
            get(handlers::admin_post::list).post(handlers::admin_post::create),
        )
        .route(
            "/api/admin/posts/{id}",
            get(handlers::admin_post::get)
                .put(handlers::admin_post::update)
                .delete(handlers::admin_post::delete),
        )
        .route(
            "/api/admin/settings",
            get(handlers::settings::get_settings).put(handlers::settings::update_settings),
        )
        .route("/api/admin/stats", get(handlers::settings::get_stats));

    // ── Theme admin routes ──

    let theme_admin_routes = Router::new()
        .route("/api/admin/themes", get(handlers::theme::list_themes))
        .route(
            "/api/admin/themes/{name}/activate",
            axum::routing::post(handlers::theme::activate_theme),
        );

    // ── Admin HTML page routes (Step 4) ──

    let admin_page_routes = Router::new()
        .route("/admin/login", get(handlers::admin_pages::login_page))
        .route("/admin", get(handlers::admin_pages::dashboard))
        .route(
            "/admin/posts/new",
            get(handlers::admin_pages::new_post),
        )
        .route(
            "/admin/posts/{id}/edit",
            get(handlers::admin_pages::edit_post),
        )
        .route(
            "/admin/settings",
            get(handlers::admin_pages::settings_page),
        )
        .route(
            "/admin/themes",
            get(handlers::admin_pages::themes_page),
        )
        .route(
            "/static/admin/{*path}",
            get(handlers::admin_pages::serve_admin_static),
        );

    // ── Frontend routes (theme-rendered pages) ──

    let frontend_routes = Router::new()
        .route("/", get(handlers::frontend::index))
        .route("/post/{slug}", get(handlers::frontend::post_by_slug))
        .route("/category/{name}", get(handlers::frontend::category_page))
        .route("/tag/{name}", get(handlers::frontend::tag_page))
        .route("/search", get(handlers::frontend::search_page))
        .route("/static/theme/{*path}", get(handlers::frontend::serve_static))
        .fallback(handlers::frontend::handle_404);

    let app = Router::new()
        .merge(public_routes)
        .merge(admin_routes)
        .merge(theme_admin_routes)
        .merge(admin_page_routes)
        .merge(frontend_routes)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    tracing::info!("Listening on {addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}

async fn health() -> &'static str {
    "OK"
}
