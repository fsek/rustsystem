mod utils;

use rustsystem_proof::{generate_token_sha, ProofContext, RegistrationInfo};
use serde::Serialize;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    utils::set_panic_hook();
}

#[wasm_bindgen]
pub async fn test_register() -> JsValue {
    register(vec![0, 1], vec![1, 0]).await
}

#[wasm_bindgen]
pub async fn register(voter_id: Vec<u8>, round_hash: Vec<u8>) -> JsValue {
    try_register(voter_id, round_hash).await.unwrap()
}

#[wasm_bindgen]
pub async fn test_post() -> JsValue {
    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(
        &serde_json::to_string("Testing").unwrap(),
    ));
    opts.set_mode(RequestMode::Cors);

    let url = "https://127.0.0.1:8443/try-post";
    let request = Request::new_with_str_and_init(&url, &opts).unwrap();
    request
        .headers()
        .set("Content-Type", "application/json")
        .unwrap();

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request))
        .await
        .unwrap();

    resp_value
}

async fn try_register(voter_id: Vec<u8>, round_hash: Vec<u8>) -> Result<JsValue, JsValue> {
    let (context, commitment, proof) = generate_token_sha(voter_id, round_hash).unwrap();
    let info = RegistrationInfo::new(context, commitment);
    let body = serde_json::to_string(&info).unwrap();

    let opts = RequestInit::new();
    opts.set_method("POST");
    opts.set_body(&JsValue::from_str(&body));
    opts.set_mode(RequestMode::Cors);

    let url = "https://127.0.0.1:8443/register";
    let request = Request::new_with_str_and_init(&url, &opts)?;
    request.headers().set("Content-Type", "application/json")?;

    let window = web_sys::window().unwrap();
    let resp_value = JsFuture::from(window.fetch_with_request(&request)).await?;

    Ok(resp_value)
}

#[wasm_bindgen]
pub fn greet() {
    alert("Hello, rustsystem-client!");
}
