use rustsystem_core::{APIError, APIErrorCode, APIErrorFinal, EndpointMeta, Method};
use axum::{Json, extract::FromRequestParts, http::{StatusCode, request::Parts}};
use axum_extra::extract::CookieJar;
use uuid::Uuid;

use crate::AppState;

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
        // CookieJar extraction is infallible; match the Infallible variant exhaustively.
        let jar = match CookieJar::from_request_parts(parts, state).await {
            Ok(jar) => jar,
            Err(infallible) => match infallible {},
        };

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

        let claims = rustsystem_core::tokens::decode_jwt(&access_token, state.secret(), rustsystem_core::tokens::TRUSTAUTH_ISSUER)
            .map_err(|e| e.finalize(endpoint).response())?;

        Ok(TrustAuthUser {
            uuuid: claims.uuuid,
            muuid: claims.muuid,
        })
    }
}
