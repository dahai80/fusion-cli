use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

const DEFAULT_SOCK: &str = "/tmp/fusion-guard.sock";
const ENV_SOCK: &str = "FUSION_GUARD_SOCK";
const TIMEOUT_SECS: u64 = 2;

#[derive(Debug, Serialize)]
struct Request {
    jsonrpc: &'static str,
    id: u64,
    method: String,
    params: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Response {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    #[allow(dead_code)]
    id: Option<u64>,
    result: Option<serde_json::Value>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PingResult {
    pub pong: bool,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub rules_epoch: u64,
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

fn call(method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
    let path = sock_path();
    info!(sock = %path.display(), method = method, "guard UDS call");
    let mut stream = UnixStream::connect(&path)
        .with_context(|| format!("cannot connect to guard socket {}", path.display()))?;
    stream.set_read_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))?;
    stream.set_write_timeout(Some(Duration::from_secs(TIMEOUT_SECS)))?;

    let req = Request {
        jsonrpc: "2.0",
        id: 1,
        method: method.to_string(),
        params,
    };
    let mut payload = serde_json::to_vec(&req)?;
    payload.push(b'\n');
    stream.write_all(&payload)?;
    stream.flush()?;

    let mut reader = BufReader::new(stream);
    let mut buf: Vec<u8> = Vec::new();
    reader.read_until(b'\n', &mut buf)?;
    let line = String::from_utf8_lossy(&buf);
    let resp: Response = serde_json::from_str(line.trim())
        .with_context(|| format!("invalid guard response: {}", line.trim()))?;
    if let Some(err) = resp.error {
        anyhow::bail!("guard rpc error ({}): {}", err.code, err.message);
    }
    resp.result.context("guard rpc returned no result")
}

pub fn ping() -> Result<PingResult> {
    let raw = call("guard.ping", serde_json::json!({}))?;
    let result: PingResult = serde_json::from_value(raw)?;
    Ok(result)
}

pub fn list_rules() -> Result<serde_json::Value> {
    call("guard.rule.list", serde_json::json!({}))
}

pub fn list_audit(limit: u32) -> Result<serde_json::Value> {
    call("guard.audit.list", serde_json::json!({ "limit": limit }))
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
        let p = resolve_sock_path(Some("/tmp/custom-guard-test.sock".to_string()));
        assert_eq!(p, PathBuf::from("/tmp/custom-guard-test.sock"));
    }

    #[test]
    fn test_response_parses_result_field() {
        let raw =
            r#"{"jsonrpc":"2.0","id":1,"result":{"pong":true,"version":"0.2.0","rules_epoch":3}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        assert!(resp.error.is_none());
        let ping: PingResult = serde_json::from_value(resp.result.unwrap()).unwrap();
        assert!(ping.pong);
        assert_eq!(ping.version, "0.2.0");
        assert_eq!(ping.rules_epoch, 3);
    }

    #[test]
    fn test_response_surfaces_rpc_error() {
        let raw =
            r#"{"jsonrpc":"2.0","id":1,"error":{"code":-32601,"message":"method not found"}}"#;
        let resp: Response = serde_json::from_str(raw).unwrap();
        let err = resp.error.unwrap();
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "method not found");
    }

    #[test]
    fn test_request_envelope_is_valid_jsonrpc() {
        let req = Request {
            jsonrpc: "2.0",
            id: 7,
            method: "guard.ping".to_string(),
            params: serde_json::json!({}),
        };
        let s = serde_json::to_string(&req).unwrap();
        let v: serde_json::Value = serde_json::from_str(&s).unwrap();
        assert_eq!(v["jsonrpc"], "2.0");
        assert_eq!(v["id"], 7);
        assert_eq!(v["method"], "guard.ping");
    }
}
