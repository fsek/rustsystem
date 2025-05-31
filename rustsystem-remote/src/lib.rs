use axum::{
    Router,
    extract::Path,
    http::{StatusCode, header},
    response::{Html, IntoResponse, Response},
    routing::get,
};
use rust_embed::Embed;

/// The bundled frontend files are embedded into this struct.
#[derive(Embed)]
#[folder = "frontend/dist"]
struct Assets;

const INDEX_HTML: &str = "index.html";

async fn static_handler(Path(path): Path<String>) -> Response {
    let path = path.trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    match Assets::get(&path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return StatusCode::NOT_FOUND.into_response();
            }

            index_html().await
        }
    }
}

async fn index_html() -> Response {
    Html(Assets::get(INDEX_HTML).unwrap().data).into_response()
}

pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    // If you change the base path here, you must also change the base path of
    // the frontend in `frontend/vite.config.ts`, or the frontend won't work
    // correctly.
    Router::new()
        .route("/remote", get(index_html))
        .route("/remote{*_}", get(static_handler))
}
