use axum::{
    Json, Router,
    extract::{Query, State},
    routing::get,
};
use serde::{Deserialize, Serialize};

use crate::{
    infrastructure::{AppContext, http::middleware::CurrentUser},
    modules::installations::{
        ListUserRepositoriesQuery, RefreshUserInstallationsCommand, UserRepositoryResponse,
        handle_list_user_repositories, handle_refresh_user_installations,
    },
    shared_kernel::error::AppError,
};

/// Mounted behind `require_session` — a repository list is a statement about
/// who is asking.
pub fn protected_routes() -> Router<AppContext> {
    Router::new().route("/repositories", get(list_repositories))
}

/// The dashboard row. Two modules' data in one shape: identity and visibility
/// from `installations`, index state from `indexing`.
///
/// `repository_id` is a string because that is what the frontend's `RepoSummary`
/// declares and what it puts in URLs. GitHub's ids are comfortably inside
/// JavaScript's safe integer range today, and sending them as text means they
/// still are on the day they are not.
#[derive(Debug, Serialize)]
struct RepoSummary {
    repository_id: String,
    owner_login: String,
    name: String,
    full_name: String,
    is_private: bool,
    default_branch: String,
    current_index_status: &'static str,
    last_indexed_commit_sha: Option<String>,
    last_indexed_at: Option<String>,
}

/// Until `indexing` exists there is nothing to ask it, and `never_indexed` is
/// not a placeholder — it is the true answer for every repository here.
///
/// When the module lands, this is where its status query gets zipped in by
/// repository id. The literal stays in the composition root rather than moving
/// into `installations`, because index state is not that module's to report.
const NEVER_INDEXED: &str = "never_indexed";

/// Refreshes before it reads, then reads.
///
/// Two calls into one module rather than one, because the module may not
/// orchestrate itself and this route may (ARCHITECTURE.md §5.1). The refresh is
/// throttled inside the command, so this is not a GitHub request per poll.
///
/// A failed refresh is logged and ignored: the stored access is still the best
/// answer available, and a GitHub outage should not blank someone's dashboard.
async fn list_repositories(
    State(ctx): State<AppContext>,
    Query(params): Query<ListParams>,
    user: CurrentUser,
) -> Result<Json<Vec<RepoSummary>>, AppError> {
    match handle_refresh_user_installations(
        RefreshUserInstallationsCommand {
            user_id: user.user_id,
            force: params.force(),
        },
        &ctx,
    )
    .await
    {
        Ok(refreshed) => tracing::debug!(
            synced = refreshed.synced,
            skipped = refreshed.skipped,
            "grants refreshed before listing"
        ),
        Err(error) => tracing::warn!(%error, "refresh failed; serving stored access"),
    }

    let repositories = handle_list_user_repositories(
        ListUserRepositoriesQuery {
            user_id: user.user_id,
        },
        &ctx,
    )
    .await?;

    Ok(Json(repositories.into_iter().map(summarise).collect()))
}

fn summarise(repo: UserRepositoryResponse) -> RepoSummary {
    RepoSummary {
        repository_id: repo.repository_id.0.to_string(),
        owner_login: repo.owner_login,
        name: repo.name,
        full_name: repo.full_name.0,
        is_private: repo.is_private,
        default_branch: repo.default_branch,
        current_index_status: NEVER_INDEXED,
        last_indexed_commit_sha: None,
        last_indexed_at: None,
    }
}

/// `?refresh=1` marks a deliberate refresh and bypasses the sync throttle. Its
/// absence is the normal case — a page load or a poll, both happy with an
/// answer up to a window old.
///
/// A `String` and not a `bool`: axum's query deserializer accepts only the
/// literals `true` and `false`, so `Option<bool>` answers `400` to
/// `?refresh=1`. A read endpoint refusing to answer because a flag said `1`
/// is the wrong trade, so the parsing is done here and is forgiving.
#[derive(Debug, Deserialize)]
struct ListParams {
    refresh: Option<String>,
}

impl ListParams {
    /// `?refresh=1`, `?refresh=true`, and a bare `?refresh` all mean yes.
    /// Anything else — including `0` and `false` — means no.
    fn force(&self) -> bool {
        matches!(self.refresh.as_deref(), Some("1" | "true" | ""))
    }
}
