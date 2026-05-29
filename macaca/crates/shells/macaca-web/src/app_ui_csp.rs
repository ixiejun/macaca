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
/// default-deny policy for scripts, network, and base navigation, then appends
/// only trusted shell origins to the frame ancestor set.
pub(crate) fn app_ui_html_csp(headers: &HeaderMap) -> String {
    let mut frame_ancestors = vec!["'self'".to_string()];

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

    let frame_ancestors = frame_ancestors.join(" ");
    tracing::debug!(
        frame_ancestors,
        "constructed application-owned UI frame ancestor policy"
    );

    format!(
        "default-src 'none'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self' data:; connect-src 'none'; base-uri 'none'; frame-ancestors {frame_ancestors}"
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
        headers.insert(
            header::REFERER,
            HeaderValue::from_static(
                "http://localhost:5173/chat/6fbb0369-e1c9-5a98-89b7-eb01f9c9fa93",
            ),
        );

        let csp = app_ui_html_csp(&headers);

        assert!(csp.contains("default-src 'none'"));
        assert!(csp.contains("connect-src 'none'"));
        assert!(csp.contains("frame-ancestors 'self'"));
        assert!(csp.contains("http://localhost:5173"));
    }

    #[test]
    fn app_ui_html_csp_rejects_remote_referer_origin() {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::REFERER,
            HeaderValue::from_static("https://example.com/host/page"),
        );

        let csp = app_ui_html_csp(&headers);

        assert!(!csp.contains("https://example.com"));
        assert!(csp.contains("http://localhost:3000"));
        assert!(csp.contains("http://localhost:8080"));
    }
}
