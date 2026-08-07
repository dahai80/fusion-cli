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
    pub model: ModelConfig,
    pub kb: KbConfig,
    pub mlx: MlxConfig,
    pub modelhub: ModelhubConfig,
    pub rag: RagConfig,
    pub desk: DeskConfig,
    pub doc: DocConfig,
    pub log: LogConfig,
    #[serde(default)]
    pub gateway: Option<GatewayConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GatewayConfig {
    pub enabled: bool,
    #[serde(default = "default_gateway_url")]
    pub base_url: String,
}

fn default_gateway_url() -> String {
    "http://localhost:11432".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelConfig {
    pub default_path: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KbConfig {
    pub default_path: String,
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MlxConfig {
    pub default_ctx: u32,
    pub enable_cache: bool,
    pub base_url: String,
    #[serde(default = "default_mlx_api_key")]
    pub api_key: String,
    pub cache_size: String,
    pub max_batch_size: u32,
}

fn default_mlx_api_key() -> String {
    "fg-admin-key".to_string()
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModelhubConfig {
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RagConfig {
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DeskConfig {
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocConfig {
    pub base_url: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LogConfig {
    pub level: String,
}

impl Default for FusionConfig {
    fn default() -> Self {
        Self {
            model: ModelConfig {
                default_path: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".fusion/models")
                    .to_string_lossy()
                    .to_string(),
            },
            kb: KbConfig {
                default_path: dirs::home_dir()
                    .unwrap_or_default()
                    .join(".fusion/kb")
                    .to_string_lossy()
                    .to_string(),
                base_url: "http://localhost:11434".to_string(),
            },
            mlx: MlxConfig {
                default_ctx: 4096,
                enable_cache: true,
                base_url: "http://localhost:11432".to_string(),
                api_key: default_mlx_api_key(),
                cache_size: "4GB".to_string(),
                max_batch_size: 8,
            },
            modelhub: ModelhubConfig {
                base_url: "http://localhost:11444".to_string(),
            },
            rag: RagConfig {
                base_url: "http://localhost:11436".to_string(),
            },
            desk: DeskConfig {
                base_url: "http://localhost:9000".to_string(),
            },
            doc: DocConfig {
                base_url: "http://localhost:11449".to_string(),
            },
            log: LogConfig {
                level: "info".to_string(),
            },
            gateway: None,
        }
    }
}

pub fn get_config_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_default()
        .join(".fusion")
        .join("config.toml")
}

pub fn load_config() -> FusionConfig {
    let path = get_config_path();
    if path.exists() {
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
            .unwrap_or_default()
    } else {
        FusionConfig::default()
    }
}

pub fn save_config(config: &FusionConfig) -> Result<()> {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    std::fs::write(&path, content)?;
    Ok(())
}

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
    println!("  Built-in keys: model.default-path, kb.default-path, kb.base-url,");
    println!(
        "    mlx.default-ctx, mlx.enable-cache, mlx.base-url, mlx.api-key, mlx.cache-size, mlx.max-batch-size,"
    );
    println!("    modelhub.base-url, rag.base-url, desk.base-url, doc.base-url, log.level");
    Ok(())
}

fn get_config(key: String) -> Result<()> {
    let config = load_config();
    let value = match key.as_str() {
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
        "log.level" => config.log.level.clone(),
        _ => {
            println!("{} Unknown config key: {}", "❌".red(), key.cyan());
            println!(
                "  Available keys: model.default-path, kb.default-path, kb.base-url, mlx.default-ctx, mlx.enable-cache, mlx.base-url, mlx.api-key, mlx.cache-size, mlx.max-batch-size, modelhub.base-url, rag.base-url, desk.base-url, doc.base-url, log.level"
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
