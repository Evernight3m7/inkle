pub mod markdown;
pub mod theme;

use sqlx::SqlitePool;

use crate::models::SearchResultRow;

/// Sanitize a snippet HTML string by escaping all HTML except `<mark>` and `</mark>` tags.
/// FTS5's `snippet()` returns raw content with `<mark>` highlights inserted — but the
/// raw content may contain user-controlled HTML (e.g. `<script>` in Markdown). We escape
/// everything except the highlight markers we intentionally added.
fn sanitize_html_keep_marks(input: &str) -> String {
    const SENTINEL_OPEN: &str = "\x01MO\x01";
    const SENTINEL_CLOSE: &str = "\x01MC\x01";

    let with_sentinels = input
        .replace("</mark>", SENTINEL_CLOSE)
        .replace("<mark>", SENTINEL_OPEN);

    let escaped = with_sentinels
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;");

    escaped
        .replace(SENTINEL_OPEN, "<mark>")
        .replace(SENTINEL_CLOSE, "</mark>")
}

/// Sanitize raw user input for FTS5 query construction.
/// Keeps alphanumeric chars and whitespace.
/// CJK chars already satisfy `is_alphanumeric()` so no separate check is needed.
fn sanitize_query(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .trim()
        .to_string()
}

/// Build an FTS5 query string from sanitized input.
/// Each term gets a `*` suffix for prefix matching (terms >= 2 chars).
/// Returns None if no valid terms remain.
fn build_fts5_query(input: &str) -> Option<String> {
    let terms: Vec<String> = input
        .split_whitespace()
        .map(|w| {
            let cleaned: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            if cleaned.len() >= 2 {
                format!("{}*", cleaned)
            } else {
                cleaned
            }
        })
        .filter(|s| !s.is_empty())
        .collect();

    if terms.is_empty() {
        None
    } else {
        Some(terms.join(" "))
    }
}

/// Log a search query for analytics.
async fn log_search(db: &SqlitePool, query: &str, results_count: i64) {
    let _ = sqlx::query("INSERT INTO search_logs (query, results_count) VALUES (?, ?)")
        .bind(query)
        .bind(results_count)
        .execute(db)
        .await;
}

// ── FTS5 trigram search ──

async fn search_fts5(
    db: &SqlitePool,
    raw_query: &str,
    sanitized: &str,
    page: usize,
    per_page: usize,
    published_only: bool,
) -> (Vec<SearchResultRow>, i64) {
    let fts_query = match build_fts5_query(sanitized) {
        Some(q) => q,
        None => return (vec![], 0),
    };

    let offset = page.saturating_sub(1) * per_page;
    let status_filter = if published_only {
        " AND p.status = 'published'"
    } else {
        ""
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM posts_fts \
         JOIN posts p ON posts_fts.rowid = p.id \
         WHERE posts_fts MATCH ?{status_filter}"
    );
    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(&fts_query)
        .fetch_one(db)
        .await
        .unwrap_or(0);

    log_search(db, raw_query, total).await;

    let select_sql = format!(
        "SELECT p.id, p.title, p.slug, p.category, p.tags, p.content, \
                p.status, p.created_at, p.updated_at, \
                snippet(posts_fts, 1, '<mark>', '</mark>', '...', 48) as snippet \
         FROM posts_fts \
         JOIN posts p ON posts_fts.rowid = p.id \
         WHERE posts_fts MATCH ?{status_filter} \
         ORDER BY rank \
         LIMIT ? OFFSET ?"
    );
    let rows = match sqlx::query(&select_sql)
        .bind(&fts_query)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(db)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("FTS5 search failed: {e}");
            return (vec![], 0);
        }
    };

    let results: Vec<SearchResultRow> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            SearchResultRow {
                id: row.get(0),
                title: row.get(1),
                slug: row.get(2),
                category: row.get(3),
                tags: row.get(4),
                content: row.get(5),
                status: row.get(6),
                created_at: row.get(7),
                updated_at: row.get(8),
                snippet: {
                    let raw: String = row.get(9);
                    Some(sanitize_html_keep_marks(&raw))
                },
            }
        })
        .collect();

    (results, total)
}

// ── LIKE fallback for short queries (< 3 chars) that trigram can't match ──

fn escape_like(s: &str) -> String {
    s.replace('%', "\\%").replace('_', "\\_")
}

