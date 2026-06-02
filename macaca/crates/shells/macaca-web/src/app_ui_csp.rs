//! Content Security Policy helpers for application-owned UI assets.
//!
//! Asset routing and bridge dispatch stay in `app_ui_routes`; this module owns
//! the CSP construction policy so the route file remains small and the
//! frame-ancestor admission rule is independently testable.

use axum::http::{header, HeaderMap};

/// Build the strict CSP used for application-owned HTML entry documents.
///
/// The asset route serves UI bundles from the API origin while the presentation
/// shell can run on a separate local development origin. `frame-ancestors`
/// therefore cannot be a fixed compile-time port list. This helper keeps the
/// default-deny policy for scripts and base navigation, then appends only
/// trusted shell origins to the frame ancestor set.  Application-owned UIs also
/// need a narrow realtime channel back to the same API origin for generic
/// execution replay/subscribe routes.  The connect policy therefore admits
/// only `'self'` plus loopback WebSocket variants derived from the request Host
/// header; it never opens arbitrary remote network access or application-owned
/// provider endpoints.
pub(crate) fn app_ui_html_csp(headers: &HeaderMap) -> String {
    let mut frame_ancestors = vec!["'self'".to_string()];
    let mut connect_src = vec!["'self'".to_string()];

    for origin in [
        "http://localhost:3000",
        "http://127.0.0.1:3000",
        "http://localhost:8080",
        "http://127.0.0.1:8080",
    ] {
        push_unique(&mut frame_ancestors, origin.to_string());
    }

    for header_name in [header::ORIGIN, header::REFERER] {
        if let Some(origin) = headers
            .get(header_name)
            .and_then(|value| value.to_str().ok())
            .and_then(trusted_loopback_origin_from_header)
        {
            push_unique(&mut frame_ancestors, origin);
        }
    }

    if let Some(authority) = headers
        .get(header::HOST)
        .and_then(|value| value.to_str().ok())
        .and_then(trusted_loopback_authority)
    {
        // WebSocket URL matching is not consistently covered by `'self'`
        // across browser/CSP combinations, so explicitly admit ws/wss for the
        // same trusted loopback authority that served the HTML entrypoint. This
        // is still provider-neutral: the policy is scoped to transport origin,
        // not to an application name, route, model, or business workflow.
        push_unique(&mut connect_src, format!("ws://{authority}"));
        push_unique(&mut connect_src, format!("wss://{authority}"));
    }

    let frame_ancestors = frame_ancestors.join(" ");
    let connect_src = connect_src.join(" ");
    tracing::debug!(
        frame_ancestors,
        connect_src,
        "constructed application-owned UI frame ancestor policy"
    );

    format!(
        "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src {connect_src}; base-uri 'none'; frame-ancestors {frame_ancestors}"
    )
}

/// Extract a CSP-safe origin from an `Origin` or `Referer` header.
fn trusted_loopback_origin_from_header(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let (scheme, rest) = trimmed.split_once("://")?;
    if !matches!(scheme, "http" | "https") {
        return None;
    }
    let authority = rest.split(&['/', '?', '#'][..]).next().unwrap_or("").trim();
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let host = authority
        .rsplit_once(':')
        .map(|(host, _port)| host)
        .unwrap_or(authority);
    let host = host.trim_matches(['[', ']']);
    if matches!(host, "localhost" | "127.0.0.1" | "::1") {
        Some(format!("{scheme}://{authority}"))
    } else {
        None
    }
}

/// Extract a CSP-safe loopback authority from a Host header.
///
/// Host is intentionally parsed separately from Origin/Referer because it does
/// not contain a scheme.  The result is an authority only, which callers can
/// combine with tightly scoped schemes such as `ws` and `wss`.  Rejecting user
/// info, paths, queries, fragments, and non-loopback hosts prevents this helper
/// from turning a spoofed Host header into an open `connect-src` allowance.
fn trusted_loopback_authority(value: &str) -> Option<String> {
    let authority = value
        .trim()
        .split(&['/', '?', '#'][..])
        .next()
        .unwrap_or("")
        .trim();
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    let host = authority
        .rsplit_once(':')
        .map(|(host, _port)| host)
        .unwrap_or(authority);
    let host = host.trim_matches(['[', ']']);
    if matches!(host, "localhost" | "127.0.0.1" | "::1") {
        Some(authority.to_string())
    } else {
        None
    }
}

/// Add a value to an ordered CSP token list without producing duplicates.
fn push_unique(values: &mut Vec<String>, value: String) {
    if !values.iter().any(|existing| existing == &value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use axum::http::HeaderValue;

    use super::*;

    #[test]
    fn app_ui_html_csp_allows_current_loopback_shell_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("localhost:3127"));
        headers.insert(
            header::REFERER,
            HeaderValue::from_static(
                "http://localhost:5173/chat/6fbb0369-e1c9-5a98-89b7-eb01f9c9fa93",
            ),
        );

        let csp = app_ui_html_csp(&headers);

        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("connect-src 'self'"));
        assert!(csp.contains("ws://localhost:3127"));
        assert!(csp.contains("wss://localhost:3127"));
        assert!(csp.contains("frame-ancestors 'self'"));
        assert!(csp.contains("http://localhost:5173"));
    }

    #[test]
    fn app_ui_html_csp_rejects_remote_referer_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("api.example.com"));
        headers.insert(
            header::REFERER,
            HeaderValue::from_static("https://example.com/host/page"),
        );

        let csp = app_ui_html_csp(&headers);

        assert!(!csp.contains("https://example.com"));
        assert!(!csp.contains("ws://api.example.com"));
        assert!(csp.contains("http://localhost:3000"));
        assert!(csp.contains("http://localhost:8080"));
    }
}
