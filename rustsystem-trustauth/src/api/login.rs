use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use chrono::Utc;
use jsonwebtoken::{EncodingKey, Header, encode};
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;
use tracing::info;
use uuid::Uuid;

use crate::AppState;

#[derive(Serialize, Deserialize)]
struct MeetingClaims {
    uuuid: Uuid,
    muuid: Uuid,
    is_host: bool,
    exp: usize,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub uuuid: String,
    pub muuid: String,
}

pub struct Login;

#[async_trait]
impl APIHandler for Login {
    type State = AppState;
    type Request = (CookieJar, State<AppState>, Json<LoginRequest>);
    type SuccessResponse = CookieJar;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/login";
    const SUCCESS_CODE: StatusCode = StatusCode::ACCEPTED;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        info!("Logging in");

        let (jar, State(state), Json(body)) = request;

        let uuuid: Uuid = body
            .uuuid
            .parse()
            .map_err(|_| APIError::from_error_code(APIErrorCode::InvalidUUuid))?;

        let muuid: Uuid = body
            .muuid
            .parse()
            .map_err(|_| APIError::from_error_code(APIErrorCode::InvalidMUuid))?;

        if !state.is_voter(uuuid, muuid).await? {
            return Err(APIError::from_error_code(APIErrorCode::UUuidNotFound));
        }

        let expiration = Utc::now()
            .checked_add_signed(chrono::Duration::hours(12))
            .ok_or_else(|| APIError::from_error_code(APIErrorCode::TimestampError))?
            .timestamp() as usize;

        let claims = MeetingClaims {
            uuuid,
            muuid,
            is_host: false,
            exp: expiration,
        };

        let jwt = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(state.secret()),
        )
        .map_err(|_| APIError::from_error_code(APIErrorCode::Other))?;

        let is_secure = state.is_secure();
        let cookie = Cookie::build(("trustauth_token", jwt))
            .http_only(true)
            .same_site(cookie::SameSite::Strict)
            .path("/")
            .expires(OffsetDateTime::now_utc().checked_add(time::Duration::hours(12)))
            .secure(is_secure)
            .max_age(time::Duration::hours(12))
            .build();

        Ok(jar.add(cookie))
    }
}
