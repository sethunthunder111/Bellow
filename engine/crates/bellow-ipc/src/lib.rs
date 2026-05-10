//! bellow-ipc — framed JSON-RPC transport for sidecar mode
//!
//! Length-prefixed JSON messages over TCP (localhost). The server reads a
//! 4-byte BE length header, then the JSON payload, dispatches to the engine,
//! and writes back a framed response.

use bellow_core::{Engine, EngineConfig};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};

#[derive(Serialize)]
struct RpcResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl RpcResult {
    fn ok() -> Self {
        Self {
            ok: true,
            error: None,
        }
    }
    fn err(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
        }
    }
}

pub mod error;
pub use error::IpcError;

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcRequest {
    pub id: u64,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RpcResponse {
    pub id: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// A framed RPC server that dispatches to an Engine instance.
pub struct RpcServer {
    engine: Arc<Engine>,
}

impl RpcServer {
    pub fn new(engine: Arc<Engine>) -> Self {
        Self { engine }
    }

    pub async fn run(self, port: u16) -> Result<(), IpcError> {
        let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).await?;
        println!("[bellow-daemon] Listening on 127.0.0.1:{}", port);

        loop {
            let (socket, addr) = listener.accept().await?;
            println!("[bellow-daemon] Client connected: {}", addr);
            let engine = Arc::clone(&self.engine);
            if let Err(e) = handle_client(socket, engine).await {
                eprintln!("[bellow-daemon] Client error: {}", e);
            }
        }
    }
}

async fn handle_client(mut socket: TcpStream, engine: Arc<Engine>) -> Result<(), IpcError> {
    let mut buf = vec![0u8; 4096];
    let mut accumulated = Vec::new();

    loop {
        // Read length-prefixed messages
        let n = socket.read(&mut buf).await?;
        if n == 0 {
            break; // client disconnected
        }
        accumulated.extend_from_slice(&buf[..n]);

        // Try to parse complete messages
        while accumulated.len() >= 4 {
            let len = u32::from_be_bytes([
                accumulated[0],
                accumulated[1],
                accumulated[2],
                accumulated[3],
            ]) as usize;
            if accumulated.len() < 4 + len {
                break; // need more data
            }

            let payload = &accumulated[4..4 + len];
            let response = match serde_json::from_slice::<RpcRequest>(payload) {
                Ok(req) => dispatch(&engine, req),
                Err(e) => RpcResponse {
                    id: 0,
                    result: None,
                    error: Some(format!("Parse error: {}", e)),
                },
            };

            let resp_bytes = serde_json::to_vec(&response)?;
            let resp_len = resp_bytes.len() as u32;
            socket.write_all(&resp_len.to_be_bytes()).await?;
            socket.write_all(&resp_bytes).await?;

            accumulated.drain(..4 + len);
        }
    }

    Ok(())
}

fn dispatch(engine: &Engine, req: RpcRequest) -> RpcResponse {
    fn extract_str<'a>(params: &'a serde_json::Value, key: &str) -> Option<&'a str> {
        params.get(key).and_then(|v| v.as_str())
    }
    fn extract_f32(params: &serde_json::Value, key: &str, default: f32) -> f32 {
        params
            .get(key)
            .and_then(|v| v.as_f64())
            .map(|f| f as f32)
            .unwrap_or(default)
    }
    fn extract_u64(params: &serde_json::Value, key: &str, default: u64) -> u64 {
        params.get(key).and_then(|v| v.as_u64()).unwrap_or(default)
    }
    fn extract_bool(params: &serde_json::Value, key: &str, default: bool) -> bool {
        params.get(key).and_then(|v| v.as_bool()).unwrap_or(default)
    }

    let result = match req.method.as_str() {
        "engine.init" => match serde_json::from_value::<EngineConfig>(req.params) {
            Ok(config) => match engine.init(config) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            },
            Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
        },
        "engine.shutdown" => match engine.shutdown() {
            Ok(()) => serde_json::to_value(RpcResult::ok()),
            Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
        },
        "engine.suspend" => match engine.suspend() {
            Ok(()) => serde_json::to_value(RpcResult::ok()),
            Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
        },
        "engine.resume" => match engine.resume() {
            Ok(()) => serde_json::to_value(RpcResult::ok()),
            Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
        },
        "engine.version" => serde_json::to_value(engine.version()),
        // ---- Sound playback (M1) ----
        "sound.load" => {
            let src = extract_str(&req.params, "src").unwrap_or("");
            match engine.sound_load(src) {
                Ok(handle) => serde_json::to_value(handle),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.play" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            match engine.sound_play(id) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.pause" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            match engine.sound_pause(id) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.stop" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            match engine.sound_stop(id) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.seek" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            let pos = extract_u64(&req.params, "positionMs", 0);
            match engine.sound_seek(id, pos) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.setVolume" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            let vol = extract_f32(&req.params, "volume", 1.0);
            match engine.sound_set_volume(id, vol) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.setRate" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            let rate = extract_f32(&req.params, "rate", 1.0);
            match engine.sound_set_rate(id, rate) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.setLoop" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            let lp = extract_bool(&req.params, "loop", false);
            match engine.sound_set_loop(id, lp) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.dispose" => {
            let id = extract_str(&req.params, "id").unwrap_or("");
            match engine.sound_dispose(id) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "sound.list" => serde_json::to_value(engine.sound_list()),
        // ---- Master ----
        "master.setVolume" => {
            let vol = extract_f32(&req.params, "volumeDb", 0.0);
            match engine.master_set_volume(vol) {
                Ok(()) => serde_json::to_value(RpcResult::ok()),
                Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
            }
        }
        "master.get" => serde_json::to_value(engine.master_get()),
        // ---- Devices ----
        "devices.list" => match bellow_io::devices::list_devices() {
            Ok(list) => serde_json::to_value(list),
            Err(e) => serde_json::to_value(RpcResult::err(e.to_string())),
        },
        _ => serde_json::to_value(RpcResult::err(format!("Unknown method: {}", req.method))),
    };

    match result {
        Ok(v) => RpcResponse {
            id: req.id,
            result: Some(v),
            error: None,
        },
        Err(e) => RpcResponse {
            id: req.id,
            result: None,
            error: Some(e.to_string()),
        },
    }
}
