use axum::{
    Router,
    body::Body,
    http::{self, Request, Response},
};
use rustsystem_trustauth::{AppState, app_internal, init_state};
use tower::util::ServiceExt;

pub struct MockApp {
    router: Router,
    pub state: AppState,
}

impl MockApp {
    pub fn new_inprocess() -> Self {
        let state = init_state().expect("init_state failed");
        let router = app_internal(state.clone());
        Self { router, state }
    }

    pub async fn oneshot(&self, req: Request<Body>) -> Response<Body> {
        self.router.clone().oneshot(req).await.unwrap()
    }
}

pub fn json_request(
    method: http::Method,
    uri: &str,
    body: serde_json::Value,
) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header(http::header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&body).unwrap()))
        .unwrap()
}

pub async fn parse_response_body<T: serde::de::DeserializeOwned>(res: Response<Body>) -> T {
    use http_body_util::BodyExt;
    let bytes = res.into_body().collect().await.unwrap().to_bytes();
    serde_json::from_slice(&bytes).unwrap()
}
