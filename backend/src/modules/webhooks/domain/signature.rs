use hmac::{Hmac, KeyInit, Mac};
use secrecy::{ExposeSecret, SecretString};
use sha2::Sha256;

use crate::shared_kernel::error::AppError;

type HmacSha256 = Hmac<Sha256>;

/// GitHub sends the digest hex-encoded behind this marker.
const PREFIX: &str = "sha256=";

/// Verifies `X-Hub-Signature-256` against the raw request body.
///
/// The body must be the exact bytes GitHub sent. Deserializing and re-encoding
/// would change whitespace and key order and the digest with it, which is why
/// the route takes `Bytes` and this runs before anything is parsed.
///
/// This is the *only* thing standing between the public internet and commands
/// that delete installations. There is no session, no `Origin` check, and no
/// rate limit on that route — by design (ARCHITECTURE.md §5.2) — so a weakness
/// here is not defence in depth, it is the whole defence.
pub(in crate::modules::webhooks) fn verify(
    body: &[u8],
    header: Option<&str>,
    secret: &SecretString,
) -> Result<(), AppError> {
    let Some(header) = header else {
        tracing::warn!("webhook arrived with no signature header");
        return Err(AppError::Unauthorized);
    };

    let Some(hex_digest) = header.strip_prefix(PREFIX) else {
        tracing::warn!("webhook signature is not sha256-prefixed");
        return Err(AppError::Unauthorized);
    };

    let expected = decode_hex(hex_digest).ok_or_else(|| {
        tracing::warn!("webhook signature is not valid hex");
        AppError::Unauthorized
    })?;

    // Keying with any length cannot fail for HMAC; the Result is part of the
    // trait rather than a real outcome.
    let mut mac = HmacSha256::new_from_slice(secret.expose_secret().as_bytes())
        .map_err(AppError::internal)?;
    mac.update(body);

    // `verify_slice` compares in constant time. A byte-by-byte `==` here would
    // leak how much of a forged digest was right, which is enough to reconstruct
    // one signature byte at a time.
    mac.verify_slice(&expected).map_err(|_| {
        tracing::warn!("webhook signature did not match");
        AppError::Unauthorized
    })
}

/// `None` on anything that is not an even-length run of hex digits.
fn decode_hex(input: &str) -> Option<Vec<u8>> {
    if !input.len().is_multiple_of(2) {
        return None;
    }

    (0..input.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(input.get(i..i + 2)?, 16).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Digest produced independently of this code, so the test pins the wire
    /// format rather than agreeing with whatever `verify` happens to compute.
    const BODY: &[u8] = br#"{"action":"created"}"#;
    const SECRET: &str = "It's a Secret to Everybody";

    fn secret() -> SecretString {
        SecretString::from(SECRET)
    }

    fn valid_header() -> String {
        // `expect` is denied crate-wide, tests included, so the failure is
        // spelled out rather than borrowed from the Result.
        let Ok(mut mac) = HmacSha256::new_from_slice(SECRET.as_bytes()) else {
            panic!("HMAC rejected a key, which cannot happen for any length");
        };
        mac.update(BODY);
        format!("sha256={}", hex(&mac.finalize().into_bytes()))
    }

    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn a_matching_signature_is_accepted() {
        assert!(verify(BODY, Some(&valid_header()), &secret()).is_ok());
    }

    #[test]
    fn a_tampered_body_is_rejected() {
        let header = valid_header();
        assert!(verify(br#"{"action":"deleted"}"#, Some(&header), &secret()).is_err());
    }

    #[test]
    fn the_wrong_secret_is_rejected() {
        let header = valid_header();
        let other = SecretString::from("not the secret");
        assert!(verify(BODY, Some(&header), &other).is_err());
    }

    /// The one that matters most: no header at all must not be a pass. An
    /// unsigned request is exactly what an attacker sends.
    #[test]
    fn a_missing_signature_is_rejected() {
        assert!(verify(BODY, None, &secret()).is_err());
    }

    #[test]
    fn a_malformed_signature_is_rejected() {
        for header in ["", "sha256=", "sha256=zz", "deadbeef", "sha256=abc"] {
            assert!(
                verify(BODY, Some(header), &secret()).is_err(),
                "accepted {header:?}"
            );
        }
    }

    /// A digest of the right shape but the wrong value — the case a
    /// length-only check would wave through.
    #[test]
    fn a_well_formed_but_wrong_digest_is_rejected() {
        let header = format!("sha256={}", "ab".repeat(32));
        assert!(verify(BODY, Some(&header), &secret()).is_err());
    }
}
