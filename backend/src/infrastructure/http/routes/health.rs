use axum::{Json, Router, extract::State, routing::get};
use serde::Serialize;

use crate::infrastructure::AppContext;

pub fn routes() -> Router<AppContext> {
    Router::new().route("/health", get(liveness))
}

#[derive(Serialize)]
struct LivenessResponse {
    status: &'static str,
    version: &'static str,
    uptime_seconds: u64,
}

async fn liveness(State(ctx): State<AppContext>) -> Json<LivenessResponse> {
    Json(LivenessResponse {
        status: "ok",
        version: env!("CARGO_PKG_VERSION"),
        uptime_seconds: ctx.started_at.elapsed().as_secs(),
    })
}
