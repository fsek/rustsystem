mod utils;

use rustsystem_proof::{
    generate_token_sha, ProofContext, RegistrationInfo, RegistrationResponse, ValidationInfo,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console::info_1, Request, RequestInit, RequestMode, Response};
use zkryptium::{
    bbsplus::commitment::BlindFactor,
    schemes::{algorithms::BbsBls12381Sha256, generics::BlindSignature},
};

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
    match serde_json::from_str::<RegistrationResponse>(&response.as_string()?).ok()? {
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
pub async fn test_register() -> RegistrationResult {
    register(vec![0, 1], vec![1, 0]).await
}

#[wasm_bindgen]
pub async fn register(voter_id: Vec<u8>, round_hash: Vec<u8>) -> RegistrationResult {
    try_register(voter_id, round_hash).await.unwrap()
}

#[wasm_bindgen]
pub async fn send_vote(reg_res: RegistrationResult) -> Result<JsValue, JsValue> {
    let info = ValidationInfo::new(
        reg_res.proof(),
        reg_res.token(),
        serde_wasm_bindgen::from_value::<BlindSignature<BbsBls12381Sha256>>(reg_res.signature())
            .unwrap(),
    );
    let body = serde_json::to_string(&info).unwrap();

    let res = send_post(&body, "vote").await?;

    Ok(res)
}

async fn send_post(body: &str, endpoint: &str) -> Result<JsValue, JsValue> {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body));
    opts.set_mode(RequestMode::Cors);

    let url = format!("https://127.0.0.1:8443/{endpoint}");
    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Content-Type", "application/json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;
    let json_promise = resp_value.clone().dyn_into::<Response>()?.json()?;
    let json_val = JsFuture::from(json_promise).await?;

    Ok(json_val)
}

async fn try_register(
    voter_id: Vec<u8>,
    round_hash: Vec<u8>,
) -> Result<RegistrationResult, JsValue> {
    let (context, token, commitment, proof) = generate_token_sha(voter_id, round_hash).unwrap();
    let info = RegistrationInfo::new(context, commitment);
    let body = serde_json::to_string(&info).unwrap();

    match get_signature(send_post(&body, "register").await?) {
        Some(sign) => Ok(RegistrationResult::with_signature(
            proof.to_bytes().to_vec(),
            token,
            serde_wasm_bindgen::to_value(&sign)?,
        )),
        None => Err(JsValue::from_str("Failed to retrieve signature")),
    }
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, rustsystem-client!");
}
