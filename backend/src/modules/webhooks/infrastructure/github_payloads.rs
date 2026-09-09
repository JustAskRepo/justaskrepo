use serde::Deserialize;

use crate::shared_kernel::{error::AppError, types::InstallationId};

/// The only part of any `installation*` payload this module reads.
///
/// Everything else GitHub sends is ignored on purpose: the sync command
/// re-fetches the truth rather than trusting a delivery that may be a
/// duplicate, out of order, or both. Parsing less is also less to break when
/// GitHub adds a field.
#[derive(Deserialize)]
struct InstallationEnvelope {
    action: String,
    installation: InstallationRef,
}

#[derive(Deserialize)]
struct InstallationRef {
    id: i64,
}

/// What the dispatcher needs to know: which installation, and whether it still
/// exists.
#[derive(Debug, PartialEq, Eq)]
pub(in crate::modules::webhooks) enum InstallationChange {
    /// Re-read it from GitHub and replace what we hold.
    Sync(InstallationId),
    /// It is gone; nothing can be fetched.
    Removed(InstallationId),
}

/// Maps `(event, action)` onto the two things `installations` can be asked to do.
///
/// `None` means "understood, nothing to do" — not an error. A webhook we choose
/// to ignore must still be answered with a 2xx, or GitHub will retry it forever.
pub(in crate::modules::webhooks) fn parse(
    event: &str,
    body: &[u8],
) -> Result<Option<InstallationChange>, AppError> {
    // Both event types carry the same `installation.id`, which is the only
    // field either branch needs.
    if event != "installation" && event != "installation_repositories" {
        return Ok(None);
    }

    let envelope: InstallationEnvelope = serde_json::from_slice(body).map_err(|error| {
        tracing::warn!(%error, %event, "webhook body did not parse");
        AppError::Validation("unparseable webhook payload".to_owned())
    })?;

    let id = InstallationId(envelope.installation.id);

    let change = match (event, envelope.action.as_str()) {
        ("installation", "deleted") => Some(InstallationChange::Removed(id)),
        // Everything else that can change a grant collapses to one action,
        // because after a re-fetch they are indistinguishable: created,
        // suspended, unsuspended, permissions accepted, repositories added or
        // removed. `installation.suspend` is included on purpose — suspension
        // is a state to record, never a delete.
        ("installation", "created" | "suspend" | "unsuspend" | "new_permissions_accepted")
        | ("installation_repositories", "added" | "removed") => Some(InstallationChange::Sync(id)),
        (_, action) => {
            tracing::debug!(%event, %action, "webhook action not acted on");
            None
        }
    };

    Ok(change)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(action: &str) -> Vec<u8> {
        format!(r#"{{"action":"{action}","installation":{{"id":42}}}}"#).into_bytes()
    }

    /// `AppError` has no `PartialEq`, so the Result is unwrapped here rather
    /// than compared.
    fn parsed(event: &str, action: &str) -> Option<InstallationChange> {
        match parse(event, &body(action)) {
            Ok(change) => change,
            Err(error) => panic!("{event}.{action} did not parse: {error}"),
        }
    }

    #[test]
    fn deletion_is_the_only_action_that_removes() {
        assert_eq!(
            parsed("installation", "deleted"),
            Some(InstallationChange::Removed(InstallationId(42)))
        );
    }

    #[test]
    fn every_other_grant_change_syncs() {
        let expected = Some(InstallationChange::Sync(InstallationId(42)));

        for action in [
            "created",
            "suspend",
            "unsuspend",
            "new_permissions_accepted",
        ] {
            assert_eq!(
                parsed("installation", action),
                expected,
                "installation.{action}"
            );
        }
        for action in ["added", "removed"] {
            assert_eq!(
                parsed("installation_repositories", action),
                expected,
                "installation_repositories.{action}"
            );
        }
    }

    /// An event we do not handle is understood, not rejected — the route turns
    /// this into a 2xx so GitHub stops retrying it.
    #[test]
    fn unhandled_events_and_actions_are_not_errors() {
        assert_eq!(parsed("push", "created"), None);
        assert_eq!(parsed("installation", "some_future_action"), None);
    }

    #[test]
    fn a_body_without_an_installation_is_rejected() {
        assert!(parse("installation", br#"{"action":"created"}"#).is_err());
    }
}
