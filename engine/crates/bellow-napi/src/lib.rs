use bellow_core::{Engine, EngineConfig, EngineInfo};
use napi::bindgen_prelude::*;
use napi_derive::napi;
use std::sync::{Arc, Mutex};

// Global engine instance, shared across all NAPI calls.
// napi-rs requires Send + Sync for values passed to JS.
lazy_static::lazy_static! {
    static ref ENGINE: Mutex<Arc<Engine>> = Mutex::new(Arc::new(Engine::new()));
}

fn get_engine() -> Arc<Engine> {
    Arc::clone(&*ENGINE.lock().unwrap())
}

#[napi]
pub fn engine_init(config: String) -> String {
    let config: EngineConfig = match serde_json::from_str(&config) {
        Ok(c) => c,
        Err(e) => {
            return serde_json::to_string(&serde_json::json!({
                "ok": false,
                "error": format!("Invalid config: {}", e)
            }))
            .unwrap();
        }
    };

    let engine = get_engine();
    let result = engine.init(config);
    serde_json::to_string(&match result {
        Ok(()) => serde_json::json!({"ok": true}),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    })
    .unwrap()
}

#[napi]
pub fn engine_shutdown() -> String {
    let engine = get_engine();
    let result = engine.shutdown();
    serde_json::to_string(&match result {
        Ok(()) => serde_json::json!({"ok": true}),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    })
    .unwrap()
}

#[napi]
pub fn engine_suspend() -> String {
    let engine = get_engine();
    let result = engine.suspend();
    serde_json::to_string(&match result {
        Ok(()) => serde_json::json!({"ok": true}),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    })
    .unwrap()
}

#[napi]
pub fn engine_resume() -> String {
    let engine = get_engine();
    let result = engine.resume();
    serde_json::to_string(&match result {
        Ok(()) => serde_json::json!({"ok": true}),
        Err(e) => serde_json::json!({"ok": false, "error": e.to_string()}),
    })
    .unwrap()
}

#[napi]
pub fn engine_version() -> String {
    let engine = get_engine();
    let info = engine.version();
    serde_json::to_string(&info).unwrap()
}
