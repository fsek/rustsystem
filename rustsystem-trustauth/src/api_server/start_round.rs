use rustsystem_core::{APIError, APIHandler, Method};
use async_trait::async_trait;
use axum::{Json, extract::State, http::StatusCode};
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc};
use uuid::Uuid;
use zkryptium::{
    bbsplus::ciphersuites::BbsCiphersuite,
    keys::pair::KeyPair,
    schemes::algorithms::{BbsBls12381Sha256, Scheme},
};

use tokio::sync::RwLock as AsyncRwLock;

use crate::{AppState, AuthenticationKeys, RoundState};

#[derive(Deserialize)]
pub struct StartRoundRequest {
    pub muuid: Uuid,
    pub name: String,
}

#[derive(Serialize)]
pub struct StartRoundResponse {
    pub pub_key_bytes: Vec<u8>,
}

fn generate_keys() -> AuthenticationKeys {
    let material: Vec<u8> = (0..<BbsBls12381Sha256 as Scheme>::Ciphersuite::IKM_LEN)
        .map(|_| {
            let mut buf = [0u8];
            rand::rng().fill(&mut buf);
            buf[0]
        })
        .collect();
    KeyPair::<BbsBls12381Sha256>::generate(&material, None, None).unwrap()
}

pub struct StartRound;

#[async_trait]
impl APIHandler for StartRound {
    type State = AppState;
    type Request = (State<AppState>, Json<StartRoundRequest>);
    type SuccessResponse = Json<StartRoundResponse>;

    const METHOD: Method = Method::Post;
    const PATH: &'static str = "/start-round";
    const SUCCESS_CODE: StatusCode = StatusCode::OK;

    async fn route(request: Self::Request) -> Result<Self::SuccessResponse, APIError> {
        let (State(state), Json(body)) = request;

        let keys = generate_keys();
        let pub_key_bytes = keys.public_key().to_bytes().to_vec();
        let header = body.name.as_bytes().to_vec();

        let round = Arc::new(RoundState {
            keys,
            header,
            registered_voters: AsyncRwLock::new(HashMap::new()),
        });

        let rounds_arc = state.rounds_write();
        rounds_arc.write().await.insert(body.muuid, round);

        Ok(Json(StartRoundResponse { pub_key_bytes }))
    }
}
