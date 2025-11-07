use api_core::APIError;
use axum::{Json, extract::FromRequestParts, http::StatusCode};

use crate::{
    AppState, MUuid, UUuid,
    tokens::{AuthError, AuthUser},
};

pub struct AuthVoter {
    pub uuuid: UUuid,
    pub muuid: MUuid,
}
impl From<AuthUser> for AuthVoter {
    fn from(value: AuthUser) -> Self {
        Self {
            uuuid: value.uuuid,
            muuid: value.muuid,
        }
    }
}

impl FromRequestParts<AppState> for AuthVoter {
    type Rejection = (StatusCode, Json<APIError>);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.is_host {
            Err(<AuthError as Into<APIError>>::into(AuthError::AuthError).finalize())
        } else {
            Ok(user.into())
        }
    }
}
