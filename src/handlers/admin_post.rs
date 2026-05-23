use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::handlers::auth::AuthUser;
use crate::models::{
    AdminPost, CreatePostRequest, ErrorResponse, PostListResponse, PostRow, SinglePostResponse,
    UpdatePostRequest,
};
use crate::state::AppState;

// ── Helpers ──

fn slugify(title: &str) -> String {
    let mut slug = String::new();
    for ch in title.to_lowercase().chars() {
        if ch.is_alphanumeric() {
            slug.push(ch);
        } else if !slug.is_empty() && !slug.ends_with('-') {
            slug.push('-');
        }
    }
    slug.trim_matches('-').to_string()
}

#[derive(Deserialize)]
pub struct PaginationQuery {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

// ── Handlers ──

pub async fn list(
    _auth: AuthUser,
    State(state): State<AppState>,
    Query(pq): Query<PaginationQuery>,
) -> Result<Json<PostListResponse>, (StatusCode, Json<ErrorResponse>)> {
    let page = pq.page.unwrap_or(1).max(1);
    let per_page = pq.per_page.unwrap_or(10).max(1).min(100);
    let offset = (page - 1).saturating_mul(per_page);

    // Get total count
    let (total,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM posts")
        .fetch_one(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("{e}"),
                }),
            )
        })?;

    let rows: Vec<PostRow> = sqlx::query_as(
        "SELECT * FROM posts ORDER BY created_at DESC LIMIT ? OFFSET ?",
    )
    .bind(per_page)
    .bind(offset)
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("{e}"),
            }),
        )
    })?;

    let posts: Vec<AdminPost> = rows.into_iter().map(AdminPost::from).collect();

    Ok(Json(PostListResponse {
        posts,
        total,
        page,
        per_page,
    }))
}

pub async fn get(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<SinglePostResponse>, (StatusCode, Json<ErrorResponse>)> {
    let row: PostRow = sqlx::query_as("SELECT * FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("{e}"),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "post not found".into(),
                }),
            )
        })?;

    Ok(Json(SinglePostResponse {
        post: AdminPost::from(row),
    }))
}

pub async fn create(
    _auth: AuthUser,
    State(state): State<AppState>,
    Json(body): Json<CreatePostRequest>,
) -> Result<(StatusCode, Json<SinglePostResponse>), (StatusCode, Json<ErrorResponse>)> {
    let slug = body
        .slug
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| {
            let s = slugify(&body.title);
            if s.is_empty() {
                format!(
                    "post-{}",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                )
            } else {
                s
            }
        });

    let category = body.category.unwrap_or_default();
    let tags = serde_json::to_string(&body.tags.unwrap_or_default()).unwrap_or_default();
    let status = body.status.unwrap_or_else(|| "draft".into());

    let row: PostRow = sqlx::query_as(
        "INSERT INTO posts (title, slug, category, tags, content, status) VALUES (?, ?, ?, ?, ?, ?) RETURNING *",
    )
    .bind(&body.title)
    .bind(&slug)
    .bind(&category)
    .bind(&tags)
    .bind(&body.content)
    .bind(&status)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        let msg = if is_unique_constraint(&e) {
            format!("slug '{slug}' already exists")
        } else {
            format!("{e}")
        };
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: msg }),
        )
    })?;

    // Sync post_tags junction table
    if let Ok(tag_list) = serde_json::from_str::<Vec<String>>(&tags) {
        for tag in &tag_list {
            let _ = sqlx::query("INSERT OR IGNORE INTO post_tags (post_id, tag) VALUES (?, ?)")
                .bind(row.id)
                .bind(tag)
                .execute(&state.db)
                .await;
        }
    }

    Ok((StatusCode::CREATED, Json(SinglePostResponse {
        post: AdminPost::from(row),
    })))
}

/// Check if a sqlx error is a SQLite UNIQUE constraint violation.
fn is_unique_constraint(e: &sqlx::Error) -> bool {
    if let sqlx::Error::Database(db_err) = e {
        db_err.code().as_deref() == Some("2067")
    } else {
        false
    }
}

pub async fn update(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(body): Json<UpdatePostRequest>,
) -> Result<Json<SinglePostResponse>, (StatusCode, Json<ErrorResponse>)> {
    // Fetch existing post
    let existing: PostRow = sqlx::query_as("SELECT * FROM posts WHERE id = ?")
        .bind(id)
        .fetch_optional(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("{e}"),
                }),
            )
        })?
        .ok_or_else(|| {
            (
                StatusCode::NOT_FOUND,
                Json(ErrorResponse {
                    error: "post not found".into(),
                }),
            )
        })?;

    let title = body.title.unwrap_or(existing.title);
    let slug = body.slug.unwrap_or(existing.slug);
    let category = body.category.unwrap_or(existing.category);
    let tags = body
        .tags
        .map(|t| serde_json::to_string(&t).unwrap_or_default())
        .unwrap_or(existing.tags);
    let content = body.content.unwrap_or(existing.content);
    let status = body.status.unwrap_or(existing.status);

    let row: PostRow = sqlx::query_as(
        "UPDATE posts SET title=?, slug=?, category=?, tags=?, content=?, status=?, updated_at=datetime('now') WHERE id=? RETURNING *",
    )
    .bind(&title)
    .bind(&slug)
    .bind(&category)
    .bind(&tags)
    .bind(&content)
    .bind(&status)
    .bind(id)
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        let msg = if is_unique_constraint(&e) {
            format!("slug '{slug}' already exists")
        } else {
            format!("{e}")
        };
        (
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: msg }),
        )
    })?;

    // Sync post_tags junction table: delete old tags and re-insert
    let _ = sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
        .bind(id)
        .execute(&state.db)
        .await;
    if let Ok(tag_list) = serde_json::from_str::<Vec<String>>(&tags) {
        for tag in &tag_list {
            let _ = sqlx::query("INSERT OR IGNORE INTO post_tags (post_id, tag) VALUES (?, ?)")
                .bind(id)
                .bind(tag)
                .execute(&state.db)
                .await;
        }
    }

    Ok(Json(SinglePostResponse {
        post: AdminPost::from(row),
    }))
}

pub async fn delete(
    _auth: AuthUser,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<StatusCode, (StatusCode, Json<ErrorResponse>)> {
    // Clean up post_tags first (defense in depth alongside FK CASCADE)
    let _ = sqlx::query("DELETE FROM post_tags WHERE post_id = ?")
        .bind(id)
        .execute(&state.db)
        .await;

    let result = sqlx::query("DELETE FROM posts WHERE id = ?")
        .bind(id)
        .execute(&state.db)
        .await
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: format!("{e}"),
                }),
            )
        })?;

    if result.rows_affected() == 0 {
        return Err((
            StatusCode::NOT_FOUND,
            Json(ErrorResponse {
                error: "post not found".into(),
            }),
        ));
    }

    Ok(StatusCode::NO_CONTENT)
}
