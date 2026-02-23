use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
    response::{Sse, sse::Event},
};
use serde::{Deserialize, Serialize};

use api_core::{APIError, APIErrorCode, APIHandler, Method};
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

pub struct MeetingSpecs;
#[async_trait]
impl APIHandler for MeetingSpecs {
    type State = AppState;
    type Request = MeetingSpecsRequest;
    type SuccessResponse = Json<MeetingSpecsResponse>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/meeting-specs";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let MeetingSpecsRequest {
            auth,
            state: State(state),
        } = request;

        let meetings_guard = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            Ok(Json(MeetingSpecsResponse {
                title: meeting.title.clone(),
                participants: meeting.voters.values().filter(|v| v.logged_in).count(),
                agenda: meeting.agenda.clone(),
            }))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}

pub struct MeetingSpecsWatch;
#[async_trait]
impl APIHandler for MeetingSpecsWatch {
    type State = AppState;
    type Request = MeetingSpecsRequest;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/meeting-specs-watch";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse =
        Sse<FilterMap<WatchStream<bool>, fn(bool) -> Option<Result<Event, APIError>>>>;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let MeetingSpecsRequest {
            auth,
            state: State(state),
        } = request;

        let upon_event = |new_state: bool| {
            if new_state {
                Some(Ok::<Event, APIError>(Event::default().data("NewData")))
            } else {
                // This should never be called. The frontend will not recognize it!
                Some(Ok::<Event, APIError>(Event::default().data("DataFailure")))
            }
        };

        let meetings_guard = {
            let guard = state.read()?;
            guard.meetings.clone()
        };

        if let Some(meeting) = meetings_guard.lock().await.get(&auth.muuid) {
            let update_rx = meeting.vote_auth.new_update_watcher();
            let stream = WatchStream::new(update_rx).filter_map(upon_event as _);
            Ok(Sse::new(stream))
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
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

pub struct UpdateAgenda;
#[async_trait]
impl APIHandler for UpdateAgenda {
    type State = AppState;
    type Request = UpdateAgendaRequestWrapper;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/update-agenda";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;
    type SuccessResponse = ();

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let UpdateAgendaRequestWrapper {
            auth,
            state: State(state),
            body: Json(req),
        } = request;

        let meetings_guard = {
            let guard = state.write()?;
            guard.meetings.clone()
        };

        if let Some(meeting) = meetings_guard.lock().await.get_mut(&auth.muuid) {
            meeting.agenda = req.agenda;
            // Trigger meeting specs update
            let _ = meeting.vote_auth.send_update();
            Ok(())
        } else {
            Err(APIError::from_error_code(APIErrorCode::MUuidNotFound))
        }
    }
}
