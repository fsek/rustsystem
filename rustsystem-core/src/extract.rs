//! Drop-in replacements for axum's `Json` and `Path` extractors whose rejections are
//! [`ApiError`]s, so malformed requests get the same `{code, message}` body as every other error.

use axum::extract::FromRequest;
use axum::extract::FromRequestParts;
use axum::response::{IntoResponse, Response};

use crate::ApiError;

#[derive(FromRequest)]
#[from_request(via(axum::Json), rejection(ApiError))]
pub struct Json<T>(pub T);

impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

#[derive(FromRequestParts)]
#[from_request(via(axum::extract::Path), rejection(ApiError))]
pub struct Path<T>(pub T);
