// modules/installations/infrastructure/installation_sync_throttle.rs
//
// One gate, so a dashboard that polls cannot turn into a GitHub call per poll.
//
// `useRepos` re-fetches every five seconds while anything is indexing. Without
// this, each of those would re-read the installation from GitHub: correct, and
// roughly 720 requests an hour per installation for an answer that changes when
// a human edits a setting.

use deadpool_redis::redis::cmd;

use crate::shared_kernel::{error::AppError, types::InstallationId};

/// How long a successful sync suppresses the next one. Short enough that
/// changing a setting on GitHub and returning to the dashboard shows it, long
/// enough that a polling tab costs one request a minute rather than twelve.
const WINDOW_SECS: u64 = 60;

fn throttle_key(installation_id: InstallationId) -> String {
    format!("installation_sync:{}", installation_id.0)
}

/// `true` when the caller may sync now, and claims the window by saying so.
///
/// `SET NX EX` is the whole mechanism: the first caller in a window creates the
/// key and proceeds, everyone else sees it exists and skips. Atomic, so two
/// concurrent dashboard loads cannot both decide they are first.
///
/// Fails **closed** — a Valkey error yields `false`, meaning "do not sync".
/// The opposite would remove the only thing bounding this call rate at the exact
/// moment we cannot measure it, and the cost of skipping is a dashboard that is
/// briefly as stale as it already was. In practice this is unreachable: sessions
/// live in Valkey too, so a request that got this far has already read one.
pub(in crate::modules::installations) async fn claim(
    installation_id: InstallationId,
    valkey: &deadpool_redis::Pool,
) -> bool {
    match try_claim(installation_id, valkey).await {
        Ok(claimed) => claimed,
        Err(error) => {
            tracing::warn!(%error, "sync throttle unreadable — skipping the refresh");
            false
        }
    }
}

async fn try_claim(
    installation_id: InstallationId,
    valkey: &deadpool_redis::Pool,
) -> Result<bool, AppError> {
    let mut conn = valkey.get().await.map_err(AppError::internal)?;

    // `SET .. NX EX` replies with OK or nil; nil means someone else holds it.
    let claimed: Option<String> = cmd("SET")
        .arg(throttle_key(installation_id))
        .arg(1)
        .arg("NX")
        .arg("EX")
        .arg(WINDOW_SECS)
        .query_async(&mut conn)
        .await
        .map_err(AppError::internal)?;

    Ok(claimed.is_some())
}

/// Resets the window without asking whether it was free.
///
/// For a sync that is happening regardless — a user pressing Refresh — so that
/// the polling that follows is still throttled. Without this, a forced refresh
/// would leave the window open and the next poll seconds later would sync again.
pub(in crate::modules::installations) async fn renew(
    installation_id: InstallationId,
    valkey: &deadpool_redis::Pool,
) {
    let result = async {
        let mut conn = valkey.get().await.map_err(AppError::internal)?;
        cmd("SET")
            .arg(throttle_key(installation_id))
            .arg(1)
            .arg("EX")
            .arg(WINDOW_SECS)
            .query_async::<()>(&mut conn)
            .await
            .map_err(AppError::internal)
    }
    .await;

    if let Err(error) = result {
        tracing::warn!(%error, "could not reset the sync throttle");
    }
}
