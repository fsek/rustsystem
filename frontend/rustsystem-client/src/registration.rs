use rustsystem_proof::{
    BallotMetaData, Provider, RegistrationReject, RegistrationSuccessResponse, Sha256Provider,
    WASMRegistrationResponse,
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

    let uuid = voter_id.into_bytes();
    let muid = meeting_id.into_bytes();

    let (context, token, commitment, proof) = Sha256Provider::generate_token(uuid, muid).unwrap();
    let info = Sha256Provider::new_reg_info(context, commitment);
    let body = serde_json::to_string(&info).unwrap();

    if let Ok(Some(res)) = send_post(&body, "api/voter/register").await {
        // Requires clone because res isn't `Copy`
        if let Ok(success_res) =
            serde_wasm_bindgen::from_value::<RegistrationSuccessResponse>(res.clone())
        {
            Ok(RegistrationResult::new(
                WASMRegistrationResponse::from(success_res),
                token,
                proof,
            ))
        } else if let Ok(err_res) = serde_wasm_bindgen::from_value::<RegistrationReject>(res) {
            Err(JsError::from(err_res))
        } else {
            Err(JsError::from(RegistrationReject::Empty))
        }
    } else {
        todo!("Fix error handling here");
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
