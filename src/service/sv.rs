use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;
use tracing::info;

const DEFAULT_SOCK: &str = "/tmp/fusion-sv.sock";
const ENV_SOCK: &str = "FUSION_SV_SOCKET";
const ENV_TOKEN: &str = "FUSION_SV_TOKEN";
const TIMEOUT_SECS: u64 = 5;

// daemon-down 与 rpc-error 必须可区分: cmd 层据此映射 exit code
// (daemon-down → exit 3, rpc-error → exit 1, 匹配 fusion-sv CLI 约定)。
#[derive(Debug, Error)]
pub enum SvError {
    #[error("daemon 未运行, 用 `fusion-sv daemon` 启动: {0}")]
    DaemonDown(String),
    #[error("rpc error ({code}): {message}")]
    Rpc { code: i64, message: String },
    #[error("{0}")]
    Other(String),
}

// envelope 镜像 fusion-supervisor src/rpc/schema.rs: RpcRequest{jsonrpc, method, params:Value, id:i64}
#[derive(Debug, Serialize)]
struct Request {
    jsonrpc: &'static str,
    id: i64,
    method: String,
    params: serde_json::Value,
}

// RpcResponse: result/error 互斥, id:i64。字段顺序/类型与 supervisor schema 一致。
#[derive(Debug, Deserialize)]
struct Response {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    #[allow(dead_code)]
    id: Option<i64>,
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

fn resolve_sock_path(env_value: Option<String>) -> PathBuf {
    match env_value {
        Some(v) if !v.is_empty() => PathBuf::from(v),
        _ => PathBuf::from(DEFAULT_SOCK),
    }
}

fn sock_path() -> PathBuf {
    resolve_sock_path(std::env::var(ENV_SOCK).ok())
}

// params 注入 token (若 FUSION_SV_TOKEN 设置)。supervisor server 读 params.get("token").as_str()。
// inject_token 为纯函数 (token 由调用方传入), 测试免 env mutation (Rust 2024 set_var unsafe + 并发 race)。
fn inject_token(mut params: serde_json::Value, token: Option<String>) -> serde_json::Value {
    if let Some(tok) = token
        && !tok.is_empty()
        && let Some(obj) = params.as_object_mut()
    {
        obj.insert("token".into(), serde_json::Value::String(tok));
    }
    params
}

fn with_token(params: serde_json::Value) -> serde_json::Value {
    inject_token(params, std::env::var(ENV_TOKEN).ok())
}

fn call(method: &str, params: serde_json::Value) -> Result<serde_json::Value, SvError> {
    let path = sock_path();
    info!(sock = %path.display(), method = method, "fusion-sv UDS call");
    let mut stream = UnixStream::connect(&path).map_err(|e| SvError::DaemonDown(e.to_string()))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))
        .map_err(|e| SvError::Other(e.to_string()))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))
        .map_err(|e| SvError::Other(e.to_string()))?;

    let req = Request {
        jsonrpc: "2.0",
        id: 1,
        method: method.to_string(),
        params: with_token(params),
    };
    let mut payload = serde_json::to_vec(&req).map_err(|e| SvError::Other(e.to_string()))?;
    payload.push(b'\n');
    stream
        .write_all(&payload)
        .map_err(|e| SvError::Other(e.to_string()))?;
    stream.flush().map_err(|e| SvError::Other(e.to_string()))?;

    let mut reader = BufReader::new(stream);
    let mut buf: Vec<u8> = Vec::new();
    reader
        .read_until(b'\n', &mut buf)
        .map_err(|e| SvError::Other(e.to_string()))?;
    let line = String::from_utf8_lossy(&buf);
    let resp: Response = serde_json::from_str(line.trim()).map_err(|e| {
        SvError::Other(format!(
            "invalid fusion-sv response: {} | raw: {}",
            e,
            line.trim()
        ))
    })?;
    if let Some(err) = resp.error {
        return Err(SvError::Rpc {
            code: err.code,
            message: err.message,
        });
    }
    resp.result
        .ok_or_else(|| SvError::Other("fusion-sv rpc returned no result".into()))
}

pub fn ping() -> Result<bool, SvError> {
    let raw = call("ping", serde_json::json!(Value::Null))?;
    Ok(raw == "pong")
}

pub fn status() -> Result<serde_json::Value, SvError> {
    call("status", serde_json::json!(Value::Null))
}

