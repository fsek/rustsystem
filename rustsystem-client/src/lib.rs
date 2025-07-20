mod utils;

use rustsystem_proof::{
    Provider, RegistrationInfo, RegistrationResponse, Sha256Provider, ValidationInfo,
};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console::info_1, Request, RequestCredentials, RequestInit, RequestMode, Response};
use zkryptium::schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature};

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

fn get_signature(response: JsValue) -> Option<BlindSignature<BbsBls12381Sha256>> {
    match serde_wasm_bindgen::from_value(response).ok()? {
        RegistrationResponse::Rejected(reason) => {
            log(&format!("{reason:?}"));
            None
        }
        RegistrationResponse::Accepted(sign) => Some(sign),
    }
}

#[wasm_bindgen]
pub fn verify_registration_success(response: JsValue) -> JsValue {
    if let Some(_sign) = get_signature(response) {
        JsValue::TRUE
    } else {
        JsValue::FALSE
    }
}

#[wasm_bindgen]
pub struct RegistrationResult {
    proof: Vec<u8>,
    token: Vec<u8>,
    signature: JsValue,
}
#[wasm_bindgen]
impl RegistrationResult {
    pub fn with_signature(proof: Vec<u8>, token: Vec<u8>, signature: JsValue) -> Self {
        Self {
            proof,
            token,
            signature,
        }
    }

    #[wasm_bindgen]
    pub fn signature(&self) -> JsValue {
        self.signature.clone()
    }

    #[wasm_bindgen]
    pub fn proof(&self) -> Vec<u8> {
        self.proof.clone()
    }

    #[wasm_bindgen]
    pub fn token(&self) -> Vec<u8> {
        self.token.clone()
    }
}

#[wasm_bindgen]
pub async fn register(voter_id: String, meeting_id: String) -> RegistrationResult {
    try_register(
        voter_id.parse::<u128>().unwrap().to_be_bytes().to_vec(),
        meeting_id.parse::<u128>().unwrap().to_be_bytes().to_vec(),
    )
    .await
    .unwrap()
}

async fn try_register(
    voter_id: Vec<u8>,
    round_hash: Vec<u8>,
) -> Result<RegistrationResult, JsValue> {
    log("Trying to register");
    let (context, token, commitment, proof) =
        Sha256Provider::generate_token(voter_id, round_hash).unwrap();
    let info = Sha256Provider::new_reg_info(context, commitment);
    let body = serde_json::to_string(&info).unwrap();

    match get_signature(send_post(&body, "api/vote/register").await?) {
        Some(sign) => Ok(RegistrationResult::with_signature(
            proof.to_bytes().to_vec(),
            token,
            serde_wasm_bindgen::to_value(&sign)?,
        )),
        None => Err(JsValue::from_str("Failed to retrieve signature")),
    }
}

#[wasm_bindgen]
pub async fn send_vote(reg_res: RegistrationResult) -> Result<JsValue, JsValue> {
    let info = Sha256Provider::new_val_info(
        reg_res.proof(),
        reg_res.token(),
        serde_wasm_bindgen::from_value::<BlindSignature<BbsBls12381Sha256>>(reg_res.signature())
            .unwrap(),
    );
    let body = serde_json::to_string(&info).unwrap();

    let res = send_post(&body, "api/vote/submit").await?;

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
pub fn greet() {
    alert("Hello, rustsystem-client!");
}
