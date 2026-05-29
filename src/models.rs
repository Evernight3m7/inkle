use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Auth ──

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// ── Site Config ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiteConfig {
    pub site_title: String,
    pub site_subtitle: String,
    pub active_theme: String,
    pub posts_per_page: u32,
    pub social_links: HashMap<String, String>,
    #[serde(default)]
    pub ai_base_url: String,
    #[serde(default)]
    pub ai_api_key: String,
    #[serde(default = "default_ai_model")]
    pub ai_model: String,
    #[serde(default = "default_ai_prompt")]
    pub ai_prompt: String,
}

fn default_ai_model() -> String {
    "gpt-4o-mini".into()
}

fn default_ai_prompt() -> String {
    "请为以下文章生成一段100-150字的中文摘要，直接返回摘要内容，不要包含任何前缀或解释：\n\n{content}".into()
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub site_title: Option<String>,
    pub site_subtitle: Option<String>,
    pub active_theme: Option<String>,
    pub posts_per_page: Option<u32>,
    pub social_links: Option<HashMap<String, String>>,
    pub ai_base_url: Option<String>,
    pub ai_api_key: Option<String>,
    pub ai_model: Option<String>,
    pub ai_prompt: Option<String>,
}

// ── Posts ──

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct PostRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub category: String,
    pub tags: String, // JSON array string from DB
    pub content: String,
    #[serde(default)]
    pub r#abstract: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Flattened post for admin API responses (parsed tags, includes id + status).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminPost {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub category: String,
    pub tags: Vec<String>,
    pub content: String,
    pub r#abstract: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PostRow> for AdminPost {
    fn from(row: PostRow) -> Self {
        let tags: Vec<String> =
            serde_json::from_str(&row.tags).unwrap_or_default();
        Self {
            id: row.id,
            title: row.title,
            slug: row.slug,
            category: row.category,
            tags,
            content: row.content,
            r#abstract: row.r#abstract,
            status: row.status,
            created_at: row.created_at,
            updated_at: row.updated_at,
        }
    }
}

/// Frontend-facing post (rendered HTML, no id/status).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Post {
    pub title: String,
    pub slug: String,
    pub category: String,
    pub tags: Vec<String>,
    pub content: String, // rendered HTML
    pub r#abstract: String, // rendered HTML
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub snippet: Option<String>, // FTS5 highlighted snippet, only for search pages
}

#[derive(Debug, Deserialize)]
pub struct CreatePostRequest {
    pub title: String,
    pub slug: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub content: String,
    pub r#abstract: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdatePostRequest {
    pub title: Option<String>,
    pub slug: Option<String>,
    pub category: Option<String>,
    pub tags: Option<Vec<String>>,
    pub content: Option<String>,
    pub r#abstract: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct SinglePostResponse {
    pub post: AdminPost,
}

#[derive(Debug, Serialize)]
pub struct PostListResponse {
    pub posts: Vec<AdminPost>,
    pub total: i64,
    pub page: i64,
    pub per_page: i64,
}

// ── Search ──

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct SearchResult {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub snippet: String,
}

#[derive(Debug, Serialize)]
pub struct SearchResponse {
    pub results: Vec<SearchResult>,
    pub total: i64,
}

/// Internal search result row used by the unified search service.
/// Handlers map this to their own response types.
#[derive(Debug, Clone)]
pub struct SearchResultRow {
    pub id: i64,
    pub title: String,
    pub slug: String,
    pub category: String,
    pub tags: String,
    pub content: String,
    pub r#abstract: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub snippet: Option<String>,
}

// ── Markdown Preview ──

#[derive(Debug, Deserialize)]
pub struct PreviewRequest {
    pub markdown: String,
}

#[derive(Debug, Serialize)]
pub struct PreviewResponse {
    pub html: String,
}

// ── Stats ──

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CategoryStat {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct TagStat {
    pub name: String,
    pub count: i64,
}

#[derive(Debug, Serialize)]
pub struct StatsResponse {
    pub categories: Vec<CategoryStat>,
    pub tags: Vec<TagStat>,
}

// ── Shared context ──

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pagination {
    pub current: usize,
    pub total: usize,
    pub prev: Option<usize>,
    pub next: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalContext {
    pub config: SiteConfig,
    pub current_theme: String,
    pub current_url: String,
}

// ── AI ──

#[derive(Debug, Deserialize)]
pub struct AiGenerateRequest {
    pub content: String,
}

#[derive(Debug, Serialize)]
pub struct AiGenerateResponse {
    pub summary: String,
}
