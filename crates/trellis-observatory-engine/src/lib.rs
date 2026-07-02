#![forbid(unsafe_code)]

mod actions;
mod bugs;
mod compute;
mod core_projection;
mod core_runtime;
mod engine;
mod invariants;
mod language;
mod ledger;
mod replay;
mod seed;
pub mod types;

use wasm_bindgen::prelude::*;

pub use engine::{dispatch_action, initial_app_state};
pub use replay::replay_current_trace;

#[wasm_bindgen]
pub fn initial_state() -> Result<String, JsValue> {
    serde_json::to_string(&initial_app_state()).map_err(to_js_error)
}

#[wasm_bindgen]
pub fn dispatch(state_json: &str, action_json: &str) -> Result<String, JsValue> {
    let state = serde_json::from_str(state_json).map_err(to_js_error)?;
    let action = serde_json::from_str(action_json).map_err(to_js_error)?;
    serde_json::to_string(&dispatch_action(state, action)).map_err(to_js_error)
}

#[wasm_bindgen]
pub fn replay(state_json: &str) -> Result<String, JsValue> {
    let state = serde_json::from_str(state_json).map_err(to_js_error)?;
    serde_json::to_string(&replay_current_trace(&state)).map_err(to_js_error)
}

fn to_js_error(error: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&error.to_string())
}
