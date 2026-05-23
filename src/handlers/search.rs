use axum::{
    extract::{Query, State},
    Json,
};
use serde::Deserialize;

use crate::models::{SearchResponse, SearchResult};
use crate::services;
use crate::state::AppState;

#[derive(Deserialize)]
pub struct SearchQueryParam {
    pub q: Option<String>,
}

pub async fn search(
    State(state): State<AppState>,
    Query(params): Query<SearchQueryParam>,
) -> Json<SearchResponse> {
    let query = match params.q.as_deref() {
        Some(q) if !q.trim().is_empty() => q.trim(),
        _ => {
            return Json(SearchResponse {
                results: vec![],
                total: 0,
            });
        }
    };

    let (rows, total) = services::execute_search(&state.db, query, 1, 20, true).await;

    let results: Vec<SearchResult> = rows
        .into_iter()
        .map(|r| SearchResult {
            id: r.id,
            title: r.title,
            slug: r.slug,
            snippet: r.snippet.unwrap_or_default(),
        })
        .collect();

    Json(SearchResponse { results, total })
}