fn generate_snippet(content: &str, keyword: &str, ctx_len: usize) -> String {
    let pos = match content.to_lowercase().find(&keyword.to_lowercase()) {
        Some(p) => p,
        None => return String::new(),
    };

    let chars: Vec<char> = content.chars().collect();
    let total = chars.len();
    let char_pos = content[..pos].chars().count();
    let char_end = char_pos + keyword.chars().count();

    let ctx_start = char_pos.saturating_sub(ctx_len);
    let ctx_end = (char_end + ctx_len).min(total);

    let mut snippet = String::new();
    if ctx_start > 0 {
        snippet.push_str("...");
    }
    for &c in &chars[ctx_start..char_pos] {
        match c {
            '<' => snippet.push_str("&lt;"),
            '>' => snippet.push_str("&gt;"),
            '&' => snippet.push_str("&amp;"),
            _ => snippet.push(c),
        }
    }
    snippet.push_str("<mark>");
    for &c in &chars[char_pos..char_end] {
        match c {
            '<' => snippet.push_str("&lt;"),
            '>' => snippet.push_str("&gt;"),
            '&' => snippet.push_str("&amp;"),
            _ => snippet.push(c),
        }
    }
    snippet.push_str("</mark>");
    for &c in &chars[char_end..ctx_end] {
        match c {
            '<' => snippet.push_str("&lt;"),
            '>' => snippet.push_str("&gt;"),
            '&' => snippet.push_str("&amp;"),
            _ => snippet.push(c),
        }
    }
    if ctx_end < total {
        snippet.push_str("...");
    }

    snippet
}

async fn search_like(
    db: &SqlitePool,
    raw_query: &str,
    sanitized: &str,
    page: usize,
    per_page: usize,
    published_only: bool,
) -> (Vec<SearchResultRow>, i64) {
    let pattern = format!("%{}%", escape_like(sanitized));
    let offset = page.saturating_sub(1) * per_page;
    let status_filter = if published_only {
        " AND status = 'published'"
    } else {
        ""
    };

    let count_sql = format!(
        "SELECT COUNT(*) FROM posts \
         WHERE (title LIKE ? ESCAPE '\\' OR content LIKE ? ESCAPE '\\' \
                OR tags LIKE ? ESCAPE '\\' OR category LIKE ? ESCAPE '\\'){status_filter}"
    );
    let total: i64 = sqlx::query_scalar(&count_sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .fetch_one(db)
        .await
        .unwrap_or(0);

    log_search(db, raw_query, total).await;

    let select_sql = format!(
        "SELECT id, title, slug, category, tags, content, \
                status, created_at, updated_at \
         FROM posts \
         WHERE (title LIKE ? ESCAPE '\\' OR content LIKE ? ESCAPE '\\' \
                OR tags LIKE ? ESCAPE '\\' OR category LIKE ? ESCAPE '\\'){status_filter} \
         ORDER BY created_at DESC \
         LIMIT ? OFFSET ?"
    );
    let rows = match sqlx::query(&select_sql)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(&pattern)
        .bind(per_page as i64)
        .bind(offset as i64)
        .fetch_all(db)
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::error!("LIKE search failed: {e}");
            return (vec![], 0);
        }
    };

    let results: Vec<SearchResultRow> = rows
        .iter()
        .map(|row| {
            use sqlx::Row;
            let content: String = row.get(5);
            let snippet = generate_snippet(&content, sanitized, 24);
            SearchResultRow {
                id: row.get(0),
                title: row.get(1),
                slug: row.get(2),
                category: row.get(3),
                tags: row.get(4),
                content,
                status: row.get(6),
                created_at: row.get(7),
                updated_at: row.get(8),
                snippet: if snippet.is_empty() { None } else { Some(snippet) },
            }
        })
        .collect();

    (results, total)
}

// ── Public API ──

/// Unified search entry point. Used by both the API handler and the frontend handler.
///
/// Queries with >= 3 alphanumeric characters use FTS5 with trigram tokenizer.
/// Shorter queries (1-2 chars) fall back to LIKE because the trigram tokenizer
/// requires at least 3 characters to form a trigram.
///
/// When `published_only` is true, only posts with `status = 'published'` are returned.
/// When false, all posts (including drafts) are searched — useful for the admin dashboard.
pub async fn execute_search(
    db: &SqlitePool,
    raw_query: &str,
    page: usize,
    per_page: usize,
    published_only: bool,
) -> (Vec<SearchResultRow>, i64) {
    let sanitized = sanitize_query(raw_query);
    if sanitized.is_empty() {
        return (vec![], 0);
    }

    let char_count = sanitized.chars().filter(|c| c.is_alphanumeric()).count();
    if char_count < 3 {
        search_like(db, raw_query, &sanitized, page, per_page, published_only).await
    } else {
        search_fts5(db, raw_query, &sanitized, page, per_page, published_only).await
    }
}
