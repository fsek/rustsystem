use std::{error::Error, fmt::Display};

use api_derive::APIEndpointError;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use serde::{Deserialize, Serialize};

use api_core::{APIErrorCode, APIHandler, APIResult};
use tokio_stream::{StreamExt, adapters::FilterMap, wrappers::WatchStream};

use crate::{AppState, tokens::AuthUser};

#[derive(FromRequest)]
pub struct MeetingSpecsRequest {
    auth: AuthUser,
    state: State<AppState>,
}

#[derive(Serialize)]
pub struct MeetingSpecsResponse {
    title: String,
    participants: usize,
    agenda: String,
}

#[derive(APIEndpointError, Debug)]
#[api(endpoint(method = "GET", path = "api/common/meeting-specs"))]
pub enum MeetingSpecsError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}
impl Display for MeetingSpecsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for MeetingSpecsError {}

pub struct MeetingSpecs;
impl APIHandler for MeetingSpecs {
    type State = AppState;
    type Request = MeetingSpecsRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = Json<MeetingSpecsResponse>;
    type ErrorResponse = MeetingSpecsError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let MeetingSpecsRequest {
            auth,
            state: State(state),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get(&auth.muuid) {
            Ok(Json(MeetingSpecsResponse {
                title: meeting.title.clone(),
                participants: meeting.voters.values().filter(|v| v.logged_in).count(),
                agenda: meeting.agenda.clone(),
            }))
        } else {
            Err(MeetingSpecsError::MUIDNotFound)
        }
    }
}

pub struct MeetingSpecsWatch;
impl APIHandler for MeetingSpecsWatch {
    type State = AppState;
    type Request = MeetingSpecsRequest;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, MeetingSpecsError>>>>;
    type ErrorResponse = MeetingSpecsError;
    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let MeetingSpecsRequest {
            auth,
            state: State(state),
        } = request;

        let upon_event = |new_state: bool| {
            if new_state {
                Some(Ok::<Event, MeetingSpecsError>(
                    Event::default().data("NewData"),
                ))
            } else {
                // This should never be called. The frontend will not recognize it!
                Some(Ok::<Event, MeetingSpecsError>(
                    Event::default().data("DataFailure"),
                ))
            }
        };

        if let Some(meeting) = state.meetings.lock().await.get(&auth.muuid) {
            let update_rx = meeting.vote_auth.new_update_watcher();
            let stream = WatchStream::new(update_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(MeetingSpecsError::MUIDNotFound)
        }
    }
}

#[derive(Deserialize)]
pub struct UpdateAgendaRequest {
    agenda: String,
}

#[derive(FromRequest)]
pub struct UpdateAgendaRequestWrapper {
    auth: AuthUser,
    state: State<AppState>,
    body: Json<UpdateAgendaRequest>,
}

#[derive(APIEndpointError, Debug)]
#[api(endpoint(method = "POST", path = "api/common/update-agenda"))]
pub enum UpdateAgendaError {
    #[api(code = APIErrorCode::MUuidNotFound, status = 404)]
    MUIDNotFound,
}
impl Display for UpdateAgendaError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl Error for UpdateAgendaError {}

pub struct UpdateAgenda;
impl APIHandler for UpdateAgenda {
    type State = AppState;
    type Request = UpdateAgendaRequestWrapper;

    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();
    type ErrorResponse = UpdateAgendaError;

    async fn route(
        request: Self::Request,
    ) -> APIResult<Self::SuccessResponse, Self::ErrorResponse> {
        let UpdateAgendaRequestWrapper {
            auth,
            state: State(state),
            body: Json(req),
        } = request;

        if let Some(meeting) = state.meetings.lock().await.get_mut(&auth.muuid) {
            meeting.agenda = req.agenda;
            // Trigger meeting specs update
            let _ = meeting.vote_auth.send_update();
            Ok(())
        } else {
            Err(UpdateAgendaError::MUIDNotFound)
        }
    }
}
