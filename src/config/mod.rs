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
    pub log: LogConfig,
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
}

fn default_rag_url() -> String {
    "http://localhost:11436".to_string()
}

impl Default for RagConfig {
    fn default() -> Self {
        Self {
            base_url: default_rag_url(),
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

pub const CURRENT_CONFIG_VERSION: &str = "0.3.5";

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
            log: LogConfig::default(),
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
                Ok(cfg) => migrate_config(cfg),
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
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    // 写入后让 service 层配置缓存失效, 下次 from_config 重新读盘。
    crate::service::invalidate_config_cache();
    // 收敛文件权限: config.toml 明文存 API key (mlx/memory/multinode), 必须 0600,
    // 否则 umask 宽松环境下同机其他用户可读集群 token (R10)。
    restrict_config_perms(&path);
    Ok(())
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
        "modelhub.base-url" => config.modelhub.base_url.clone(),
        "rag.base-url" => config.rag.base_url.clone(),
        "desk.base-url" => config.desk.base_url.clone(),
        "doc.base-url" => config.doc.base_url.clone(),
        "memory.base-url" => config.memory.base_url.clone(),
        "memory.api-key" => config.memory.api_key.clone(),
        "bench.base-url" => config.bench.base_url.clone(),
        "multinode.base-url" => config.multinode.base_url.clone(),
        "multinode.api-key" => config.multinode.api_key.clone(),
        "log.level" => config.log.level.clone(),
        _ => {
            println!("{} Unknown config key: {}", "❌".red(), key.cyan());
            println!(
                "  Available keys: config-version, model.default-path, kb.default-path, kb.base-url, mlx.default-ctx, mlx.enable-cache, mlx.base-url, mlx.api-key, mlx.cache-size, mlx.max-batch-size, modelhub.base-url, rag.base-url, desk.base-url, doc.base-url, memory.base-url, memory.api-key, bench.base-url, multinode.base-url, multinode.api-key, log.level"
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
        "modelhub.base-url" => config.modelhub.base_url = value.clone(),
        "rag.base-url" => config.rag.base_url = value.clone(),
        "desk.base-url" => config.desk.base_url = value.clone(),
        "doc.base-url" => config.doc.base_url = value.clone(),
        "memory.base-url" => config.memory.base_url = value.clone(),
        "memory.api-key" => config.memory.api_key = value.clone(),
        "bench.base-url" => config.bench.base_url = value.clone(),
        "multinode.base-url" => config.multinode.base_url = value.clone(),
        "multinode.api-key" => config.multinode.api_key = value.clone(),
        "log.level" => config.log.level = value.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
