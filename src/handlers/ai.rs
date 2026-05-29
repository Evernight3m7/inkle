use axum::{extract::State, http::StatusCode, Json};
use serde_json::Value;
use std::net::SocketAddr;

use axum::extract::ConnectInfo;

use crate::handlers::auth::AuthUser;
use crate::models::{AiGenerateRequest, AiGenerateResponse, ErrorResponse};
use crate::services;
use crate::state::AppState;

const MAX_CONTENT_LENGTH: usize = 50_000;

pub async fn generate_summary(
    _auth: AuthUser,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    State(state): State<AppState>,
    Json(body): Json<AiGenerateRequest>,
) -> Result<Json<AiGenerateResponse>, (StatusCode, Json<ErrorResponse>)> {
    let content = body.content.trim().to_string();
    if content.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Content is empty".into(),
            }),
        ));
    }
    if content.len() > MAX_CONTENT_LENGTH {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: format!("Content too long (max {} characters)", MAX_CONTENT_LENGTH),
            }),
        ));
    }

    // Rate limit: 10 requests per 60 seconds per IP
    {
        let mut limiter = state.ai_rate_limiter.lock().await;
        let now = std::time::Instant::now();
        let ip = addr.ip().to_string();

        // Clean up expired entries
        limiter.retain(|_, (_, timestamp)| {
            now.duration_since(*timestamp).as_secs() < 60
        });

        let entry = limiter.get(&ip).map(|(c, t)| (*c, *t));
        if let Some((count, time)) = entry {
            let elapsed = now.duration_since(time).as_secs();
            if elapsed < 60 && count >= 10 {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ErrorResponse {
                        error: "Too many requests. Please wait before generating another summary.".into(),
                    }),
                ));
            }
            if elapsed >= 60 {
                limiter.insert(ip, (1, now));
            } else {
                limiter.insert(ip, (count + 1, time));
            }
        } else {
            limiter.insert(ip, (1, now));
        }
    }

    let config = services::theme::load_site_config(&state).await;

    if config.ai_base_url.is_empty() || config.ai_api_key.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "AI_CONFIG_INCOMPLETE: Please configure AI Base URL and API Key in settings.".into(),
            }),
        ));
    }

    let prompt = config.ai_prompt.replacen("{content}", &content, 1);

    let endpoint = format!("{}/chat/completions", config.ai_base_url.trim().trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build HTTP client: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(ErrorResponse {
                    error: "Failed to initialize HTTP client".into(),
                }),
            )
        })?;

    let request_body = serde_json::json!({
        "model": config.ai_model,
        "messages": [
            {"role": "user", "content": prompt}
        ],
        "max_tokens": 300,
        "temperature": 0.3
    });

    let res = client
        .post(&endpoint)
        .header("Authorization", format!("Bearer {}", config.ai_api_key))
        .header("Content-Type", "application/json")
        .json(&request_body)
        .send()
        .await
        .map_err(|e| {
            tracing::error!("AI API request failed: {e}");
            (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: format!("Failed to reach AI API: {e}"),
                }),
            )
        })?;

    let status = res.status();
    let body_text = res.text().await.unwrap_or_default();

    if !status.is_success() {
        tracing::error!("AI API returned {status}: {body_text}");
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: format!("AI API error ({}): {}", status.as_u16(), body_text),
            }),
        ));
    }

    let parsed: Value = serde_json::from_str(&body_text).map_err(|e| {
        tracing::error!("Failed to parse AI API response: {e}");
        (
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "Failed to parse AI API response".into(),
            }),
        )
    })?;

    let choices = parsed["choices"].as_array().and_then(|arr| {
        if arr.is_empty() {
            None
        } else {
            Some(arr)
        }
    });

    let summary = match choices {
        Some(arr) => arr[0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .trim()
            .to_string(),
        None => {
            tracing::error!("AI API returned no choices: {body_text}");
            return Err((
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: "AI API returned an unexpected response format".into(),
                }),
            ));
        }
    };

    if summary.is_empty() {
        return Err((
            StatusCode::BAD_GATEWAY,
            Json(ErrorResponse {
                error: "AI returned an empty summary".into(),
            }),
        ));
    }

    Ok(Json(AiGenerateResponse { summary }))
}
