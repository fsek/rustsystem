#![allow(async_fn_in_trait)]

use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::FromRequest,
    http::StatusCode,
    response::IntoResponse,
    routing::{connect, delete, get, head, options, patch, post, put, trace},
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::{fmt::Display, time::SystemTime};

use std::error::Error;

pub mod logging;
pub mod mtls;
pub mod secret;
pub mod tokens;

/// Similar to APIResult, but also requires that the success type includes a [`StatusCode`].
/// If `T` and `E` are the same in `APIResult` as they are in `APIResponse`, the "?" operator can
/// still be used, just like for any other result.
pub type APIResponse<T, E> = Result<(StatusCode, T), (StatusCode, E)>;

#[derive(Serialize, Clone, Copy, Debug)]
pub enum APIErrorCode {
    InvalidUUuid,
    InvalidMUuid,

    UUuidNotFound,
    MUuidNotFound,

    VoterNameNotFound,

    UUIDAlreadyClaimed,
    NameTaken,
    AlreadyRegistered,
    NotRegistered,

    MUIDMismatch,

    InvalidMetaData,
    InvalidVoteMethod,
    InvalidVoteLength,
    InvalidCandidateId,
    VotingInactive,

    SignatureInvalid,
    SignatureExpired,
    SignatureFailure,

    InvalidState,

    // TODO: AuthError should be expanded to be more specific as to what exactly failed during
    // authentication
    AuthError,

    InvalidStatusCode,

    StateCurrupt,
    TrustAuthFetch,

    // Infrastructure / init errors
    InitError,
    CryptoError,
    QrCodeError,
    TimestampError,
    IoError,

    Other,
}
impl APIErrorCode {
    pub fn default(self) -> (&'static str, u16) {
        match self {
            Self::InvalidUUuid => ("The specified UUID could not be processed.", 422),
            Self::InvalidMUuid => ("The specified MUID could not be processed.", 422),

            Self::UUuidNotFound => (
                "The specified UUID could not be found in the meeting registry.",
                404,
            ),
            Self::MUuidNotFound => ("The specified MUID does not exist in on server.", 404),

            Self::VoterNameNotFound => (
                "The specified Voter Name could not be found in the meeting registry.",
                404,
            ),

            Self::UUIDAlreadyClaimed => (
                "The specified UUID has already been claimed. Please reattempt login.",
                409,
            ),
            Self::NameTaken => ("The name provided already exists.", 409),
            Self::AlreadyRegistered => ("User has already registered for this voting round.", 409),
            Self::NotRegistered => ("User has not registered for this voting round.", 404),
            Self::MUIDMismatch => ("The MUID doesn't match validation through JWT.", 409),

            Self::InvalidMetaData => (
                "Metadata from client doesn't match what server expected.",
                409,
            ),
            Self::InvalidVoteMethod => (
                "Vote method specified doesn't match what is set for this voting round",
                409,
            ),
            Self::InvalidVoteLength => ("Too many candidates were provided.", 409),
            Self::InvalidCandidateId => ("Vote contains an out-of-bounds candidate index.", 422),
            Self::VotingInactive => ("Vote has already been closed, or it was never opened.", 410),

            Self::SignatureInvalid => (
                "Server rejected validation signature because it doesn't match vote round keys.",
                401,
            ),
            Self::SignatureExpired => (
                "Server rejected validation signature because it has already been used.",
                409,
            ),
            Self::SignatureFailure => ("Failed to create blindsignature from token.", 500),

            Self::InvalidState => ("Action cannot be executed while in the current state.", 409),

            Self::AuthError => ("Authentication Failed", 401),

            // System faults - Immediate cause for patch.
            Self::InvalidStatusCode => ("Invalid HTTP status code.", 500),

            Self::StateCurrupt => ("AppState could not be read.", 500),
            Self::TrustAuthFetch => ("TrustAuth failed to fetch from server.", 500),

            Self::InitError => ("Server component failed to initialise.", 500),
            Self::CryptoError => ("Cryptographic operation failed.", 500),
            Self::QrCodeError => ("Failed to generate QR code.", 500),
            Self::TimestampError => ("Failed to calculate token expiry timestamp.", 500),
            Self::IoError => ("File system operation failed.", 500),

            Self::Other => ("An unexpected error occured. Please contact an admin.", 500),
        }
    }
}

#[derive(Serialize, Debug, Clone, Copy)]
pub enum Method {
    Get,
    Post,
    Put,
    Delete,
    Head,
    Options,
    Connect,
    Patch,
    Trace,
    Invalid,
}

