//! The built UI, optionally compiled into the server binary.
//!
//! There are three ways the server can serve the web app, in the order the
//! startup path resolves them:
//!
//! 1. `SCANOPY_WEB_EXTERNAL_PATH` is set — serve that directory from disk.
//!    This is what the Docker image does (`/app/static`), and it stays the
//!    override for anyone who wants to swap the UI without a rebuild.
//! 2. The binary was built with `--features embed-ui` — serve the copy
//!    compiled in. This is what makes the standalone release artifact a
//!    single file with nothing beside it.
//! 3. Neither — API-only.
//!
//! This module covers (2). With the feature off, [`is_embedded`] is false and
//! [`index_html`] is `None`, so callers fall through to (3) exactly as before.
//!
//! Responses here deliberately set only `Content-Type`. Cache-control and the
//! security headers come from the layers wrapping the whole router, so
//! embedded and on-disk serving stay identical in everything but the source
//! of the bytes.

#[cfg(feature = "embed-ui")]
mod embedded {
    use axum::http::{Uri, header};
    use axum::response::{IntoResponse, Response};
    use rust_embed::Embed;

    /// The SvelteKit build output. Compiled in only under `embed-ui` — the
    /// folder exists only after a UI build, so an unconditional embed would
    /// break `cargo build` for anyone who hasn't run one.
    #[derive(Embed)]
    #[folder = "../ui/build"]
    struct WebAssets;

    const INDEX: &str = "index.html";

    pub fn index_html() -> Option<String> {
        let file = WebAssets::get(INDEX)?;
        String::from_utf8(file.data.into_owned()).ok()
    }

    /// Serve an embedded file, falling back to `index.html` for anything not
    /// found — the SPA owns client-side routing, which is the same behavior
    /// `ServeDir(...).fallback(ServeFile::new(index.html))` gives on the
    /// external-path branch.
    pub async fn handler(uri: Uri) -> Response {
        let path = uri.path().trim_start_matches('/');

        // A bare `/` or a directory path resolves to that directory's
        // index.html, matching `append_index_html_on_directories`.
        let candidate = if path.is_empty() || path.ends_with('/') {
            format!("{path}{INDEX}")
        } else {
            path.to_string()
        };

        serve(&candidate)
            .or_else(|| serve(INDEX))
            .unwrap_or_else(|| {
                // Only reachable if the build had no index.html, which the
                // startup check rules out before this handler is mounted.
                (axum::http::StatusCode::NOT_FOUND, "Not found").into_response()
            })
    }

    fn serve(path: &str) -> Option<Response> {
        let file = WebAssets::get(path)?;
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        Some(
            (
                [(header::CONTENT_TYPE, mime.as_ref().to_string())],
                file.data.into_owned(),
            )
                .into_response(),
        )
    }
}

/// Whether this binary carries a compiled-in UI.
pub fn is_embedded() -> bool {
    cfg!(feature = "embed-ui")
}

/// The compiled-in `index.html`, or `None` when no UI is embedded.
///
/// The share routes serve this document themselves so a per-share CSP applies
/// to it, rather than letting it fall through to the router fallback.
pub fn index_html() -> Option<String> {
    #[cfg(feature = "embed-ui")]
    {
        embedded::index_html()
    }
    #[cfg(not(feature = "embed-ui"))]
    {
        None
    }
}

/// Router fallback that serves the compiled-in UI.
///
/// Only mount this when [`is_embedded`] is true; without the feature it
/// answers 404 for everything.
pub async fn fallback_handler(uri: axum::http::Uri) -> axum::response::Response {
    #[cfg(feature = "embed-ui")]
    {
        embedded::handler(uri).await
    }
    #[cfg(not(feature = "embed-ui"))]
    {
        let _ = uri;
        use axum::response::IntoResponse;
        (axum::http::StatusCode::NOT_FOUND, "Not found").into_response()
    }
}
