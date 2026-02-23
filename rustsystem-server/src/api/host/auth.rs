use api_core::{APIError, APIErrorCode, APIErrorFinal, EndpointMeta};
use axum::{Json, extract::FromRequestParts, http::StatusCode};

use crate::{AppState, MUuid, UUuid, tokens::AuthUser};

pub struct AuthHost {
    pub uuuid: UUuid,
    pub muuid: MUuid,
}
impl From<AuthUser> for AuthHost {
    fn from(value: AuthUser) -> Self {
        Self {
            uuuid: value.uuuid,
            muuid: value.muuid,
        }
    }
}

impl FromRequestParts<AppState> for AuthHost {
    type Rejection = (StatusCode, Json<APIErrorFinal>);

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.is_host {
            Ok(user.into())
        } else {
            let endpoint = EndpointMeta {
                method: api_core::Method::from(parts.method.clone()),
                path: parts.uri.path().to_string(),
            };
            Err(APIError::from_error_code(APIErrorCode::AuthError)
                .finalize(endpoint)
                .response())
        }
    }
}
