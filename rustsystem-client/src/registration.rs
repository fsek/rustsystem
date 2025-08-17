use rustsystem_proof::{
    BallotMetaData, Provider, RegistrationResponse, Sha256Provider, WASMRegistrationResponse,
};
use wasm_bindgen::prelude::*;
use zkryptium::bbsplus::commitment::BlindFactor;

use crate::utils::{log, send_post};

#[wasm_bindgen]
pub async fn try_register(
    voter_id: String,
    meeting_id: String,
) -> Result<RegistrationResult, JsError> {
    log("Trying to register");

    let uuid = voter_id
        .parse::<u128>()
        .map_err(JsError::from)?
        .to_be_bytes()
        .to_vec();
    let muid = meeting_id
        .parse::<u128>()
        .map_err(JsError::from)?
        .to_be_bytes()
        .to_vec();

    let (context, token, commitment, proof) = Sha256Provider::generate_token(uuid, muid).unwrap();
    let info = Sha256Provider::new_reg_info(context, commitment);
    let body = serde_json::to_string(&info).unwrap();

    match serde_wasm_bindgen::from_value::<RegistrationResponse>(
        send_post(&body, "api/voter/register").await.unwrap(),
    ) {
        Ok(res) => Ok(RegistrationResult::new(
            WASMRegistrationResponse::from(res),
            token,
            proof,
        )),
        Err(e) => Err(JsError::from(e)),
    }
}

#[wasm_bindgen]
pub struct RegistrationResult {
    response: WASMRegistrationResponse,
    token: Vec<u8>,
    proof: Vec<u8>,
}
impl RegistrationResult {
    pub fn new(response: WASMRegistrationResponse, token: Vec<u8>, proof: BlindFactor) -> Self {
        Self {
            response,
            token,
            proof: proof.to_bytes().to_vec(),
        }
    }
}
#[wasm_bindgen]
impl RegistrationResult {
    #[wasm_bindgen]
    pub fn proof(&self) -> Vec<u8> {
        self.proof.clone()
    }
    #[wasm_bindgen]
    pub fn token(&self) -> Vec<u8> {
        self.token.clone()
    }
    #[wasm_bindgen]
    pub fn signature(&self) -> Result<JsValue, JsError> {
        serde_wasm_bindgen::to_value(
            &self
                .response
                .signature()
                .ok_or(JsError::new("Signature is empty"))?,
        )
        .map_err(JsError::from)
    }

    #[wasm_bindgen]
    pub fn metadata(&self) -> Option<BallotMetaData> {
        self.response.metadata()
    }

    #[wasm_bindgen]
    pub fn is_valid(&self) -> bool {
        self.response.is_valid()
    }

    #[wasm_bindgen]
    pub fn is_successful(&self) -> bool {
        self.response.is_successful()
    }
}
