use rustsystem_proof::{Ballot, WASMChoice};
use wasm_bindgen::prelude::*;

use crate::utils::send_post;

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
