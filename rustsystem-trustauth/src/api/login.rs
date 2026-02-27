use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use axum_extra::extract::{
    CookieJar,
    cookie::{self, Cookie},
};
use serde::Deserialize;
use tracing::info;
use uuid::Uuid;

use crate::AppState;

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

        let jwt = rustsystem_core::tokens::encode_jwt(uuuid, muuid, false, state.secret(), rustsystem_core::tokens::TRUSTAUTH_ISSUER)?;

        let is_secure = state.is_secure();
        let cookie = Cookie::build(("trustauth_token", jwt))
            .http_only(true)
            .same_site(cookie::SameSite::Strict)
            .path("/")
            .max_age(time::Duration::hours(12))
            .secure(is_secure)
            .build();

        info!(
            muuid = %muuid,
            uuuid = %uuuid,
            "Voter logged in to trustauth"
        );

        Ok(jar.add(cookie))
    }
}
