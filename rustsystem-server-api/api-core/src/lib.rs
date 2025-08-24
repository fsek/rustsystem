use std::{fmt::Display, time::SystemTime};

use axum::{Json, extract::FromRequest, http::StatusCode, response::IntoResponse};
use chrono::{DateTime, Utc};
use serde::Serialize;

/// The response may be a failure type which is equally valid as it pertains to the API structure.
/// Helper functions may use the APIResult type as a return type such that the "?" operator can be
/// used to send the error all the way to the response
pub type APIResult<T, E: APIEndpointError> = Result<T, E>;

/// Similar to APIResult, but also requires that the success type includes a [`StatusCode`].
/// If `T` and `E` are the same in `APIResult` as they are in `APIResponse`, the "?" operator can
/// still be used, just like for any other result.
pub type APIResponse<T, E> = Result<(StatusCode, T), (StatusCode, E)>;

#[derive(Serialize, Clone, Copy, Debug)]
pub enum APIErrorCode {
    InvalidUUID,
    InvalidMUID,

    UUIDNotFound,
    MUIDNotFound,

    UUIDAlreadyClaimed,
    AlreadyRegistered,

    MUIDMismatch,

    InvalidMetaData,
    InvalidVoteMethod,
    VotingInactive,

    SignatureInvalid,
    SignatureExpired,
    SignatureFailure,

    InvalidStatusCode,
}
impl APIErrorCode {
    pub fn message(self) -> &'static str {
        match self {
            Self::InvalidUUID => "The specified UUID could not be processed.",
            Self::InvalidMUID => "The specified MUID could not be processed.",

            Self::UUIDNotFound => "The specified UUID could not be found in the meeting registry.",
            Self::MUIDNotFound => "The specified MUID does not exist in on server.",

            Self::UUIDAlreadyClaimed => {
                "The specified UUID has already been claimed. Please reattempt login."
            }

            Self::AlreadyRegistered => "User has already registered for this voting round.",

            Self::MUIDMismatch => "The MUID doesn't match validation through JWT.",

            Self::InvalidMetaData => "Metadata from client doesn't match what server expected.",
            Self::InvalidVoteMethod => {
                "Vote method specified doesn't match what is set for this voting round"
            }
            Self::VotingInactive => "Vote has already been closed, or it was never opened.",

            Self::SignatureInvalid => {
                "Server rejected validation signature because it doesn't match vote round keys."
            }
            Self::SignatureExpired => {
                "Server rejected validation signature because it has already been used."
            }
            Self::SignatureFailure => "Failed to create blindsignature from token.",

            // System faults - Immediate cause for patch.
            Self::InvalidStatusCode => "Invalid HTTP status code.",
        }
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
pub struct EndpointMeta {
    pub method: &'static str,
    pub path: &'static str,
}

#[derive(Serialize, Debug)]
pub struct APIError {
    pub code: APIErrorCode,
    // The message is not actually optional. It's only ever None to differentiate from an
    // overwrite message. When finding None, the `message` method will fill the field before
    // sending to client
    pub message: Option<&'static str>,
    pub http_status: u16,
    pub timestamp: String,
    pub endpoint: EndpointMeta,
}
impl APIError {
    pub fn timestamp() -> String {
        DateTime::<Utc>::from(SystemTime::now())
            .format("%+")
            .to_string()
    }

    fn invalid_status_code(endpoint: EndpointMeta) -> (StatusCode, Json<Self>) {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Self {
                code: APIErrorCode::InvalidStatusCode,
                message: None,
                http_status: StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                timestamp: Self::timestamp(),
                endpoint,
            }),
        )
    }

    fn finalize(self, endpoint: EndpointMeta) -> (StatusCode, Json<Self>) {
        let (status, mut res) = match StatusCode::from_u16(self.http_status) {
            Ok(status) => (status, Json(self)),
            Err(_err) => APIError::invalid_status_code(endpoint),
        };

        if res.message == None {
            res.message = Some(res.code.message());
        }

        (status, res)
    }
}
impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

pub trait APIEndpointError: Into<APIError> {}

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
pub trait APIHandler {
    type State: Send + Sync;
    type Request: FromRequest<Self::State>;

    const SUCCESS_CODE: StatusCode;
    type SuccessResponse: IntoResponse;
    type ErrorResponse: APIEndpointError;

    async fn route(request: Self::Request)
    -> APIResult<Self::SuccessResponse, Self::ErrorResponse>;

    async fn handler(request: Self::Request) -> APIResponse<Self::SuccessResponse, Json<APIError>> {
        match Self::route(request).await {
            Ok(res) => Ok((Self::SUCCESS_CODE, res)),
            Err(endpoint_err) => {
                let err = endpoint_err.into();
                let endpoint = err.endpoint;
                Err(err.finalize(endpoint))
            }
        }
    }
}
