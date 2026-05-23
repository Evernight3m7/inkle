use std::str::FromStr;

use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;

/// Check if char is a word character (alphanumeric or underscore).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Split SQL content into individual statements, respecting BEGIN...END blocks
/// so internal semicolons inside trigger bodies don't break parsing.
/// Uses word-boundary checks to avoid false positives on "SEND", "FRIEND", etc.
fn split_sql_statements(sql: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut depth: usize = 0;

    for ch in sql.chars() {
        current.push(ch);

        if ch == ';' && depth == 0 {
            let trimmed = current.trim().to_string();
            let without_trailing_semi = trimmed.strip_suffix(';').unwrap_or(&trimmed);
            if !without_trailing_semi.is_empty() {
                statements.push(without_trailing_semi.to_string());
            }
            current.clear();
        }

        // Track BEGIN keyword (case-insensitive, word boundary)
        if word_at_end(&current, "BEGIN") {
            depth += 1;
        }
        // Track END keyword (case-insensitive, word boundary)
        if depth > 0 && word_at_end(&current, "END") {
            depth -= 1;
        }
    }

    let trimmed = current.trim().to_string();
    let without_trailing_semi = trimmed.strip_suffix(';').unwrap_or(&trimmed);
    if !without_trailing_semi.is_empty() {
        statements.push(without_trailing_semi.to_string());
    }

    statements
}

/// Check if `word` appears at the end of `s` with a word boundary before it.
fn word_at_end(s: &str, word: &str) -> bool {
    if s.len() < word.len() {
        return false;
    }
    let suffix = &s[s.len() - word.len()..];
    if !suffix.eq_ignore_ascii_case(word) {
        return false;
    }
    // Check word boundary: char before `word` must not be a word character
    if s.len() > word.len() {
        let preceding = s.as_bytes()[s.len() - word.len() - 1] as char;
        if is_word_char(preceding) {
            return false;
        }
    }
    true
}

/// Run all SQL migration files from `migrations/` in sorted order.
/// Tracks executed migrations in a `_migrations` table to ensure idempotent restarts.
async fn run_migrations(pool: &SqlitePool) -> Result<(), Box<dyn std::error::Error>> {
    sqlx::query(
        "CREATE TABLE IF NOT EXISTS _migrations (filename TEXT PRIMARY KEY, executed_at TEXT NOT NULL DEFAULT (datetime('now')))"
    ).execute(pool).await?;

    let mut entries: Vec<_> = std::fs::read_dir("migrations")?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map_or(false, |ext| ext == "sql"))
        .collect();

    entries.sort_by_key(|e| e.file_name());

    for entry in &entries {
        let filename = entry.file_name().to_string_lossy().to_string();

        // Skip already-executed migrations
        let already_run: bool = sqlx::query_scalar(
            "SELECT COUNT(*) > 0 FROM _migrations WHERE filename = ?"
        )
        .bind(&filename)
        .fetch_one(pool)
        .await?;

        if already_run {
            tracing::info!("Skipping already executed: {filename}");
            continue;
        }

        let sql = std::fs::read_to_string(entry.path())?;

        for statement in split_sql_statements(&sql) {
            sqlx::query(&statement).execute(pool).await?;
        }

        sqlx::query("INSERT INTO _migrations (filename) VALUES (?)")
            .bind(&filename)
            .execute(pool)
            .await?;

        tracing::info!("Executed migration: {filename}");
    }

    Ok(())
}

/// Initialize the database connection pool and run migrations.
pub async fn init_db(database_url: &str) -> Result<SqlitePool, Box<dyn std::error::Error>> {
    let options = SqliteConnectOptions::from_str(database_url)?
        .foreign_keys(true)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await?;

    run_migrations(&pool).await?;

    tracing::info!("Database initialized successfully");
    Ok(pool)
}
