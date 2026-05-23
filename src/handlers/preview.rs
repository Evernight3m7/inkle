use axum::Json;

use crate::models::{PreviewRequest, PreviewResponse};

pub async fn preview(Json(body): Json<PreviewRequest>) -> Json<PreviewResponse> {
    let html = crate::services::markdown::render_markdown(&body.markdown);
    Json(PreviewResponse { html })
}
