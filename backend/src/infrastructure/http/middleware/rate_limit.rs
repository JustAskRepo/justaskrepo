// infrastructure/http/middleware/rate_limit.rs
//
// The HTTP adapter over infrastructure/rate_limiter.rs: work out who the caller
// is, spend one unit of a budget, and turn a refusal into an AppError. All the
// counting lives in the limiter; all the status-code knowledge lives in
// infrastructure/http/error.rs. This file is the seam between them.

use std::net::SocketAddr;

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};

use crate::{
    infrastructure::{
        AppContext,
        http::client_ip::{client_ip, rate_limit_subject},
        rate_limiter::{self, RateLimitDecision, RateLimitPolicy},
    },
    shared_kernel::error::AppError,
};

#[derive(Clone)]
pub struct RateLimitState {
    ctx: AppContext,
    policy: RateLimitPolicy,
}

impl RateLimitState {
    pub fn new(ctx: AppContext, policy: RateLimitPolicy) -> Self {
        Self { ctx, policy }
    }
}

pub async fn rate_limit(
    State(state): State<RateLimitState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let peer = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|ConnectInfo(addr)| *addr)
        .ok_or_else(|| {
            AppError::internal(anyhow::anyhow!(
                "rate_limit layer ran without ConnectInfo — serve the router with \
                 into_make_service_with_connect_info::<SocketAddr>()"
            ))
        })?;

    let client = client_ip(request.headers(), peer, state.ctx.trusted_proxy_hops);
    let subject = rate_limit_subject(client);

    match rate_limiter::check(&state.policy, &subject, &state.ctx.valkey).await {
        RateLimitDecision::Allowed => Ok(next.run(request).await),
        RateLimitDecision::Exceeded { retry_after_secs } => {
            tracing::warn!(
                policy = state.policy.name,
                %subject,
                path = %request.uri().path(),
                "rate limit exceeded"
            );
            Err(AppError::RateLimited { retry_after_secs })
        }
    }
}
