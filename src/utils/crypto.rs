// utils/crypto.rs — 配置静态加密 (#50)
//
// AES-256-GCM 封套: master.key (32B 随机, 0600, ~/.fusion/master.key) 本地保管,
// 落盘密文格式 enc:<base64(nonce||ciphertext||tag)>, 内存明文。
// 与 age / credential-helper 同模型: 不引入 KMS, 单机封套, master.key 0600 防同机他用户读。
// master.key 丢失则密文不可解 → load_config 走明文回退路径, doctor 告警。

use anyhow::{Context, Result};
use base64::{Engine, engine::general_purpose::STANDARD as B64};
use rand::RngCore;
use std::path::PathBuf;

use aes_gcm::{
    Aes256Gcm, KeyInit, Nonce,
    aead::{Aead, OsRng},
};

const PREFIX: &str = "enc:";
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

fn master_key_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion")
        .join("master.key")
}

// 生成 32B 随机 master key 并落盘 0600。已存在则不覆盖 (防轮换丢密)。
pub fn ensure_master_key() -> Result<Vec<u8>> {
    let path = master_key_path();
    if path.exists() {
        let key = std::fs::read(&path).context("read master.key")?;
        if key.len() != KEY_LEN {
            anyhow::bail!(
                "master.key corrupt: expected {} bytes, got {} — remove {} to regenerate",
                KEY_LEN,
                key.len(),
                path.display()
            );
        }
        return Ok(key);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut key = vec![0u8; KEY_LEN];
    // OsRng: 密码学级熵源 (aes-gcm aead::OsRng 包装 getrandom)。
    OsRng.fill_bytes(&mut key);
    std::fs::write(&path, &key).context("write master.key")?;
    restrict_perms(&path);
    tracing::info!(path = %path.display(), "generated new master.key");
    Ok(key)
}

#[cfg(unix)]
fn restrict_perms(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn restrict_perms(_path: &std::path::Path) {}

// 加密明文 → enc:<b64(nonce||ct||tag)>。master key 缺失自动生成。
pub fn encrypt(plaintext: &str) -> Result<String> {
    if plaintext.is_empty() {
        return Ok(String::new());
    }
    // 已加密 (带前缀) 不重复加密 — 防 save_config 二次包装。
    if plaintext.starts_with(PREFIX) {
        return Ok(plaintext.to_string());
    }
    let key = ensure_master_key()?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow::anyhow!("invalid master key length"))?;
    let mut nonce_bytes = [0u8; NONCE_LEN];
    rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ct = cipher
        .encrypt(nonce, plaintext.as_bytes())
        .map_err(|_| anyhow::anyhow!("AES-GCM encrypt failed"))?;
    // nonce(12) || ct||tag 拼接后整体 base64。
    let mut bundle = Vec::with_capacity(NONCE_LEN + ct.len());
    bundle.extend_from_slice(&nonce_bytes);
    bundle.extend_from_slice(&ct);
    Ok(format!("{}{}", PREFIX, B64.encode(&bundle)))
}

// 解密 enc:<b64> → 明文。非 enc: 前缀 (明文) 原样返回 (向后兼容)。
// master key 缺失 → 返回原文 (告警由 doctor 负责), 不阻断 CLI 可用。
pub fn decrypt(value: &str) -> Result<String> {
    if !value.starts_with(PREFIX) {
        return Ok(value.to_string());
    }
    let b64 = &value[PREFIX.len()..];
    let bundle = B64.decode(b64).context("decrypt: base64 decode failed")?;
    if bundle.len() < NONCE_LEN {
        anyhow::bail!("decrypt: ciphertext too short");
    }
    let (nonce_bytes, ct) = bundle.split_at(NONCE_LEN);
    let path = master_key_path();
    if !path.exists() {
        tracing::warn!(
            "encrypted value present but master.key missing — returning ciphertext, secrets unavailable"
        );
        return Ok(value.to_string());
    }
    let key = std::fs::read(&path).context("read master.key")?;
    if key.len() != KEY_LEN {
        anyhow::bail!("master.key wrong length: {}", key.len());
    }
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| anyhow::anyhow!("invalid master key length"))?;
    let nonce = Nonce::from_slice(nonce_bytes);
    let pt = cipher.decrypt(nonce, ct).map_err(|_| {
        anyhow::anyhow!("AES-GCM decrypt failed — wrong master.key or tampered ciphertext")
    })?;
    String::from_utf8(pt).context("decrypt: plaintext not UTF-8")
}

// doctor: master.key 是否存在 (供告警)。
pub fn master_key_exists() -> bool {
    master_key_path().exists()
}

#[cfg(test)]
mod tests {
    use super::*;

    // 跨模块共享 HOME 锁 (audit + crypto 串行改 HOME)。
    use crate::utils::HOME_LOCK;

    // 生成唯一临时 HOME (per-call 纳秒后缀, 防并发测试目录撞名)。
    fn tmp_home(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!("fusion-crypto-{tag}-{nanos}"));
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    fn with_home<F: FnOnce() -> Result<()>>(tmp: &std::path::Path, f: F) {
        let _guard = HOME_LOCK.lock().unwrap();
        let prev = std::env::var_os("HOME");
        // edition 2024: set_var unsafe
        unsafe {
            std::env::set_var("HOME", tmp);
        }
        let res = f();
        if let Some(h) = prev {
            unsafe {
                std::env::set_var("HOME", h);
            }
        }
        res.unwrap();
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let tmp = tmp_home("rt");
        with_home(&tmp, || {
            let ct = encrypt("super-secret-key")?;
            assert!(ct.starts_with("enc:"));
            assert_ne!(ct, "enc:");
            let pt = decrypt(&ct)?;
            assert_eq!(pt, "super-secret-key");
            Ok(())
        });
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_encrypt_empty_passthrough() {
        assert_eq!(encrypt("").unwrap(), "");
        assert_eq!(decrypt("").unwrap(), "");
    }

    #[test]
    fn test_decrypt_plaintext_passthrough() {
        assert_eq!(decrypt("fg-admin-key").unwrap(), "fg-admin-key");
    }

    #[test]
    fn test_encrypt_idempotent_on_ciphertext() {
        let tmp = tmp_home("idem");
        with_home(&tmp, || {
            let ct = encrypt("once")?;
            let ct2 = encrypt(&ct)?;
            assert_eq!(ct, ct2, "re-encrypting ciphertext must not re-wrap");
            Ok(())
        });
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_decrypt_wrong_key_fails() {
        let tmp = tmp_home("wrongkey");
        with_home(&tmp, || {
            let ct = encrypt("secret")?;
            let key_path = tmp.join(".fusion").join("master.key");
            std::fs::remove_file(&key_path).unwrap();
            let _ = ensure_master_key()?;
            let res = decrypt(&ct);
            assert!(res.is_err(), "decrypt with wrong key must fail");
            Ok(())
        });
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
