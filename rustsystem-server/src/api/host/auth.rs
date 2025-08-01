use axum::{extract::FromRequestParts, http::StatusCode};

use crate::{AppState, MUID, UUID, tokens::AuthUser};

pub struct AuthHost {
    pub uuid: UUID,
    pub muid: MUID,
}
impl From<AuthUser> for AuthHost {
    fn from(value: AuthUser) -> Self {
        Self {
            uuid: value.uuid,
            muid: value.muid,
        }
    }
}

impl FromRequestParts<AppState> for AuthHost {
    type Rejection = StatusCode;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let user = AuthUser::from_request_parts(parts, state).await?;
        if user.is_host {
            Ok(user.into())
        } else {
            Err(StatusCode::FORBIDDEN)
        }
    }
}
