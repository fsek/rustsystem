use async_trait::async_trait;
use axum::{
    Json,
    extract::{FromRequest, State},
    http::StatusCode,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use rustsystem_core::{APIError, APIErrorCode, APIHandler, Method};
use serde::Serialize;

use crate::{AppState, api::host::auth::AuthHost};

#[derive(Serialize)]
pub struct TallyFileEntry {
    pub filename: String,
    pub data: String, // base64-encoded encrypted bytes
}

#[derive(FromRequest)]
pub struct GetAllTallyRequest {
    auth: AuthHost,
    state: State<AppState>,
}

pub struct GetAllTally;

#[async_trait]
impl APIHandler for GetAllTally {
    type State = AppState;
    type Request = GetAllTallyRequest;
    type SuccessResponse = Json<Vec<TallyFileEntry>>;

    const METHOD: Method = Method::Get;
    const PATH: &'static str = "/get-all-tally";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let GetAllTallyRequest {
            auth,
            state: State(state),
        } = request;

        // Ensure the meeting exists
        state.get_meeting(auth.muuid).await?;

        let dir = format!("meetings/{}", auth.muuid);
        let entries = std::fs::read_dir(&dir)
            .map_err(|_| APIError::from_error_code(APIErrorCode::Other))?;

        let mut files: Vec<TallyFileEntry> = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|_| APIError::from_error_code(APIErrorCode::Other))?;
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if name.starts_with("tally-") && name.ends_with(".enc") {
                let data = std::fs::read(entry.path())
                    .map_err(|_| APIError::from_error_code(APIErrorCode::Other))?;
                files.push(TallyFileEntry {
                    filename: name.to_string(),
                    data: STANDARD.encode(&data),
                });
            }
        }

        // Sort by filename (timestamp-based) for a deterministic order
        files.sort_by(|a, b| a.filename.cmp(&b.filename));

        Ok(Json(files))
    }
}