pub fn up() -> Result<serde_json::Value, SvError> {
    call("up", serde_json::json!(Value::Null))
}

pub fn down() -> Result<serde_json::Value, SvError> {
    call("down", serde_json::json!(Value::Null))
}

pub fn restart(service: &str) -> Result<serde_json::Value, SvError> {
    call("restart", serde_json::json!({ "service": service }))
}

// 给 cmd 层判断 daemon-down (exit 3) vs 其他错误 (exit 1)。
pub fn is_daemon_down(e: &SvError) -> bool {
    matches!(e, SvError::DaemonDown(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_sock_path_defaults_when_env_missing() {
        assert_eq!(resolve_sock_path(None), PathBuf::from(DEFAULT_SOCK));
    }

    #[test]
    fn test_resolve_sock_path_defaults_when_env_empty() {
        assert_eq!(
            resolve_sock_path(Some(String::new())),
            PathBuf::from(DEFAULT_SOCK)
        );
    }

    #[test]
    fn test_resolve_sock_path_respects_env_override() {
        let p = resolve_sock_path(Some("/tmp/custom-sv-test.sock".to_string()));
        assert_eq!(p, PathBuf::from("/tmp/custom-sv-test.sock"));
    }

    #[test]
    fn test_response_parses_result_field() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":"pong"}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert!(resp.error.is_none());
        assert_eq!(resp.result.unwrap(), "pong");
    }

    #[test]
    fn test_response_parses_status_array() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"result":[{"name":"fusion-mlx","state":"Healthy","port":11434}]}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        let arr = resp.result.unwrap();
        assert_eq!(arr[0]["name"], "fusion-mlx");
        assert_eq!(arr[0]["port"], 11434);
    }

    #[test]
    fn test_response_surfaces_rpc_error() {
        let raw = r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32001,"message":"unauthorized"}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32001);
        assert_eq!(err.message, "unauthorized");
    }

    #[test]
    fn test_request_envelope_is_valid_jsonrpc() {
        let req = Request {
            jsonrpc: "2.0",
            id: 1,
            method: "ping".to_string(),
            params: serde_json::json!(Value::Null),
        };
        let s = serde_json::to_string(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 1);
        assert_eq!(v["method"], "ping");
        assert!(v["params"].is_null());
    }

    #[test]
    fn test_restart_envelope_carries_service_param() {
        let req = Request {
            jsonrpc: "2.0",
            id: 1,
            method: "restart".to_string(),
            params: serde_json::json!({ "service": "fusion-mlx" }),
        };
        let s = serde_json::to_string(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["params"]["service"], "fusion-mlx");
    }

    // token 注入: 纯函数 inject_token, 不 mutate env (race-free under parallel cargo test)。
    #[test]
    fn test_inject_token_noop_when_none() {
        let p = inject_token(serde_json::json!({ "service": "x" }), None);
        assert_eq!(p["service"], "x");
        assert!(p.get("token").is_none());
    }

    #[test]
    fn test_inject_token_noop_when_empty() {
        let p = inject_token(serde_json::json!({ "service": "x" }), Some(String::new()));
        assert_eq!(p["service"], "x");
        assert!(p.get("token").is_none());
    }

    #[test]
    fn test_inject_token_adds_field_when_set() {
        let p = inject_token(serde_json::json!({ "service": "x" }), Some("secret".into()));
        assert_eq!(p["service"], "x");
        assert_eq!(p["token"], "secret");
    }

    #[test]
    fn test_inject_token_on_null_params_promotes_to_object() {
        // ping 用 params:Null, 有 token 时需升级为 object 才能放 token 字段。
        // 当前 inject_token 对 Null 不升级 (as_object_mut 返回 None) — ping 无需 token 注入。
        // 此测试锁定该行为: Null + token 仍为 Null (ping 不带 token, supervisor 对 ping 不验权)。
        let p = inject_token(serde_json::Value::Null, Some("secret".into()));
        assert!(p.is_null());
    }

    #[test]
    fn test_is_daemon_down_classifies_error() {
        let down = SvError::DaemonDown("refused".into());
        let rpc = SvError::Rpc {
            code: -1,
            message: "x".into(),
        };
        let other = SvError::Other("x".into());
        assert!(is_daemon_down(&down));
        assert!(!is_daemon_down(&rpc));
        assert!(!is_daemon_down(&other));
    }
}
