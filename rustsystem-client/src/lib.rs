use rustsystem_proof::{
    Ballot, Provider, RegistrationResponse, Sha256Provider, WASMRegistrationResponse,
};
use wasm_bindgen::prelude::*;
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

// Required on the frontend (already wasm_bindgen)
pub use rustsystem_proof::{BallotMetaData, BallotValidation, WASMChoice};

mod utils;

mod registration;

mod validation;

const API_ENDPOINT: &str = env!("API_ENDPOINT");

#[wasm_bindgen]
pub fn new_ballot_validation(
    proof: Vec<u8>,
    token: Vec<u8>,
    signature: JsValue,
) -> Result<BallotValidation, JsError> {
    let blind_sign: BlindSignature<BbsBls12381Sha256> =
        serde_wasm_bindgen::from_value(signature).map_err(JsError::from)?;
    Ok(BallotValidation::new(proof, token, blind_sign))
}

#[wasm_bindgen]
pub fn start_vote_json_req(name: String, metadata: BallotMetaData) -> Result<JsValue, JsError> {
    serde_wasm_bindgen::to_value(&(name, metadata)).map_err(JsError::from)
}