impl From<axum::http::Method> for Method {
    fn from(value: axum::http::Method) -> Self {
        match value {
            axum::http::Method::GET => Method::Get,
            axum::http::Method::POST => Method::Post,
            axum::http::Method::PUT => Method::Put,
            axum::http::Method::DELETE => Method::Delete,
            axum::http::Method::HEAD => Method::Head,
            axum::http::Method::OPTIONS => Method::Options,
            axum::http::Method::CONNECT => Method::Connect,
            axum::http::Method::PATCH => Method::Patch,
            axum::http::Method::TRACE => Method::Trace,
            _ => Method::Invalid,
        }
    }
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct EndpointMeta {
    pub method: Method,
    pub path: String,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct APIError {
    pub code: APIErrorCode,
    // The message is not actually optional. It's only ever None to differentiate from an
    // overwrite message. When finding None, the `message` method will fill the field before
    // sending to client
    pub message: &'static str,
    pub http_status: u16,
    pub timestamp: String,
}
impl APIError {
    pub fn from_error_code(code: APIErrorCode) -> Self {
        let (message, http_status) = code.default();
        Self {
            code,
            message,
            http_status,
            timestamp: Self::timestamp(),
        }
    }

    pub fn new(code: APIErrorCode, message: &'static str, http_status: u16) -> Self {
        Self {
            code,
            message,
            http_status,
            timestamp: Self::timestamp(),
        }
    }

    pub fn timestamp() -> String {
        DateTime::<Utc>::from(SystemTime::now())
            .format("%+")
            .to_string()
    }

    pub fn finalize(self, endpoint: EndpointMeta) -> APIErrorFinal {
        APIErrorFinal {
            error: self,
            endpoint,
        }
    }
}
impl Display for APIError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl Error for APIError {}

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct APIErrorFinal {
    error: APIError,
    endpoint: EndpointMeta,
}
impl APIErrorFinal {
    fn invalid_status_code(endpoint: EndpointMeta) -> (StatusCode, Json<Self>) {
        let code = APIErrorCode::InvalidStatusCode;
        let (message, http_status) = code.default();
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(Self {
                error: APIError {
                    code,
                    message,
                    http_status,
                    timestamp: APIError::timestamp(),
                },
                endpoint,
            }),
        )
    }

    pub fn response(self) -> (StatusCode, Json<Self>) {
        let (status, res) = match StatusCode::from_u16(self.error.http_status) {
            Ok(status) => (status, Json(self)),
            Err(_err) => Self::invalid_status_code(self.endpoint),
        };

        (status, res)
    }
}

pub trait APIEndpointError: Into<APIError> {}

/// Defines one API route. Implement this trait on an empty struct, then register it with
/// [`add_handler`].
///
/// - `State` — the Axum application state type.
/// - `Request` — any type implementing [`FromRequest`]. Typically a tuple of extractors
///   (e.g. `(State<AppState>, Json<Body>)`).
/// - `SuccessResponse` — the response body type on success. Use `()` for no body.
///
/// Errors are always returned as [`APIError`]. The provided `handler` method wraps `route`,
/// converts any `APIError` into a JSON [`APIErrorFinal`] response, and attaches the correct
/// HTTP status code. Implementors only need to write `route`.
///
/// Route registration is done via [`add_handler`], which reads `METHOD` and `PATH` to call
/// the appropriate Axum routing method automatically.
///
/// Example:
/// ```rust,ignore
/// use serde::{Deserialize, Serialize};
/// use axum::{Json, extract::State, http::StatusCode};
/// use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method, add_handler};
///
/// #[derive(Deserialize)]
/// struct ExampleRequestBody {
///     name: String,
/// }
///
/// #[derive(Serialize)]
/// struct ExampleSuccess {
///     greeting: String,
/// }
///
/// struct ExampleHandler;
///
/// #[async_trait::async_trait]
/// impl APIHandler for ExampleHandler {
///     type State = AppState;
///     type Request = (State<AppState>, Json<ExampleRequestBody>);
///     // Use () if there is no response body
///     type SuccessResponse = Json<ExampleSuccess>;
///
///     const METHOD: Method = Method::Post;
///     const PATH: &'static str = "/example";
///     const SUCCESS_CODE: StatusCode = StatusCode::OK;
///
///     async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
///         let (State(state), Json(body)) = request;
///         Ok(Json(ExampleSuccess { greeting: format!("Hello, {}!", body.name) }))
///     }
/// }
///
/// // Register in a router:
/// let router = add_handler::<ExampleHandler>(Router::new());
/// ```

#[async_trait]
pub trait APIHandler: Send + Sync + 'static {
    type State: Clone + Send + Sync + 'static;
    type Request: FromRequest<Self::State> + Send + 'static;
    type SuccessResponse: IntoResponse + Send + 'static;

    const METHOD: Method;
    const PATH: &'static str;
    const SUCCESS_CODE: StatusCode;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError>;

    async fn handler(
        request: Self::Request,
    ) -> APIResponse<Self::SuccessResponse, Json<APIErrorFinal>> {
        match Self::route(request).await {
            Ok(res) => Ok((Self::SUCCESS_CODE, res)),
            Err(err) => Err(err
                .finalize(EndpointMeta {
                    method: Self::METHOD,
                    path: Self::PATH.to_string(),
                })
                .response()),
        }
    }

    fn add(router: Router<Self::State>) -> Router<Self::State> {
        match Self::METHOD {
            Method::Get => router.route(Self::PATH, get(Self::handler)),
            Method::Post => router.route(Self::PATH, post(Self::handler)),
            Method::Put => router.route(Self::PATH, put(Self::handler)),
            Method::Delete => router.route(Self::PATH, delete(Self::handler)),
            Method::Head => router.route(Self::PATH, head(Self::handler)),
            Method::Options => router.route(Self::PATH, options(Self::handler)),
            Method::Connect => router.route(Self::PATH, connect(Self::handler)),
            Method::Patch => router.route(Self::PATH, patch(Self::handler)),
            Method::Trace => router.route(Self::PATH, trace(Self::handler)),
            Method::Invalid => router, // Don't add anything
        }
    }
}

pub fn add_handler<H: APIHandler>(router: Router<H::State>) -> Router<H::State> {
    H::add(router)
}
