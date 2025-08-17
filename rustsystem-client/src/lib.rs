mod utils;

use rustsystem_proof::{
    Ballot, Choice, Provider, RegistrationInfo, RegistrationResponse, Sha256Provider,
    ValidationInfo, VoteMethod, WASMRegistrationResponse,
};
use wasm_bindgen::{convert::IntoWasmAbi, prelude::*};
use wasm_bindgen_futures::JsFuture;
use web_sys::{
    console::info_1, js_sys, Request, RequestCredentials, RequestInit, RequestMode, Response,
};
use zkryptium::{
    bbsplus::commitment::BlindFactor,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

// Required on the frontend (already wasm_bindgen)
pub use rustsystem_proof::{BallotMetaData, BallotValidation, WASMChoice};

const API_ENDPOINT: &str = env!("API_ENDPOINT");

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    utils::set_panic_hook();
}

fn log(value: &str) {
    info_1(&JsValue::from_str(value));
}

fn get_registration_response(response: JsValue) -> Option<RegistrationResponse> {
    serde_wasm_bindgen::from_value(response).ok()
}

#[wasm_bindgen]
pub struct RegistrationResult {
    response: WASMRegistrationResponse,
    token: Vec<u8>,
    proof: Vec<u8>,
}
impl RegistrationResult {
    fn new(response: WASMRegistrationResponse, token: Vec<u8>, proof: BlindFactor) -> Self {
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
    pub fn signature(&self) -> Result<JsValue, js_sys::Error> {
        serde_wasm_bindgen::to_value(
            &self
                .response
                .signature()
                .ok_or(js_sys::Error::new("Signature is empty"))?,
        )
        .map_err(|e| js_sys::Error::new(&format!("Could not convert signature to JsValue: {e}")))
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

#[wasm_bindgen]
pub fn new_ballot_validation(
    proof: Vec<u8>,
    token: Vec<u8>,
    signature: JsValue,
) -> Result<BallotValidation, js_sys::Error> {
    let blind_sign: BlindSignature<BbsBls12381Sha256> =
        serde_wasm_bindgen::from_value(signature)
            .map_err(|e| js_sys::Error::new(&format!("Could not parse signature: {e}")))?;
    Ok(BallotValidation::new(proof, token, blind_sign))
}

#[wasm_bindgen]
pub async fn try_register(
    voter_id: String,
    meeting_id: String,
) -> Result<RegistrationResult, js_sys::Error> {
    log("Trying to register");

    let uuid = voter_id
        .parse::<u128>()
        .map_err(|e| js_sys::Error::new(&e.to_string()))?
        .to_be_bytes()
        .to_vec();
    let muid = meeting_id
        .parse::<u128>()
        .map_err(|e| js_sys::Error::new(&e.to_string()))?
        .to_be_bytes()
        .to_vec();

    let (context, token, commitment, proof) = Sha256Provider::generate_token(uuid, muid).unwrap();
    let info = Sha256Provider::new_reg_info(context, commitment);
    let body = serde_json::to_string(&info).unwrap();

    match get_registration_response(
        send_post(&body, "api/voter/register")
            .await
            .map_err(|e| e.unchecked_into::<js_sys::Error>())?,
    ) {
        Some(res) => Ok(RegistrationResult::new(
            WASMRegistrationResponse::from(res),
            token,
            proof,
        )),
        None => Err(js_sys::Error::new("Failed to retrieve signature")),
    }
}

#[wasm_bindgen]
pub async fn send_vote(
    metadata_parsed: JsValue,
    choice_parsed: JsValue,
    validation_parsed: JsValue,
) -> Result<JsValue, JsError> {
    let metadata = serde_wasm_bindgen::from_value(metadata_parsed).map_err(JsError::from)?;
    let choice =
        serde_wasm_bindgen::from_value::<WASMChoice>(choice_parsed).map_err(JsError::from)?;
    let validation = serde_wasm_bindgen::from_value(validation_parsed).map_err(JsError::from)?;

    let ballot = Ballot::new(metadata, choice.into_choice(), validation);
    let body = serde_json::to_string(&ballot).map_err(JsError::from)?;

    let res = send_post(&body, "api/voter/submit").await.unwrap();

    Ok(res)
}

async fn send_post(body: &str, endpoint: &str) -> Result<JsValue, JsValue> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body));
    opts.set_mode(RequestMode::Cors);
    opts.set_credentials(RequestCredentials::Include);

    let url = format!("{API_ENDPOINT}/{endpoint}");
    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Content-Type", "application/json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let json_promise = resp_value.clone().dyn_into::<Response>()?.json()?;
    let json_val = JsFuture::from(json_promise).await?;

    Ok(json_val)
}

#[wasm_bindgen]
pub fn start_vote_json_req(
    name: String,
    metadata: BallotMetaData,
) -> Result<JsValue, js_sys::Error> {
    serde_wasm_bindgen::to_value(&(name, metadata))
        .map_err(|e| js_sys::Error::new(&format!("Failed to serialize StartVoteRequest: {e}")))
}
