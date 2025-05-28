use axum::{
    Router,
    extract::Request,
    http::{Uri, header},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

/// The bundled frontend files are embedded into this struct.
#[derive(Embed)]
#[folder = "frontend/dist"]
struct Assets;

const INDEX_HTML: &str = "index.html";

async fn static_handler(uri: Uri, req: Request, next: Next) -> impl IntoResponse {
    let path = uri.path().trim_start_matches('/');

    if path.is_empty() || path == INDEX_HTML {
        return index_html().await;
    }

    match Assets::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();

            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            if path.contains('.') {
                return next.run(req).await;
            }

            index_html().await
        }
    }
}

async fn index_html() -> Response {
    Html(Assets::get(INDEX_HTML).unwrap().data).into_response()
}

/// Mount this router under `/remote`. If you want to use a different path, you must
/// update the base path of the frontend in `frontend/vite.config.ts`.
pub fn router<S>() -> Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    Router::new().layer(middleware::from_fn(static_handler))
}
