// config/mod.rs — 全局配置管理

use anyhow::Result;
use clap::Subcommand;
use colored::*;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Subcommand)]
pub enum ConfigCommands {
    /// 查看所有全局配置
    List,
    /// 查看单条配置项
    Get { key: String },
    /// 修改配置项
    Set { key: String, value: String },
    /// 重置全局配置为默认
    Reset,
    /// 列出 RBAC 角色密钥
    RolesList,
    /// 添加/更新 RBAC 角色密钥 (role: reader/operator/admin)
    RolesAdd { key: String, role: String },
    /// 删除 RBAC 角色密钥
    RolesRemove { key: String },
    /// 启用/禁用 RBAC 门禁 (on/off)
    RolesEnable { state: String },
    /// 轮换 MLX API key (生成随机 key, 旧 key 存入 key_previous 供 24h 宽限回滚)
    RotateKey,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FusionConfig {
    // config_version: 升级迁移依据。缺失时迁移函数补默认, 不阻断解析。
    #[serde(default)]
    pub config_version: String,
    #[serde(default)]
    pub model: ModelConfig,
    #[serde(default)]
    pub kb: KbConfig,
    #[serde(default)]
    pub mlx: MlxConfig,
    #[serde(default)]
    pub modelhub: ModelhubConfig,
    #[serde(default)]
    pub rag: RagConfig,
    #[serde(default)]
    pub desk: DeskConfig,
    #[serde(default)]
    pub doc: DocConfig,
    #[serde(default)]
    pub memory: MemoryConfig,
    #[serde(default)]
    pub bench: BenchConfig,
    #[serde(default)]
    pub multinode: MultinodeConfig,
    #[serde(default)]
    pub auth: AuthConfig,
    #[serde(default)]
    pub log: LogConfig,
    // #52 限流熔断: Option → 老配置无此段时 None (用代码默认), 有段时覆盖默认。
    #[serde(default)]
    pub backpressure: Option<crate::utils::backpressure::BackpressureConfig>,
}

// RBAC 多租户角色门禁 (defense-in-depth): enabled=false 不设门禁 (默认, 保持旧行为);
// enabled=true 后按 FUSION_API_KEY 查 keys 表, mlx.api_key 隐式 admin (owner 永不锁死)。
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct AuthConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub keys: Vec<KeyEntry>,
    // #50 静态加密: true 时 save_config 加密 4 个 api_key 落盘 (enc: 前缀),
    // load_config 解密回内存明文。默认 false (向后兼容老明文配置)。
    #[serde(default)]
    pub encrypt_secrets: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KeyEntry {
    pub key: String,
    pub role: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    #[serde(default = "default_model_path")]
    pub default_path: String,
}

fn default_model_path() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion/models")
        .to_string_lossy()
        .to_string()
}

