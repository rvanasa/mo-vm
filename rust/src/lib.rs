use std::sync::Mutex;

use lazy_static::lazy_static;
use send_wrapper::SendWrapper;
use serde::{Deserialize, Serialize};
use wasm_bindgen::prelude::*;

use motoko::check::parse;
use motoko::vm_types::{Core, Interruption};

#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "state_type", content = "value")]
pub enum HistoryState {
    Core(Core),
    Interruption(Interruption),
}

lazy_static! {
    #[wasm_bindgen]
    static ref HISTORY: Mutex<Vec<SendWrapper<HistoryState>>> = Mutex::new(vec![]);
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);
}

#[wasm_bindgen(start)]
pub fn start() {
    #[cfg(feature = "console_error_panic_hook")]
    console_error_panic_hook::set_once();
}

#[wasm_bindgen]
pub fn set_input(input: &str) -> JsValue {
    match parse(&input) {
        Ok(prog) => {
            let core = Core::new(prog);

            let history = &mut *HISTORY.lock().unwrap();
            history.clear();
            history.push(SendWrapper::new(HistoryState::Core(core)));
            JsValue::undefined()
        }
        Err(err) => JsValue::from_serde(&err).unwrap(),
    }
}

#[wasm_bindgen]
pub fn forward() -> bool {
    let history = &mut *HISTORY.lock().unwrap();

    if !history.is_empty() {
        let state = history[history.len() - 1].clone().take();
        match state {
            HistoryState::Core(mut core) => {
                let limits = motoko::vm_types::Limits::none();
                match core.step(&limits) {
                    Ok(_) => {
                        history.push(SendWrapper::new(HistoryState::Core(core)));
                        true
                    }
                    // Err(end) => JsValue::from_serde(&end).unwrap(),
                    Err(end) => {
                        history.push(SendWrapper::new(HistoryState::Interruption(end)));
                        true
                    }
                }
            }
            HistoryState::Interruption(_) => false,
        }
    } else {
        false
    }
}

#[wasm_bindgen]
pub fn backward() -> bool {
    let history = &mut *HISTORY.lock().unwrap();
    if history.len() > 1 {
        history.pop();
        true
    } else {
        false
    }
}

#[wasm_bindgen]
pub fn history() -> JsValue {
    let history = &mut *HISTORY.lock().unwrap();

    let result = &history.iter().map(|c| c.clone().take()).collect::<Vec<_>>();

    JsValue::from_serde(result).unwrap()
}
