//! The one error type every endpoint returns.
//!
//! An [`ApiError`] is an [`ErrorCode`] plus a human-readable message. The HTTP status is a
//! property of the code, so a handler can never pair a code with the wrong status. On the wire
//! it is always:
//!
//! ```json
//! { "code": "RoundInProgress", "message": "The voter list can't change while a round is open." }
//! ```
//!
//! The frontend mirrors [`ErrorCode`] in `frontend/src/api/error.ts`; keep the two in sync.

use std::borrow::Cow;
use std::fmt;

use axum::{
    Json,
    extract::rejection::{JsonRejection, PathRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorCode {
    // ── Request shape ────────────────────────────────────────────────────────
    /// A field is missing, too long, out of range, or otherwise not acceptable.
    InvalidInput,
    BodyTooLarge,

    // ── Authentication ───────────────────────────────────────────────────────
    /// No session cookie was sent.
    NotLoggedIn,
    /// A session cookie was sent but is unknown: it expired, the voter was removed, or the
    /// meeting was closed.
    SessionExpired,
    /// The voter is logged in but is not a host.
    NotHost,
    /// The invite link is wrong, was already used, or was replaced by a reset.
    InviteInvalid,
    /// The trustauth login ticket is unknown, used, or expired.
    TicketInvalid,

    // ── Meeting and voters ───────────────────────────────────────────────────
    /// No such API endpoint.
    NotFound,
    MeetingNotFound,
    VoterNotFound,
    NameTaken,
    /// A host tried to remove themselves.
    CannotRemoveSelf,

    // ── Round state ──────────────────────────────────────────────────────────
    /// The action needs no round to be open (e.g. changing the voter list).
    RoundInProgress,
    /// The action needs an open round.
    VotingClosed,
    /// The request names a round that is not the current one.
    WrongRound,

    // ── Agenda ───────────────────────────────────────────────────────────────
    /// The action needs an agenda, and the meeting has none.
    NoAgenda,

    // ── Signing (trustauth) ──────────────────────────────────────────────────
    /// The voter is not in this round's eligible set.
    NotEligible,
    /// The voter already got their one signature for this round.
    AlreadySigned,

    // ── Ballots (server) ─────────────────────────────────────────────────────
    MalformedBallot,
    InvalidSignature,
    InvalidBallot,
    /// This exact ballot was already counted. Clients treat this as success.
    AlreadyReceived,
    /// As many ballots as eligible voters have been counted.
    BallotLimitReached,

    // ── Capacity ─────────────────────────────────────────────────────────────
    RateLimited,
    TooManyConnections,

    // ── Faults (never the client's fault) ────────────────────────────────────
    TrustauthUnavailable,
    Internal,
}

impl ErrorCode {
    pub fn status(self) -> StatusCode {
        use ErrorCode::*;
        match self {
            InvalidInput | MalformedBallot | InvalidSignature | InvalidBallot => {
                StatusCode::BAD_REQUEST
            }
            NotLoggedIn | SessionExpired | InviteInvalid | TicketInvalid => {
                StatusCode::UNAUTHORIZED
            }
            NotHost | NotEligible => StatusCode::FORBIDDEN,
            NotFound | MeetingNotFound | VoterNotFound => StatusCode::NOT_FOUND,
            NameTaken | CannotRemoveSelf | RoundInProgress | VotingClosed | WrongRound | NoAgenda
            | AlreadySigned | AlreadyReceived | BallotLimitReached => StatusCode::CONFLICT,
            RateLimited | TooManyConnections => StatusCode::TOO_MANY_REQUESTS,
            BodyTooLarge => StatusCode::PAYLOAD_TOO_LARGE,
            TrustauthUnavailable => StatusCode::BAD_GATEWAY,
            Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn default_message(self) -> &'static str {
        use ErrorCode::*;
        match self {
            InvalidInput => "The request contained an invalid value.",
            NotLoggedIn => "You are not logged in.",
            SessionExpired => {
                "Your session has ended. The meeting may have closed, or you were removed."
            }
            NotHost => "Only hosts can do this.",
            InviteInvalid => {
                "This invite link is not valid. It may already have been used or replaced."
            }
            TicketInvalid => "The login ticket is not valid or has expired.",
            NotFound => "No such API endpoint.",
            MeetingNotFound => "The meeting does not exist.",
            VoterNotFound => "The voter does not exist.",
            NameTaken => "Someone in the meeting already has that name.",
            CannotRemoveSelf => "You can't remove yourself.",
            RoundInProgress => "This can't be done while a vote round is open.",
            VotingClosed => "No vote round is open.",
            WrongRound => "That vote round is no longer the current one.",
            NoAgenda => "This meeting has no agenda yet.",
            NotEligible => "You are not eligible to vote in this round.",
            AlreadySigned => "You have already voted in this round.",
            MalformedBallot => "The ballot could not be read.",
            InvalidSignature => "The ballot's signature is not valid for this round.",
            InvalidBallot => "The ballot does not follow the rules of this round.",
            AlreadyReceived => "This ballot has already been counted.",
            BallotLimitReached => "Every eligible voter's ballot has already been counted.",
            RateLimited => "Too many requests. Slow down and try again shortly.",
            TooManyConnections => "Too many open connections. Try again shortly.",
            BodyTooLarge => "The request is too large.",
            TrustauthUnavailable => "The signing service could not be reached. Try again.",
            Internal => "Something went wrong on the server. Please tell a host.",
        }
    }
}

#[derive(Serialize, Debug, Clone)]
pub struct ApiError {
    pub code: ErrorCode,
    pub message: Cow<'static, str>,
}

impl ApiError {
    pub fn new(code: ErrorCode) -> Self {
        Self {
            code,
            message: Cow::Borrowed(code.default_message()),
        }
    }

    /// An error with a message specific to this situation, e.g. which field was invalid.
    pub fn with_message(code: ErrorCode, message: impl Into<Cow<'static, str>>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }

    pub fn invalid_input(message: impl Into<Cow<'static, str>>) -> Self {
        Self::with_message(ErrorCode::InvalidInput, message)
    }

    /// A server fault. The details are logged; the client only sees a generic message.
    pub fn internal(context: impl fmt::Display) -> Self {
        tracing::error!("internal error: {context}");
        Self::new(ErrorCode::Internal)
    }
}

impl From<ErrorCode> for ApiError {
    fn from(code: ErrorCode) -> Self {
        Self::new(code)
    }
}

impl From<JsonRejection> for ApiError {
    fn from(rejection: JsonRejection) -> Self {
        if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
            return Self::new(ErrorCode::BodyTooLarge);
        }
        Self::invalid_input(rejection.body_text())
    }
}

impl From<PathRejection> for ApiError {
    fn from(rejection: PathRejection) -> Self {
        Self::invalid_input(rejection.body_text())
    }
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.code, self.message)
    }
}

impl std::error::Error for ApiError {}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.code.status(), Json(self)).into_response()
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serializes_code_and_message() {
        let json = serde_json::to_value(ApiError::new(ErrorCode::NotHost)).unwrap();
        assert_eq!(json["code"], "NotHost");
        assert_eq!(json["message"], "Only hosts can do this.");
    }

    #[test]
    fn status_comes_from_code() {
        let res = ApiError::new(ErrorCode::AlreadyReceived).into_response();
        assert_eq!(res.status(), StatusCode::CONFLICT);
        let res = ApiError::invalid_input("Title is too long").into_response();
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}