impl Default for ModelConfig {
    fn default() -> Self {
        Self {
            default_path: default_model_path(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KbConfig {
    #[serde(default = "default_kb_path")]
    pub default_path: String,
    #[serde(default = "default_kb_url")]
    pub base_url: String,
}

fn default_kb_path() -> String {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion/kb")
        .to_string_lossy()
        .to_string()
}

fn default_kb_url() -> String {
    "http://localhost:11434".to_string()
}

impl Default for KbConfig {
    fn default() -> Self {
        Self {
            default_path: default_kb_path(),
            base_url: default_kb_url(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MlxConfig {
    #[serde(default = "default_mlx_ctx")]
    pub default_ctx: u32,
    #[serde(default = "default_mlx_cache_enabled")]
    pub enable_cache: bool,
    #[serde(default = "default_mlx_url")]
    pub base_url: String,
    #[serde(default = "default_mlx_api_key")]
    pub api_key: String,
    #[serde(default = "default_mlx_cache_size")]
    pub cache_size: String,
    #[serde(default = "default_mlx_batch")]
    pub max_batch_size: u32,
    // HA failover: 主 base_url 不可达时按序尝试的备用网关列表 (如多节点 gateway)。
    #[serde(default)]
    pub failover_urls: Vec<String>,
    // 密钥轮换: rotated_at 记录上次轮换时间 (RFC3339), 供 doctor 算密钥年龄 (>90d 告警)。
    // key_previous 保留旧 key 供 24h 宽限期内回滚 (网关侧双 key 并存属上游, CLI 仅记录)。
    #[serde(default)]
    pub key_rotated_at: String,
    #[serde(default)]
    pub key_previous: String,
}

fn default_mlx_ctx() -> u32 {
    4096
}
fn default_mlx_cache_enabled() -> bool {
    true
}
fn default_mlx_url() -> String {
    "http://localhost:11432".to_string()
}
fn default_mlx_api_key() -> String {
    "fg-admin-key".to_string()
}
fn default_mlx_cache_size() -> String {
    "4GB".to_string()
}
fn default_mlx_batch() -> u32 {
    8
}

impl Default for MlxConfig {
    fn default() -> Self {
        Self {
            default_ctx: default_mlx_ctx(),
            enable_cache: default_mlx_cache_enabled(),
            base_url: default_mlx_url(),
            api_key: default_mlx_api_key(),
            cache_size: default_mlx_cache_size(),
            max_batch_size: default_mlx_batch(),
            failover_urls: Vec::new(),
            key_rotated_at: String::new(),
            key_previous: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelhubConfig {
    #[serde(default = "default_modelhub_url")]
    pub base_url: String,
}

fn default_modelhub_url() -> String {
    "http://localhost:11444".to_string()
}

impl Default for ModelhubConfig {
    fn default() -> Self {
        Self {
            base_url: default_modelhub_url(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagConfig {
    #[serde(default = "default_rag_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
}

fn default_rag_url() -> String {
    "http://localhost:11436".to_string()
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            base_url: default_rag_url(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeskConfig {
    #[serde(default = "default_desk_url")]
    pub base_url: String,
}

fn default_desk_url() -> String {
    "http://localhost:9000".to_string()
}

impl Default for DeskConfig {
    fn default() -> Self {
        Self {
            base_url: default_desk_url(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocConfig {
    #[serde(default = "default_doc_url")]
    pub base_url: String,
}

fn default_doc_url() -> String {
    "http://localhost:11449".to_string()
}

impl Default for DocConfig {
    fn default() -> Self {
        Self {
            base_url: default_doc_url(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MemoryConfig {
    #[serde(default = "default_memory_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
}

fn default_memory_url() -> String {
    "http://localhost:11435".to_string()
}

impl Default for MemoryConfig {
    fn default() -> Self {
        Self {
            base_url: default_memory_url(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BenchConfig {
    #[serde(default = "default_bench_url")]
    pub base_url: String,
}

fn default_bench_url() -> String {
    "http://localhost:11467".to_string()
}

impl Default for BenchConfig {
    fn default() -> Self {
        Self {
            base_url: default_bench_url(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MultinodeConfig {
    #[serde(default = "default_multinode_url")]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
}

fn default_multinode_url() -> String {
    "http://localhost:11452".to_string()
}

impl Default for MultinodeConfig {
    fn default() -> Self {
        Self {
            base_url: default_multinode_url(),
            api_key: String::new(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

fn default_log_level() -> String {
    "info".to_string()
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

pub const CURRENT_CONFIG_VERSION: &str = "0.4.2";

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            config_version: CURRENT_CONFIG_VERSION.to_string(),
            model: ModelConfig::default(),
            kb: KbConfig::default(),
            mlx: MlxConfig::default(),
            modelhub: ModelhubConfig::default(),
            rag: RagConfig::default(),
            desk: DeskConfig::default(),
            doc: DocConfig::default(),
            memory: MemoryConfig::default(),
            bench: BenchConfig::default(),
            multinode: MultinodeConfig::default(),
            auth: AuthConfig::default(),
            log: LogConfig::default(),
            backpressure: None,
        }
    }
}

// 升级迁移: 老配置缺 config_version 或版本低于当前 → 标记并补默认段。
// 各子结构体已 #[serde(default)], 缺段不报错; 此函数仅补 version 字段以便后续诊断。
fn migrate_config(mut cfg: FusionConfig) -> FusionConfig {
    if cfg.config_version != CURRENT_CONFIG_VERSION {
        tracing::info!(
            from = %cfg.config_version,
            to = CURRENT_CONFIG_VERSION,
            "Migrating config version"
        );
        cfg.config_version = CURRENT_CONFIG_VERSION.to_string();
    }
    cfg
}

pub fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion")
        .join("config.toml")
}

// P1-8: 仅解析, 不读盘不回退, 供 doctor 校验配置健康度 (是否可解析)。
pub fn parse_config(content: &str) -> Result<FusionConfig, toml::de::Error> {
    toml::from_str::<FusionConfig>(content).map(migrate_config)
}

pub fn load_config() -> FusionConfig {
    let path = get_config_path();
    if path.exists() {
        match std::fs::read_to_string(&path) {
            Ok(content) => match toml::from_str::<FusionConfig>(&content) {
                Ok(cfg) => {
                    let mut cfg = migrate_config(cfg);
                    // #50: 落盘密文 → 内存明文。解密失败不阻断 (回退明文, doctor 告警)。
                    decrypt_secrets(&mut cfg);
                    cfg
                }
                Err(e) => {
                    // F1/A4/O1 修复: 不再静默回退。配置损坏必须:
                    // (1) 终端可见 eprintln (交互用户立刻看到, 不依赖日志文件)
                    // (2) 备份坏文件到 config.toml.bak.<ts> 防止用户丢失手改内容
                    // (3) 再回退默认 (保证 CLI 可用)
                    // 否则单段笔误 → 全服务 "stopped" 无线索, 运维误查网络。
                    eprintln!(
                        "⚠️  config.toml parse failed, falling back to defaults: {}",
                        e
                    );
                    eprintln!("   Path: {}", path.display());
                    backup_corrupt_config(&path, &content);
                    tracing::error!(
                        path = %path.display(),
                        error = %e,
                        "Failed to parse config.toml, backed up and falling back to defaults"
                    );
                    FusionConfig::default()
                }
            },
            Err(e) => {
                eprintln!(
                    "⚠️  config.toml unreadable, falling back to defaults: {}",
                    e
                );
                tracing::error!(
                    path = %path.display(),
                    error = %e,
                    "Failed to read config.toml, falling back to defaults"
                );
                FusionConfig::default()
            }
        }
    } else {
        FusionConfig::default()
    }
}

// 备份损坏/旧配置, 防升级或笔误时丢失用户手改内容。时间戳来自系统 (非 workflow 限制, 此为运行时)。
fn backup_corrupt_config(path: &std::path::Path, content: &str) {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let bak = path.with_extension(format!("toml.bak.{}", ts));
    if std::fs::write(&bak, content).is_ok() {
        eprintln!("   Backed up original to: {}", bak.display());
    }
}

pub fn save_config(config: &FusionConfig) -> Result<()> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // #50: encrypt_secrets 开启时克隆一份, 加密 4 个 api_key 后落盘 (内存原件仍明文)。
    // 关闭时直接落盘明文 (向后兼容)。加密失败应 bail — 用户期望密文落盘, 静默落明文 = 泄露。
    let to_write = if config.auth.encrypt_secrets {
        let mut enc = config.clone();
        encrypt_secrets(&mut enc)?;
        enc
    } else {
        config.clone()
    };
    let content = toml::to_string_pretty(&to_write)?;
    std::fs::write(&path, content)?;
    // 写入后让 service 层配置缓存失效, 下次 from_config 重新读盘。
    crate::service::invalidate_config_cache();
    // 收敛文件权限: config.toml 明文存 API key (mlx/memory/multinode), 必须 0600,
    // 否则 umask 宽松环境下同机其他用户可读集群 token (R10)。
    restrict_config_perms(&path);
    Ok(())
}

// #50: 加密 4 个 api_key 字段 (mlx/rag/memory/multinode)。encrypt() 对 enc: 前缀幂等。
fn encrypt_secrets(cfg: &mut FusionConfig) -> Result<()> {
    cfg.mlx.api_key = crate::utils::crypto::encrypt(&cfg.mlx.api_key)?;
    cfg.rag.api_key = crate::utils::crypto::encrypt(&cfg.rag.api_key)?;
    cfg.memory.api_key = crate::utils::crypto::encrypt(&cfg.memory.api_key)?;
    cfg.multinode.api_key = crate::utils::crypto::encrypt(&cfg.multinode.api_key)?;
    // key_previous 也含旧密钥, 一并加密。
    cfg.mlx.key_previous = crate::utils::crypto::encrypt(&cfg.mlx.key_previous)?;
    Ok(())
}

// #50: 解密上述字段。decrypt() 对明文幂等 (passthrough), 对 enc: 前缀解密。
// master.key 缺失时 decrypt 返回原文 (不 bail), doctor 负责告警。
fn decrypt_secrets(cfg: &mut FusionConfig) {
    // 逐字段解密, 单字段失败不影响其余 (尽最大努力恢复, 失败字段保留密文)。
    if let Ok(v) = crate::utils::crypto::decrypt(&cfg.mlx.api_key) {
        cfg.mlx.api_key = v;
    }
    if let Ok(v) = crate::utils::crypto::decrypt(&cfg.rag.api_key) {
        cfg.rag.api_key = v;
    }
    if let Ok(v) = crate::utils::crypto::decrypt(&cfg.memory.api_key) {
        cfg.memory.api_key = v;
    }
    if let Ok(v) = crate::utils::crypto::decrypt(&cfg.multinode.api_key) {
        cfg.multinode.api_key = v;
    }
    if let Ok(v) = crate::utils::crypto::decrypt(&cfg.mlx.key_previous) {
        cfg.mlx.key_previous = v;
    }
}

#[cfg(unix)]
fn restrict_config_perms(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(0o600);
        if let Err(e) = std::fs::set_permissions(path, perms) {
            tracing::warn!(path = %path.display(), error = %e, "Failed to chmod 0600 config.toml");
        }
    }
}

#[cfg(not(unix))]
fn restrict_config_perms(_path: &std::path::Path) {}

pub async fn handle_config(action: ConfigCommands) -> Result<()> {
    match action {
        ConfigCommands::List => list_config(),
        ConfigCommands::Get { key } => get_config(key),
        ConfigCommands::Set { key, value } => set_config(key, value).await,
        ConfigCommands::Reset => reset_config().await,
        ConfigCommands::RolesList => roles_list(),
        ConfigCommands::RolesAdd { key, role } => roles_add(key, role),
        ConfigCommands::RolesRemove { key } => roles_remove(key),
        ConfigCommands::RolesEnable { state } => roles_enable(state).await,
        ConfigCommands::RotateKey => rotate_key(),
    }
}

fn list_config() -> Result<()> {
    let config = load_config();
    let config_str = toml::to_string_pretty(&config)?;
    println!();
    println!("{}", "⚙️  Fusion Configuration".bold());
    println!("  Path: {}", get_config_path().display().to_string().cyan());
    println!();
    println!("{}", config_str);
    println!("  Use `fusion config set <key> <value>` to modify.");
    println!("  Built-in keys: config-version, model.default-path, kb.default-path, kb.base-url,");
    println!(
        "    mlx.default-ctx, mlx.enable-cache, mlx.base-url, mlx.api-key, mlx.cache-size, mlx.max-batch-size,"
    );
    println!("    modelhub.base-url, rag.base-url, desk.base-url, doc.base-url, log.level,");
    println!(
        "    memory.base-url, memory.api-key, bench.base-url, multinode.base-url, multinode.api-key"
    );
    Ok(())
}

fn get_config(key: String) -> Result<()> {
    let config = load_config();
    let value = match key.as_str() {
        "config-version" => config.config_version.clone(),
        "model.default-path" => config.model.default_path.clone(),
        "kb.default-path" => config.kb.default_path.clone(),
        "kb.base-url" => config.kb.base_url.clone(),
        "mlx.default-ctx" => config.mlx.default_ctx.to_string(),
        "mlx.enable-cache" => config.mlx.enable_cache.to_string(),
        "mlx.base-url" => config.mlx.base_url.clone(),
        "mlx.api-key" => config.mlx.api_key.clone(),
        "mlx.cache-size" => config.mlx.cache_size.clone(),
        "mlx.max-batch-size" => config.mlx.max_batch_size.to_string(),
        "mlx.failover-urls" => config.mlx.failover_urls.join(","),
        "mlx.key-rotated-at" => config.mlx.key_rotated_at.clone(),
        "mlx.key-previous" => config.mlx.key_previous.clone(),
        "modelhub.base-url" => config.modelhub.base_url.clone(),
        "rag.base-url" => config.rag.base_url.clone(),
        "rag.api-key" => config.rag.api_key.clone(),
        "desk.base-url" => config.desk.base_url.clone(),
        "doc.base-url" => config.doc.base_url.clone(),
        "memory.base-url" => config.memory.base_url.clone(),
        "memory.api-key" => config.memory.api_key.clone(),
        "bench.base-url" => config.bench.base_url.clone(),
        "multinode.base-url" => config.multinode.base_url.clone(),
        "multinode.api-key" => config.multinode.api_key.clone(),
        "log.level" => config.log.level.clone(),
        "auth.encrypt-secrets" => config.auth.encrypt_secrets.to_string(),
        "backpressure.capacity" => bp_get(&config, |c| c.capacity.to_string()),
        "backpressure.refill-rate" => bp_get(&config, |c| c.refill_rate.to_string()),
        "backpressure.failure-threshold" => bp_get(&config, |c| c.failure_threshold.to_string()),
        "backpressure.open-secs" => bp_get(&config, |c| c.open_secs.to_string()),
        "backpressure.breaker-enabled" => bp_get(&config, |c| c.breaker_enabled.to_string()),
        _ => {
            println!("{} Unknown config key: {}", "❌".red(), key.cyan());
            println!(
                "  Available keys: config-version, model.default-path, kb.default-path, kb.base-url, mlx.default-ctx, mlx.enable-cache, mlx.base-url, mlx.api-key, mlx.cache-size, mlx.max-batch-size, mlx.failover-urls, mlx.key-rotated-at, mlx.key-previous, modelhub.base-url, rag.base-url, rag.api-key, desk.base-url, doc.base-url, memory.base-url, memory.api-key, bench.base-url, multinode.base-url, multinode.api-key, log.level, auth.encrypt-secrets, backpressure.capacity, backpressure.refill-rate, backpressure.failure-threshold, backpressure.open-secs, backpressure.breaker-enabled"
            );
            return Ok(());
        }
    };
    println!("{} {} = {}", "⚙️".cyan(), key.cyan(), value.green());
    Ok(())
}

async fn set_config(key: String, value: String) -> Result<()> {
    let mut config = load_config();
    match key.as_str() {
        "model.default-path" => config.model.default_path = value.clone(),
        "kb.default-path" => config.kb.default_path = value.clone(),
        "kb.base-url" => config.kb.base_url = value.clone(),
        "mlx.default-ctx" => {
            config.mlx.default_ctx = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid integer"))?
        }
        "mlx.enable-cache" => {
            config.mlx.enable_cache = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean (true/false)"))?
        }
        "mlx.base-url" => config.mlx.base_url = value.clone(),
        "mlx.api-key" => config.mlx.api_key = value.clone(),
        "mlx.cache-size" => config.mlx.cache_size = value.clone(),
        "mlx.max-batch-size" => {
            config.mlx.max_batch_size = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid integer"))?
        }
        "mlx.failover-urls" => {
            config.mlx.failover_urls = value
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        "mlx.key-rotated-at" => config.mlx.key_rotated_at = value.clone(),
        "mlx.key-previous" => config.mlx.key_previous = value.clone(),
        "modelhub.base-url" => config.modelhub.base_url = value.clone(),
        "rag.base-url" => config.rag.base_url = value.clone(),
        "rag.api-key" => config.rag.api_key = value.clone(),
        "desk.base-url" => config.desk.base_url = value.clone(),
        "doc.base-url" => config.doc.base_url = value.clone(),
        "memory.base-url" => config.memory.base_url = value.clone(),
        "memory.api-key" => config.memory.api_key = value.clone(),
        "bench.base-url" => config.bench.base_url = value.clone(),
        "multinode.base-url" => config.multinode.base_url = value.clone(),
        "multinode.api-key" => config.multinode.api_key = value.clone(),
        "log.level" => config.log.level = value.clone(),
        "auth.encrypt-secrets" => {
            let on: bool = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean (true/false)"))?;
            config.auth.encrypt_secrets = on;
            // 开启时立即生成 master.key (避免下次 save 才生成, doctor 提前可见)。
            if on {
                crate::utils::crypto::ensure_master_key()?;
            }
        }
        "backpressure.capacity" => bp_set(&mut config, |c| {
            c.capacity = value.parse().map_err(|_| anyhow::anyhow!("Invalid u32"))?;
            Ok(())
        })?,
        "backpressure.refill-rate" => bp_set(&mut config, |c| {
            c.refill_rate = value.parse().map_err(|_| anyhow::anyhow!("Invalid u32"))?;
            Ok(())
        })?,
        "backpressure.failure-threshold" => bp_set(&mut config, |c| {
            c.failure_threshold = value.parse().map_err(|_| anyhow::anyhow!("Invalid u32"))?;
            Ok(())
        })?,
        "backpressure.open-secs" => bp_set(&mut config, |c| {
            c.open_secs = value.parse().map_err(|_| anyhow::anyhow!("Invalid u64"))?;
            Ok(())
        })?,
        "backpressure.breaker-enabled" => bp_set(&mut config, |c| {
            c.breaker_enabled = value
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid boolean (true/false)"))?;
            Ok(())
        })?,
        _ => {
            println!("{} Unknown config key: {}", "❌".red(), key.cyan());
            return Ok(());
        }
    }
    save_config(&config)?;
    println!("{} Set {} = {}", "✅".green(), key.cyan(), value.green());
    Ok(())
}

async fn reset_config() -> Result<()> {
    let confirm = dialoguer::Confirm::new()
        .with_prompt("Reset all configuration to defaults?")
        .default(false)
        .interact()?;

    if confirm {
        let config = FusionConfig::default();
        save_config(&config)?;
        println!("{} Configuration reset to defaults.", "✅".green());
    } else {
        println!("{} Cancelled.", "ℹ️".blue());
    }
    Ok(())
}

fn valid_role(role: &str) -> bool {
    matches!(
        role.to_ascii_lowercase().as_str(),
        "reader" | "operator" | "admin"
    )
}

fn roles_list() -> Result<()> {
    let config = load_config();
    println!();
    println!("{}", "🔑  RBAC Role Keys".bold());
    println!(
        "  Enabled: {}",
        if config.auth.enabled {
            "on".green()
        } else {
            "off".red()
        }
    );
    println!(
        "  Owner key (implicit admin): mlx.api-key = {}",
        "***".yellow()
    );
    println!();
    if config.auth.keys.is_empty() {
        println!("  No role keys configured. Use `fusion config roles-add <key> <role>`.");
    } else {
        for entry in &config.auth.keys {
            println!(
                "  role={:<10} key={}",
                entry.role.cyan(),
                entry.key.yellow()
            );
        }
    }
    println!();
    println!("  Roles: reader (info+inference) / operator (services+models) / admin (config+init)");
    println!("  Auth via env FUSION_API_KEY; mlx.api-key always resolves to admin.");
    Ok(())
}

fn roles_add(key: String, role: String) -> Result<()> {
    if !valid_role(&role) {
        anyhow::bail!("invalid role '{}': must be reader/operator/admin", role);
    }
    let mut config = load_config();
    let normalized = role.to_ascii_lowercase();
    if let Some(entry) = config.auth.keys.iter_mut().find(|e| e.key == key) {
        entry.role = normalized.clone();
        tracing::info!(key = %key, role = %normalized, "updated existing role key");
    } else {
        config.auth.keys.push(KeyEntry {
            key: key.clone(),
            role: normalized.clone(),
        });
        tracing::info!(key = %key, role = %normalized, "added new role key");
    }
    save_config(&config)?;
    println!(
        "{} Role key added: {} = {}",
        "✅".green(),
        key.yellow(),
        normalized.cyan()
    );
    if !config.auth.enabled {
        println!(
            "{} RBAC not enabled yet. Run `fusion config roles-enable on` to activate.",
            "ℹ️".blue()
        );
    }
    Ok(())
}

fn roles_remove(key: String) -> Result<()> {
    let mut config = load_config();
    let before = config.auth.keys.len();
    config.auth.keys.retain(|e| e.key != key);
    if config.auth.keys.len() == before {
        println!("{} No matching role key for: {}", "⚠️".red(), key.yellow());
        return Ok(());
    }
    save_config(&config)?;
    tracing::info!(key = %key, "removed role key");
    println!("{} Role key removed: {}", "✅".green(), key.yellow());
    Ok(())
}

async fn roles_enable(state: String) -> Result<()> {
    let enabled = match state.to_ascii_lowercase().as_str() {
        "on" | "true" | "1" | "enable" => true,
        "off" | "false" | "0" | "disable" => false,
        _ => anyhow::bail!("invalid state '{}': use on/off", state),
    };
    let mut config = load_config();
    config.auth.enabled = enabled;
    save_config(&config)?;
    tracing::info!(enabled, "toggled RBAC");
    if enabled && config.auth.keys.is_empty() {
        println!(
            "{} RBAC enabled, but no role keys set. Only mlx.api-key (admin) will pass operator/admin commands.",
            "⚠️".yellow()
        );
        println!("   Add keys: `fusion config roles-add <key> <role>`");
    } else {
        println!(
            "{} RBAC {}",
            "✅".green(),
            if enabled {
                "enabled".green()
            } else {
                "disabled".red()
            }
        );
    }
    Ok(())
}

// 密钥轮换: 生成 32 字节随机 hex key, 旧 key→key_previous (24h 宽限回滚), 当前→新,
// 盖时间戳 key_rotated_at。无 rand crate 依赖 — 用 std::collections::RandomState
// (SipHash, OS 熵种子) 取 8 字节再 hex, 循环 4 次拼 64 hex 字符 (256-bit)。
fn random_hex_key() -> String {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    let mut out = String::with_capacity(64);
    for _ in 0..4 {
        let mut hasher = RandomState::new().build_hasher();
        // 混入纳秒时间戳增加时序熵 (RandomState 本身已 OS-entropy, 此为额外)。
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0);
        hasher.write_u64(nanos);
        let v = hasher.finish();
        out.push_str(&format!("{:016x}", v));
    }
    out
}

// #52 backpressure 配置读写 helper: backpressure 是 Option, 缺段用默认。
fn bp_get<F: FnOnce(&crate::utils::backpressure::BackpressureConfig) -> String>(
    config: &FusionConfig,
    f: F,
) -> String {
    match &config.backpressure {
        Some(bp) => f(bp),
        None => f(&crate::utils::backpressure::BackpressureConfig::default()),
    }
}

fn bp_set<F: FnOnce(&mut crate::utils::backpressure::BackpressureConfig) -> Result<()>>(
    config: &mut FusionConfig,
    f: F,
) -> Result<()> {
    let bp = config
        .backpressure
        .get_or_insert_with(crate::utils::backpressure::BackpressureConfig::default);
    f(bp)
}

fn rotate_key() -> Result<()> {
    let mut config = load_config();
    let old_key = config.mlx.api_key.clone();
    if old_key.is_empty() {
        anyhow::bail!("mlx.api-key is empty; set an initial key before rotating");
    }
    let new_key = random_hex_key();
    config.mlx.key_previous = old_key;
    config.mlx.api_key = new_key.clone();
    config.mlx.key_rotated_at = chrono::Utc::now().to_rfc3339();
    save_config(&config)?;
    tracing::info!(rotated_at = %config.mlx.key_rotated_at, "rotated MLX api key");
    println!("{} Rotated MLX API key.", "✅".green());
    println!("  {} new key: {}", "🔑".cyan(), new_key.yellow());
    println!(
        "  {} old key retained in mlx.key_previous for 24h rollback grace window.",
        "ℹ️".blue()
    );
    println!("  Update the gateway to accept the new key, then remove the old key after 24h.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_auth_disabled() {
        let config = FusionConfig::default();
        assert!(!config.auth.enabled);
        assert!(config.auth.keys.is_empty());
    }

    #[test]
    fn test_auth_section_parses() {
        let toml = r#"
[auth]
enabled = true
[[auth.keys]]
key = "op-token-1"
role = "operator"
[[auth.keys]]
key = "reader-token-1"
role = "reader"
"#;
        let cfg: FusionConfig = toml::from_str(toml).expect("auth section must parse");
        assert!(cfg.auth.enabled);
        assert_eq!(cfg.auth.keys.len(), 2);
        assert_eq!(cfg.auth.keys[0].role, "operator");
        assert_eq!(cfg.auth.keys[1].role, "reader");
    }

    #[test]
    fn test_auth_absent_defaults_disabled() {
        let toml = "[mlx]\nbase_url = \"http://x\"\n";
        let cfg: FusionConfig = toml::from_str(toml).expect("config without auth must parse");
        assert!(!cfg.auth.enabled);
        assert!(cfg.auth.keys.is_empty());
    }

    #[test]
    fn test_default_mlx_uses_gateway_port() {
        let config = FusionConfig::default();
        assert_eq!(config.mlx.base_url, "http://localhost:11432");
    }

    #[test]
    fn test_default_mlx_api_key_is_admin_key() {
        let config = FusionConfig::default();
        assert_eq!(config.mlx.api_key, "fg-admin-key");
    }

    #[test]
    fn test_default_mlx_ctx_and_cache() {
        let config = FusionConfig::default();
        assert_eq!(config.mlx.default_ctx, 4096);
        assert!(config.mlx.enable_cache);
        assert_eq!(config.mlx.cache_size, "4GB");
        assert_eq!(config.mlx.max_batch_size, 8);
    }

    #[test]
    fn test_default_direct_service_ports_not_gateway() {
        let config = FusionConfig::default();
        assert_eq!(config.kb.base_url, "http://localhost:11434");
        assert_eq!(config.modelhub.base_url, "http://localhost:11444");
        assert_eq!(config.rag.base_url, "http://localhost:11436");
        assert_eq!(config.desk.base_url, "http://localhost:9000");
        assert_eq!(config.doc.base_url, "http://localhost:11449");
    }

    #[test]
    fn test_default_new_service_ports() {
        let config = FusionConfig::default();
        assert_eq!(config.memory.base_url, "http://localhost:11435");
        assert_eq!(config.bench.base_url, "http://localhost:11467");
        assert_eq!(config.multinode.base_url, "http://localhost:11452");
    }

    #[test]
    fn test_default_memory_api_key_empty() {
        // fm-server B5: 未配 key 则 HTTP 拒启, 故 CLI 默认空 (auth_header → None, 仅用公开端点)。
        let config = FusionConfig::default();
        assert_eq!(config.memory.api_key, "");
    }

    #[test]
    fn test_default_multinode_api_key_empty() {
        // Master BearerAuthMiddleware: 除 /api/health* 外强制 token。CLI 默认空 → 仅 health 可用;
        // 配 `fusion config set multinode.api-key <FUSION_CLUSTER_TOKEN>` 后解锁 cluster/sync。
        let config = FusionConfig::default();
        assert_eq!(config.multinode.api_key, "");
    }

    #[test]
    fn test_default_log_level_info() {
        let config = FusionConfig::default();
        assert_eq!(config.log.level, "info");
    }

    #[test]
    fn test_default_config_version() {
        let config = FusionConfig::default();
        assert_eq!(config.config_version, CURRENT_CONFIG_VERSION);
    }

    // A5 修复回归: 老配置只含 [mlx] 段, 缺其余所有段 → 应解析成功并补默认, 不再整文件回退。
    #[test]
    fn test_partial_config_parses_with_defaults() {
        let toml = r#"
[mlx]
base_url = "http://10.0.0.5:11432"
api_key = "remote-key"
"#;
        let cfg: FusionConfig = toml::from_str(toml).expect("partial config must parse");
        assert_eq!(cfg.mlx.base_url, "http://10.0.0.5:11432");
        assert_eq!(cfg.mlx.api_key, "remote-key");
        // 缺失段补默认
        assert_eq!(cfg.kb.base_url, "http://localhost:11434");
        assert_eq!(cfg.memory.base_url, "http://localhost:11435");
        assert_eq!(cfg.multinode.base_url, "http://localhost:11452");
    }

    // 单段笔误 (此处 [mlx] 缺右括号) 仍应解析失败 → load_config 走备份+回退路径。
    #[test]
    fn test_malformed_toml_fails_to_parse() {
        let toml = "[mlx\nbase_url = \"x\"";
        let res: Result<FusionConfig, _> = toml::from_str(toml);
        assert!(res.is_err(), "malformed TOML must fail parse");
    }

    // #49: key_rotated_at / key_previous 默认空, 老配置缺字段不阻断解析。
    #[test]
    fn test_mlx_key_rotation_fields_default_empty() {
        let config = FusionConfig::default();
        assert!(config.mlx.key_rotated_at.is_empty());
        assert!(config.mlx.key_previous.is_empty());
    }

    #[test]
    fn test_mlx_key_rotation_parses() {
        let toml = r#"
[mlx]
base_url = "http://localhost:11432"
api_key = "newkey"
key_rotated_at = "2026-09-01T00:00:00Z"
key_previous = "oldkey"
"#;
        let cfg: FusionConfig = toml::from_str(toml).expect("rotation fields must parse");
        assert_eq!(cfg.mlx.api_key, "newkey");
        assert_eq!(cfg.mlx.key_previous, "oldkey");
        assert_eq!(cfg.mlx.key_rotated_at, "2026-09-01T00:00:00Z");
    }

    // 随机 key 生成: 64 hex 字符 (256-bit), 两次调用几乎不可能相同 (不要求严格唯一,
    // 仅要求足够熵防爆破; 两次同值概率可忽略)。
    #[test]
    fn test_random_hex_key_length_and_format() {
        let k = random_hex_key();
        assert_eq!(k.len(), 64);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
    }

    // #50: encrypt_secrets 默认 false (向后兼容老明文配置)。
    #[test]
    fn test_encrypt_secrets_default_off() {
        let config = FusionConfig::default();
        assert!(!config.auth.encrypt_secrets);
    }

    #[test]
    fn test_encrypt_secrets_parses() {
        let toml = r#"
[auth]
encrypt_secrets = true
"#;
        let cfg: FusionConfig = toml::from_str(toml).expect("encrypt_secrets must parse");
        assert!(cfg.auth.encrypt_secrets);
    }
}
