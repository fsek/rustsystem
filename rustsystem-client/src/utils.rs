use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{console::info_1, Request, RequestCredentials, RequestInit, RequestMode, Response};

use crate::API_ENDPOINT;

pub fn set_panic_hook() {
    // When the `console_error_panic_hook` feature is enabled, we can call the
    // `set_panic_hook` function at least once during initialization, and then
    // we will get better error messages if our code ever panics.
    //
    // For more details see
    // https://github.com/rustwasm/console_error_panic_hook#readme
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
extern "C" {
    fn alert(s: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    set_panic_hook();
}

pub fn log(value: &str) {
    info_1(&JsValue::from_str(value));
}

pub async fn send_post(body: &str, endpoint: &str) -> Result<Option<JsValue>, JsValue> {
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
    if let Ok(json_val) = JsFuture::from(json_promise).await {
        Ok(Some(json_val))
    } else {
        // For responses wihout json
        Ok(None)
    }
}
