//! Embedded frontend static file serving.
//!
//! Currently serves a committed placeholder page (`frontend-assets/index.html`)
//! because the dashboard is not yet feature-ready enough to bundle into the
//! binary. To re-bundle the real dashboard:
//!   1. set `#[folder = "../apps/dashboard/out/"]` again, AND
//!   2. add a CI step that runs `next build` (in `apps/dashboard`) before
//!      `cargo build`, so the export exists in every fresh checkout.
//!
//! Falls back to `index.html` for SPA client-side routing.

use axum::body::Body;
use axum::http::{header, StatusCode};
use axum::response::Response;
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "frontend-assets/"]
struct FrontendAssets;

/// Serve a static file from the embedded frontend assets.
/// Falls back to `index.html` for SPA routing.
pub async fn serve_static(path: &str) -> Response {
    let normalized = path.trim_start_matches('/');

    let file_path = if normalized.is_empty() {
        "index.html"
    } else {
        normalized
    };

    // Try exact path first
    if let Some(content) = FrontendAssets::get(file_path) {
        let mime = content.metadata.mimetype();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .body(Body::from(content.data.to_vec()))
            .unwrap();
    }

    // SPA fallback: serve index.html for client-side routing
    if let Some(content) = FrontendAssets::get("index.html") {
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
            .body(Body::from(content.data.to_vec()))
            .unwrap();
    }

    Response::builder()
        .status(StatusCode::NOT_FOUND)
        .body(Body::from("Not found"))
        .unwrap()
}

/// Axum handler that extracts the path from the request URI
pub async fn handle_frontend(req: axum::extract::Request) -> Response {
    let path = req.uri().path().to_string();
    serve_static(&path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_assets_contain_index() {
        assert!(
            FrontendAssets::get("index.html").is_some(),
            "index.html should be embedded in the binary"
        );
    }

    #[tokio::test]
    async fn test_serve_static_index() {
        let response = serve_static("/").await;
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_serve_static_spa_fallback() {
        let response = serve_static("/some/unknown/route").await;
        assert_eq!(response.status(), StatusCode::OK);
    }
}
