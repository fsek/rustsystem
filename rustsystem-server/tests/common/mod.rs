use axum::{
    Router,
    body::Body,
    http::{self, HeaderValue, Request, Response},
};
use rustsystem_server::app;
use tower::util::ServiceExt;

pub struct MockApp {
    router: Router,
}
impl MockApp {
    pub fn new_inprocess() -> Self {
        let router = app();

        Self { router: router }
    }

    pub async fn oneshot(&self, req: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(req).await.unwrap()
    }
}

pub fn json_request(
    method: http::Method,
    uri: &str,
    body: serde_json::Value,
    cookie_val: Option<HeaderValue>,
) -> Request<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json");

    builder = if let Some(cookie_val) = cookie_val {
        builder.header(http::header::COOKIE, cookie_val)
    } else {
        builder
    };

    builder
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}
