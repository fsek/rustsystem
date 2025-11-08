use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use rustsystem_proof::BallotMetaData;
use serde::Deserialize;

use api_core::{APIErrorCode, APIHandler, APIResult};

use crate::{
    AppState,
    vote_auth::{self, TallyError},
};

use super::auth::AuthHost;

#[derive(Deserialize)]
pub struct StartVoteRequest {
    name: String,
    shuffle: bool,
    metadata: BallotMetaData,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/start-vote"))]
pub enum StartVoteError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
    #[api(code = APIErrorCode::MeetingUnlocked, status = 409)]
    MeetingUnlocked,
}

pub struct StartVote;
impl APIHandler for StartVote {
    type State = AppState;
    type Request = (AuthHost, State<AppState>, Json<StartVoteRequest>);

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = StartVoteError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let (AuthHost { uuuid, muuid }, State(state), Json(body)) = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            if !meeting.locked {
                return Err(StartVoteError::MeetingUnlocked);
            }
            meeting
                .get_auth()
                .start_round(body.metadata, body.shuffle, body.name);

            Ok(())
        } else {
            Err(StartVoteError::MUIDNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct TallyRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct Tally;
impl APIHandler for Tally {
    type State = AppState;
    type Request = TallyRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<vote_auth::Tally>;
    type ErrorResponse = TallyError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let TallyRequest {
            auth: AuthHost { uuuid, muuid },
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            let vote_auth = meeting.get_auth();

            Ok(Json(vote_auth.finalize_round()?))
        } else {
            Err(TallyError::MUIDNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct EndVoteRoundRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "DELETE", path = "/api/host/end-vote-round"))]
pub enum EndVoteRoundError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}

pub struct EndVoteRound;
impl APIHandler for EndVoteRound {
    type State = AppState;
    type Request = EndVoteRoundRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = EndVoteRoundError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let EndVoteRoundRequest {
            auth: AuthHost { uuuid, muuid },
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            meeting.get_auth().reset();

            Ok(())
        } else {
            Err(EndVoteRoundError::MUIDNotFound)
        }
    }
}

#[derive(FromRequest)]
pub struct LockRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/lock"))]
pub enum LockError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUuidNotFound,
    #[api(code = APIErrorCode::MeetingLocked, status = 409)]
    MeetingLocked,
}
pub struct Lock;
impl APIHandler for Lock {
    type State = AppState;
    type Request = LockRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = LockError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let LockRequest {
            auth: AuthHost { uuuid: _, muuid },
            state: State(state),
        } = request;
        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            if !meeting.locked {
                meeting.lock();
            } else {
                // this is what we want, meeting is already locked
                // just return ok
            }
        } else {
            return Err(LockError::MUuidNotFound);
        }
        Ok(())
    }
}

#[derive(FromRequest)]
pub struct UnlockRequest {
    auth: AuthHost,
    state: State<AppState>,
}

#[derive(APIEndpointError)]
#[api(endpoint(method = "POST", path = "/api/host/unlock"))]
pub enum UnlockError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUuidNotFound,
    #[api(code = APIErrorCode::MeetingUnlocked, status = 409)]
    MeetingUnlocked,
}
pub struct Unlock;
impl APIHandler for Unlock {
    type State = AppState;
    type Request = LockRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = UnlockError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let LockRequest {
            auth: AuthHost { uuuid, muuid },
            state: State(state),
        } = request;
        if let Some(meeting) = state.meetings.lock().await.get_mut(&muuid) {
            if !meeting.locked {
                return Err(UnlockError::MeetingUnlocked);
            } else {
                meeting.unlock();
            }
        } else {
            return Err(UnlockError::MUuidNotFound);
        }
        Ok(())
    }
}
