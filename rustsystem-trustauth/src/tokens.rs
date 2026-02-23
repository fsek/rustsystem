use api_core::{APIError, APIErrorCode, APIErrorFinal, EndpointMeta, Method};
use axum::{Json, extract::FromRequestParts, http::{StatusCode, request::Parts}};
use axum_extra::extract::CookieJar;
use jsonwebtoken::{DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::AppState;

#[derive(Serialize, Deserialize)]
struct MeetingClaims {
    uuuid: Uuid,
    muuid: Uuid,
    is_host: bool,
    exp: usize,
}

pub struct TrustAuthUser {
    pub uuuid: Uuid,
    pub muuid: Uuid,
}

impl FromRequestParts<AppState> for TrustAuthUser {
    type Rejection = (StatusCode, Json<APIErrorFinal>);

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .expect("infallible");

        let endpoint = EndpointMeta {
            method: Method::from(parts.method.clone()),
            path: parts.uri.path().to_string(),
        };

        let access_token = jar
            .get("trustauth_token")
            .ok_or_else(|| {
                APIError::from_error_code(APIErrorCode::AuthError)
                    .finalize(endpoint.clone())
                    .response()
            })?
            .value()
            .to_owned();

        let token_data = decode::<MeetingClaims>(
            &access_token,
            &DecodingKey::from_secret(state.secret()),
            &Validation::default(),
        )
        .map_err(|_| {
            APIError::from_error_code(APIErrorCode::AuthError)
                .finalize(endpoint)
                .response()
        })?;

        Ok(TrustAuthUser {
            uuuid: token_data.claims.uuuid,
            muuid: token_data.claims.muuid,
        })
    }
}
