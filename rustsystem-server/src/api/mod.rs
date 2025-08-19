use crate::{AppState, tokens::AuthUser};
use axum::{
    Json, Router,
    extract::{FromRequest, State},
    handler::Handler,
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{MethodRouter, get, post},
};
mod create_meeting;
use axum_extra::extract::CookieJar;
use create_meeting::{CreateMeetingQuery, create_meeting};

mod login;
use login::login;

mod auth;
use auth::AuthMeeting;

mod voter;
use serde::{Deserialize, Serialize};
use voter::voter_routes;

mod host;
use host::host_routes;

mod common;
use common::common_routes;

// Routes at /api/...
pub fn api_routes() -> Router<AppState> {
    Router::new()
        .route("/create-meeting", post(create_meeting))
        .route("/auth-meeting", post(AuthMeeting::handler))
        .route("/login", post(login))
        .nest("/host", host_routes())
        .nest("/voter", voter_routes())
        .nest("/common", common_routes())
}

/// The response may be a failure type which is equally valid as it pertains to the API structure.
/// Helper functions may use the APIResult type as a return type such that the "?" operator can be
/// used to send the error all the way to the response
type APIResult<T, E> = Result<T, (StatusCode, E)>;

/// Similar to APIResult, but also requires that the success type includes a [`StatusCode`].
/// If `T` and `E` are the same in `APIResult` as they are in `APIResponse`, the "?" operator can
/// still be used, just like for any other result.
type APIResponse<T, E> = Result<(StatusCode, T), (StatusCode, E)>;

/// Defines one API route. Implementing this trait for an empty struct will requires the `handler`
/// method that can be used as a [`Handler`] for a [`MethodRouter`].
///
/// The `State` should be set to the type expected when calling upon the server State.
///
/// The `Request` type can be any type that implements [`FromRequest`]. The simplest case is for
/// `Request` to be a tuple of the parameters that would form the parameters of the equivalent
/// handler function.
///
/// The `SuccessResponse` is the type that forms the response in the successful case (i.e. the
/// expected success structure)
///
/// Equivalently, the `ErrorResponse` is the structure of the unsuccessful case.
///
/// Note that the StatusCode should not be included in either `SuccessResponse` or `ErrorResponse`.
/// rather, the StatusCode is enforced in the `APIResult` and `APIResponse` return types. A
/// response cannot be sent without a StatusCode.
///
/// Example:
/// ```rust
///
///
/// #[derive(Deserialize)]
/// struct ExampleRequestBody {
///     name: String,
///     age: u8,
///     id: usize,
/// }
///
/// #[derive(Serialize)]
/// enum ExampleError {
///     SomethingFailed,
///     ServerSadness { tears: u8 },
///     Other,
/// }
///
/// #[derive(Serialize)]
/// struct ExampleSuccess {
///     epoch: u64,
///     reference: String,
/// }
///
/// struct ExampleHandler;
/// impl APIHandler for ExampleHandler {
///     type State = AppState;
///     // Any type that can be found in `FromRequestParts` can be included in the Request
///     type Request = (
///         CookieJar,
///         State<AppState>,
///         AuthUser,
///         Json<ExampleRequestBody>,
///     );
///
///     // Note that the `SuccessResponse` can also be unit type if there should be no response body
///     type SuccessResponse = Json<ExampleSuccess>;
///     type ErrorResponse = Json<ExampleError>;
///
///     async fn handler(
///         request: Self::Request,
///     ) -> APIResponse<Self::SuccessResponse, Self::ErrorResponse> {
///         // Destructure, just like you would in a handler function
///         let (
///             jar,
///             State(state),
///             AuthUser {
///                 uuid,
///                 muid,
///                 is_host,
///             },
///             Json(body),
///         ) = request;
///
///         // Do some stuff
///         unimplemented!()
///     }
/// }
///
/// fn main() {
///     // The `handler` function can now be used in the router
///     Router::new().route("/example", post(ExampleHandler::handler));
/// }
/// ```
trait APIHandler {
    type State: Send + Sync;
    type Request: FromRequest<Self::State>;

    type SuccessResponse: IntoResponse;
    type ErrorResponse: IntoResponse;

    async fn handler(
        request: Self::Request,
    ) -> APIResponse<Self::SuccessResponse, Self::ErrorResponse>;
}
