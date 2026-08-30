// infrastructure/http/client_ip.rs
//
// Who to hold a request against.
//
// `X-Forwarded-For` is a client-writable header. Anyone can send one, so a
// rate limiter keyed on whatever it says is not a limiter at all — an attacker
// rotates the value per request and gets an unlimited budget. The only address
// nobody can forge is the socket peer, and the only forwarded entries worth
// trusting are the ones a proxy *we* control appended.
//
// That is a fact about the deployment, not about the request, so it is
// configuration: TRUSTED_PROXY_HOPS says how many proxies sit in front of this
// process. It defaults to 0 (trust nothing, use the peer) because guessing
// wrong in that direction costs accuracy, and guessing wrong in the other
// direction hands out an unlimited budget.

use std::net::{IpAddr, SocketAddr};

use axum::http::HeaderMap;

pub fn client_ip(headers: &HeaderMap, peer: SocketAddr, trusted_hops: u8) -> IpAddr {
    if trusted_hops == 0 {
        return peer.ip();
    }

    let mut chain: Vec<IpAddr> = headers
        .get_all("x-forwarded-for")
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .filter_map(|entry| parse_ip(entry.trim()))
        .collect();
    chain.push(peer.ip());

    match chain.len().checked_sub(1 + usize::from(trusted_hops)) {
        Some(index) => chain.get(index).copied().unwrap_or_else(|| peer.ip()),

        None => {
            tracing::warn!(
                trusted_hops,
                observed = chain.len(),
                "TRUSTED_PROXY_HOPS exceeds the observed forwarding chain — \
                 falling back to the peer address"
            );
            peer.ip()
        }
    }
}

fn parse_ip(entry: &str) -> Option<IpAddr> {
    if let Ok(ip) = entry.parse::<IpAddr>() {
        return Some(ip);
    }
    if let Ok(addr) = entry.parse::<SocketAddr>() {
        return Some(addr.ip());
    }
    entry
        .strip_prefix('[')
        .and_then(|rest| rest.split(']').next())
        .and_then(|inner| inner.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    const PEER: &str = "203.0.113.9:44001";

    fn peer() -> SocketAddr {
        PEER.parse()
            .unwrap_or_else(|_| SocketAddr::from(([0; 4], 0)))
    }

    fn headers(forwarded: &[&str]) -> HeaderMap {
        let mut headers = HeaderMap::new();
        for value in forwarded {
            if let Ok(value) = HeaderValue::from_str(value) {
                headers.append("x-forwarded-for", value);
            }
        }
        headers
    }

    fn ip(text: &str) -> IpAddr {
        text.parse().unwrap_or(IpAddr::from([0, 0, 0, 0]))
    }

    #[test]
    fn with_no_trusted_proxies_the_header_is_ignored_entirely() {
        assert_eq!(
            client_ip(&headers(&["1.2.3.4"]), peer(), 0),
            ip("203.0.113.9")
        );
    }

    #[test]
    fn one_trusted_proxy_yields_the_address_that_proxy_saw() {
        assert_eq!(
            client_ip(&headers(&["198.51.100.7"]), peer(), 1),
            ip("198.51.100.7")
        );
    }

    /// The whole point. The attacker prepends a forged entry; our proxy appends
    /// what it actually observed; one hop back skips the forgery.
    #[test]
    fn a_forged_prefix_cannot_shift_the_bucket() {
        let forged = headers(&["9.9.9.9, 8.8.8.8, 198.51.100.7"]);
        assert_eq!(client_ip(&forged, peer(), 1), ip("198.51.100.7"));
    }

    /// A CDN in front of a reverse proxy: two appended entries, so the client
    /// is two hops back from the peer.
    #[test]
    fn two_trusted_proxies_step_two_entries_back() {
        let chain = headers(&["9.9.9.9, 198.51.100.7, 192.0.2.50"]);
        assert_eq!(client_ip(&chain, peer(), 2), ip("198.51.100.7"));
    }

    /// Proxies are free to split the chain across repeated headers instead of
    /// one comma-joined value.
    #[test]
    fn the_chain_may_arrive_as_repeated_headers() {
        let split = headers(&["9.9.9.9", "198.51.100.7"]);
        assert_eq!(client_ip(&split, peer(), 1), ip("198.51.100.7"));
    }

    /// Misconfiguration falls back to the peer, never to the forgeable end of
    /// the chain — otherwise setting the number too high would silently hand
    /// every caller an unlimited budget.
    #[test]
    fn more_hops_than_the_chain_holds_falls_back_to_the_peer() {
        assert_eq!(
            client_ip(&headers(&["1.2.3.4"]), peer(), 4),
            ip("203.0.113.9")
        );
        assert_eq!(client_ip(&headers(&[]), peer(), 1), ip("203.0.113.9"));
    }

    #[test]
    fn ports_and_ipv6_forms_all_parse() {
        assert_eq!(parse_ip("1.2.3.4"), Some(ip("1.2.3.4")));
        assert_eq!(parse_ip("1.2.3.4:5678"), Some(ip("1.2.3.4")));
        assert_eq!(parse_ip("2001:db8::1"), Some(ip("2001:db8::1")));
        assert_eq!(parse_ip("[2001:db8::1]:5678"), Some(ip("2001:db8::1")));
        assert_eq!(parse_ip("[2001:db8::1]"), Some(ip("2001:db8::1")));
        assert_eq!(parse_ip("unknown"), None);
    }

    /// Unparseable entries are dropped. Counting from the right is what makes
    /// that safe: a trusted proxy always appends a real address, so junk can
    /// only ever appear to the left of the entry we select.
    #[test]
    fn junk_entries_are_dropped_not_counted() {
        let noisy = headers(&["_hidden, 198.51.100.7"]);
        assert_eq!(client_ip(&noisy, peer(), 1), ip("198.51.100.7"));
    }
}
