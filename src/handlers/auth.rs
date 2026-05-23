use std::net::SocketAddr;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use axum::{
    extract::{ConnectInfo, FromRequestParts},
    http::{
        header::{COOKIE, SET_COOKIE},
        request::Parts,
        HeaderMap, StatusCode,
    },
    response::{IntoResponse, Response},
    Json,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::models::{ErrorResponse, LoginRequest};
use crate::state::AppState;

const TOKEN_NAME: &str = "token";

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: String,
    exp: usize,
    iat: usize,
}

/// Auth extractor — validates JWT cookie.
pub struct AuthUser;

impl FromRequestParts<AppState> for AuthUser {
    type Rejection = (StatusCode, Json<ErrorResponse>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = get_cookie(&parts.headers, TOKEN_NAME);

        match token {
            Some(t) => {
                let decoding = DecodingKey::from_secret(state.env.jwt_secret.as_bytes());
                let mut validation = Validation::default();
                validation.sub = Some("admin".into());
                match decode::<Claims>(&t, &decoding, &validation) {
                    Ok(_data) => Ok(Self),
                    Err(_) => Err((
                        StatusCode::UNAUTHORIZED,
                        Json(ErrorResponse {
                            error: "unauthorized".into(),
                        }),
                    )),
                }
            }
            None => Err((
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "unauthorized".into(),
                }),
            )),
        }
    }
}

/// Extract a cookie value by name from request headers.
pub(crate) fn get_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookie_header = headers.get(COOKIE)?.to_str().ok()?;
    for part in cookie_header.split(';') {
        let kv: Vec<&str> = part.trim().splitn(2, '=').collect();
        if kv.len() == 2 && kv[0].trim() == name {
            return Some(kv[1].trim().to_string());
        }
    }
    None
}

/// Build a Set-Cookie header value.
fn set_cookie(name: &str, value: &str, max_age: Option<&str>) -> String {
    let mut cookie = format!(
        "{name}={value}; HttpOnly; SameSite=Lax; Secure; Path=/"
    );
    if let Some(age) = max_age {
        cookie.push_str(&format!("; Max-Age={age}"));
    }
    cookie
}

fn clear_cookie(name: &str) -> String {
    format!("{name}=; HttpOnly; SameSite=Lax; Secure; Path=/; Max-Age=0")
}

const MAX_LOGIN_ATTEMPTS: u32 = 5;
const RATE_LIMIT_WINDOW_SECS: u64 = 60;

pub async fn login(
    axum::extract::State(state): axum::extract::State<AppState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(body): Json<LoginRequest>,
) -> Response {
    let ip = addr.ip().to_string();

    // ── Rate limiting ──
    {
        let mut rate_limiter = state.login_rate_limiter.lock().await;
        let now = Instant::now();

        // Clean up expired entries
        rate_limiter.retain(|_, (_, timestamp)| {
            now.duration_since(*timestamp).as_secs() < RATE_LIMIT_WINDOW_SECS
        });

        if let Some((attempts, window_start)) = rate_limiter.get(&ip) {
            if *attempts >= MAX_LOGIN_ATTEMPTS
                && now.duration_since(*window_start).as_secs() < RATE_LIMIT_WINDOW_SECS
            {
                return (
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ErrorResponse {
                        error: "too_many_requests".into(),
                    }),
                )
                    .into_response();
            }
        }
    }

    // ── Password validation ──
    if body.password != state.env.admin_password {
        // Record failed attempt
        {
            let mut rate_limiter = state.login_rate_limiter.lock().await;
            let now = Instant::now();
            let entry = rate_limiter.entry(ip.clone()).or_insert((0, now));
            entry.0 += 1;
        }

        return (
            StatusCode::UNAUTHORIZED,
            Json(ErrorResponse {
                error: "invalid_password".into(),
            }),
        )
            .into_response();
    }

    // ── Clear rate limit on success ──
    {
        let mut rate_limiter = state.login_rate_limiter.lock().await;
        rate_limiter.remove(&ip);
    }

    // ── Generate JWT ──
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as usize;

    let claims = Claims {
        sub: "admin".into(),
        iat: now,
        exp: now + 86400, // 24 hours
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.env.jwt_secret.as_bytes()),
    )
    .unwrap();

    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        set_cookie(TOKEN_NAME, &token, Some("86400"))
            .parse()
            .unwrap(),
    );
    response
}

pub async fn logout() -> Response {
    let mut response = StatusCode::NO_CONTENT.into_response();
    response.headers_mut().insert(
        SET_COOKIE,
        clear_cookie(TOKEN_NAME).parse().unwrap(),
    );
    response
}

// ── AuthPage: redirect-based auth extractor for admin HTML pages ──

use axum::response::Redirect;

/// Auth extractor for admin HTML pages — redirects to /admin/login instead
/// of returning a JSON 401 response.
pub struct AuthPage;

impl FromRequestParts<AppState> for AuthPage {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let token = get_cookie(&parts.headers, TOKEN_NAME);

        match token {
            Some(t) => {
                let decoding = DecodingKey::from_secret(state.env.jwt_secret.as_bytes());
                let mut validation = Validation::default();
                validation.sub = Some("admin".into());
                match decode::<Claims>(&t, &decoding, &validation) {
                    Ok(_) => Ok(Self),
                    Err(_) => Err(Redirect::to("/admin/login").into_response()),
                }
            }
            None => Err(Redirect::to("/admin/login").into_response()),
        }
    }
}
