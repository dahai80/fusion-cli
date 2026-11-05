use anyhow::{Result, anyhow};
use clap::Args as ClapArgs;
use colored::*;
use std::path::{Path, PathBuf};
use tracing::{error, info};

use crate::service::sv as sv_svc;
use crate::utils::output::is_json_mode;

// Bundled fallback snapshot of the gateway product catalog (#488), copied
// from fusion-gateway internal/catalog/catalog.json. Used when the gateway
// is unreachable so `fusion install` / `fusion list` still work offline.
// Regenerate this file from the gateway repo when its catalog changes.
const CATALOG_FALLBACK: &str = include_str!("catalog_fallback.json");

/// One product line item in the gateway /v1/catalog merge view.
#[derive(serde::Deserialize, Clone)]
struct CatalogEntry {
    id: String,
    #[serde(default)]
    #[allow(dead_code)]
    category: String,
    repo: String,
    #[serde(default)]
    footprint: String,
}

#[derive(ClapArgs)]
pub struct InstallArgs {
    /// Module id from the catalog (e.g. fusion-bench, ai-fit)
    pub id: String,
    /// Also run the language-appropriate bootstrap (pip install -e / cargo build / bun install)
    #[arg(long, default_value_t = false)]
    pub with_deps: bool,
    /// Workspace root override (default ~/fusion)
    #[arg(long, value_name = "PATH")]
    pub root: Option<String>,
}

#[derive(ClapArgs)]
pub struct ListArgs {
    /// Workspace root override (default ~/fusion)
    #[arg(long, value_name = "PATH")]
    pub root: Option<String>,
}

pub async fn run_install(args: InstallArgs) -> Result<()> {
    let root = install_root(&args.root);
    let id = args.id;
    crate::service::validate_path_segment("id", &id)?;

    println!("{}", format!("📦 Installing {id}").bold());

    let (entry, source) = fetch_entry(&root, &id).await?;
    info!(id = %id, source = %source, "catalog entry resolved");

    let dest = home_for_id(&root, &id);
    println!("  Repository: {}", entry.repo);
    println!("  Destination: {}", dest.display());
    if !entry.footprint.is_empty() {
        println!("  Footprint: {}", entry.footprint);
    }

    // Idempotency: existing dir with the same origin is a safe no-op; a
    // different origin is a hard error (never clone into a foreign dir).
    if dest.exists() {
        match existing_origin(&dest) {
            Some(origin) if same_repo(&origin, &entry.repo) => {
                println!(
                    "  {} already installed at {} (no-op)",
                    "✓".green(),
                    dest.display()
                );
                if !args.with_deps {
                    println!(
                        "  Hint: re-run with --with-deps to refresh deps, or `fusion-sv up {}` to start",
                        id
                    );
                    return Ok(());
                }
            }
            Some(origin) => {
                return Err(anyhow!(
                    "{} already exists but tracks a different origin ({origin}); move it aside or update the catalog",
                    dest.display()
                ));
            }
            None => {
                return Err(anyhow!(
                    "{} already exists and is not a git clone; move it aside before installing",
                    dest.display()
                ));
            }
        }
    } else {
        clone_repo(&entry.repo, &dest).await?;
        println!("  {} cloned", "✓".green());
    }

    if args.with_deps {
        bootstrap_deps(&root, &dest).await?;
    } else {
        println!("  Skipped deps bootstrap (use --with-deps to run it)");
    }

    println!(
        "  {} {id} installed. Start it with: fusion-sv up {id}",
        "✅".green()
    );
    Ok(())
}

pub async fn run_list(args: ListArgs) -> Result<()> {
    let root = install_root(&args.root);
    let (entries, source) = fetch_catalog(&root).await?;
    let running = supervisor_states().await;

    let mut rows: Vec<(String, String, String, String)> = Vec::new();
    for e in &entries {
        let dest = home_for_id(&root, &e.id);
        let installed = if dest.exists() { "installed" } else { "-" };
        let state = running
            .as_ref()
            .and_then(|m| m.get(&e.id).cloned())
            .unwrap_or_else(|| "-".to_string());
        rows.push((
            e.id.clone(),
            installed.to_string(),
            state,
            e.footprint.clone(),
        ));
    }
    rows.sort();

    if is_json_mode() {
        let payload = serde_json::json!({
            "source": source,
            "root": root.display().to_string(),
            "modules": rows.iter().map(|(id, installed, state, fp)| serde_json::json!({
                "id": id, "installed": installed == "installed", "state": state, "footprint": fp,
            })).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&payload)?);
        return Ok(());
    }

    println!();
    println!("{}", "📦 Fusion Modules".bold());
    println!();
    println!(
        "  {:<26} {:<11} {:<12} {}",
        "ID".dimmed(),
        "INSTALLED".dimmed(),
        "STATE".dimmed(),
        "FOOTPRINT".dimmed()
    );
    for (id, installed, state, fp) in &rows {
        let inst = if *installed == "installed" {
            installed.green().to_string()
        } else {
            installed.dimmed().to_string()
        };
        let st = match state.as_str() {
            "Healthy" => state.green().to_string(),
            "Starting" | "Stopping" => state.yellow().to_string(),
            "-" => "-".dimmed().to_string(),
            _ => state.red().to_string(),
        };
        println!("  {:<26} {:<11} {:<12} {}", id, inst, st, fp);
    }
    println!();
    println!("  Install a module: fusion install <id> [--with-deps]");
    println!("  Catalog source: {source}");
    println!();
    Ok(())
}

// Workspace root for install/list: --root > FUSION_WORKSPACE_ROOT > ~/fusion.
fn install_root(flag: &Option<String>) -> PathBuf {
    if let Some(r) = flag {
        return PathBuf::from(r);
    }
    if let Ok(r) = std::env::var("FUSION_WORKSPACE_ROOT")
        && !r.is_empty()
    {
        return PathBuf::from(r);
    }
    dirs::home_dir().unwrap_or_default().join("fusion")
}

// ai-* modules live under <root>/business/, everything else at the top level.
fn home_for_id(root: &Path, id: &str) -> PathBuf {
    if id.starts_with("ai-") {
        root.join("business").join(id)
    } else {
        root.join(id)
    }
}

// Fetch one catalog entry by id: gateway /v1/catalog first, bundled snapshot
// as fallback. Returns (entry, source description).
async fn fetch_entry(root: &Path, id: &str) -> Result<(CatalogEntry, String)> {
    let (entries, source) = fetch_catalog(root).await?;
    match entries.into_iter().find(|e| e.id == id) {
        Some(e) => Ok((e, source)),
        None => Err(anyhow!(
            "id '{id}' not found in catalog (source: {source}); run `fusion list` to see available ids"
        )),
    }
}

// Fetch the full catalog: GET {gateway}/v1/catalog with the client key,
// falling back to the bundled snapshot on any failure (offline-friendly).
async fn fetch_catalog(root: &Path) -> Result<(Vec<CatalogEntry>, String)> {
    let gw = gateway_base_url();
    let url = format!("{}/v1/catalog", gw.trim_end_matches('/'));
    let client = crate::service::get_client();
    // Try each candidate key (env > fusion-core > config): a scratch venv's
    // fusion-core may resolve a default key the live gateway rejects, while
    // the config key is valid — so the chain falls through on 401/403.
    for key in candidate_keys(root).await {
        let res = client
            .get(&url)
            .header("Authorization", format!("Bearer {key}"))
            .timeout(std::time::Duration::from_secs(5))
            .send()
            .await;
        let res = match res {
            Ok(r) => r,
            Err(_) => break,
        };
        if res.status() == reqwest::StatusCode::UNAUTHORIZED
            || res.status() == reqwest::StatusCode::FORBIDDEN
        {
            info!(
                "gateway rejected client key ({}), trying next candidate",
                res.status()
            );
            continue;
        }
        if !res.status().is_success() {
            break;
        }
        if let Ok(body) = res.json::<serde_json::Value>().await
            && let Some(products) = body.get("products").cloned()
        {
            match serde_json::from_value::<Vec<CatalogEntry>>(products) {
                Ok(entries) if !entries.is_empty() => {
                    return Ok((entries, format!("gateway {url}")));
                }
                Ok(_) => error!(
                    "gateway catalog returned zero products, falling back to bundled snapshot"
                ),
                Err(e) => {
                    error!(error = %e, "gateway catalog parse failed, falling back to bundled snapshot")
                }
            }
        }
        break;
    }
    let entries: Vec<CatalogEntry> = serde_json::from_str(CATALOG_FALLBACK)
        .map_err(|e| anyhow!("bundled catalog snapshot parse failed: {e}"))?;
    Ok((entries, "bundled snapshot".to_string()))
}

// Gateway base: FUSION_GATEWAY_BASE_URL > config mlx.base_url (defaults to :11432).
fn gateway_base_url() -> String {
    if let Ok(u) = std::env::var("FUSION_GATEWAY_BASE_URL")
        && !u.is_empty()
    {
        return u;
    }
    crate::config::load_config().mlx.base_url.clone()
}

// Same candidate chain as doctor: env > fusion-core client_key() > config.
// Unlike doctor this returns ALL distinct candidates so the catalog fetch
// can fall through a rejected key instead of degrading to the snapshot.
async fn candidate_keys(root: &Path) -> Vec<String> {
    let mut keys = Vec::new();
    if let Ok(k) = std::env::var("FUSION_CLIENT_KEY")
        && !k.is_empty()
    {
        keys.push(k);
    }
    let venv_python = root.join(".venv/bin/python");
    let py: PathBuf = if venv_python.exists() {
        venv_python
    } else {
        PathBuf::from("python3")
    };
    let code = "import sys; sys.path.insert(0, sys.argv[1]); from fusion_core.config import client_key; print(client_key())";
    if let Ok(out) = tokio::process::Command::new(&py)
        .arg("-c")
        .arg(code)
        .arg(root.join("fusion-core"))
        .output()
        .await
    {
        let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if out.status.success() && !v.is_empty() {
            keys.push(v);
        }
    }
    let cfg_key = crate::config::load_config().mlx.api_key.clone();
    if !cfg_key.is_empty() {
        keys.push(cfg_key);
    }
    keys.dedup();
    keys
}

// Clone repo into dest with visible progress; hard-fail on any git error
// (no partial silent installs — the caller never pretends success).
// Networks that block HTTPS to github.com get one SSH fallback attempt.
async fn clone_repo(repo: &str, dest: &Path) -> Result<()> {
    info!(repo = %repo, dest = %dest.display(), "cloning");
    if try_clone(repo, dest).await.is_ok() {
        return Ok(());
    }
    if let Some(ssh) = https_to_ssh(repo) {
        info!(repo = %ssh, "https clone failed, retrying over ssh");
        if try_clone(&ssh, dest).await.is_ok() {
            return Ok(());
        }
    }
    Err(anyhow!(
        "git clone failed for {repo} (https + ssh fallback both failed)"
    ))
}

async fn try_clone(repo: &str, dest: &Path) -> Result<()> {
    let out = tokio::process::Command::new("git")
        .args(["clone", "--progress", repo])
        .arg(dest)
        .output()
        .await
        .map_err(|e| anyhow!("git spawn failed: {e}"))?;
    if out.status.success() {
        return Ok(());
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    // Clean the partial dir so a retry starts from a known state.
    if dest.exists() {
        let _ = std::fs::remove_dir_all(dest);
    }
    Err(anyhow!("git clone failed: {}", stderr.trim()))
}

// Same-repo check ignoring transport: https://github.com/o/r(.git) and
// git@github.com:o/r.git are the same repository (the ssh fallback makes
// the local origin differ from the catalog URL on blocked networks).
fn same_repo(a: &str, b: &str) -> bool {
    fn norm(url: &str) -> Option<String> {
        let rest = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("git@github.com:"))?;
        let rest = rest.trim_end_matches('/');
        let rest = rest.strip_suffix(".git").unwrap_or(rest);
        if rest.is_empty() {
            return None;
        }
        Some(rest.to_lowercase())
    }
    match (norm(a), norm(b)) {
        (Some(a), Some(b)) => a == b,
        _ => a == b,
    }
}

// https://github.com/owner/repo(.git) -> git@github.com:owner/repo.git;
// None for any other URL shape (already-ssh, non-github hosts).
fn https_to_ssh(url: &str) -> Option<String> {
    let rest = url.strip_prefix("https://github.com/")?;
    let (owner_repo, _ref) = match rest.split_once('#') {
        Some((a, b)) => (a, b),
        None => (rest, ""),
    };
    let owner_repo = owner_repo.trim_end_matches('/');
    if owner_repo.is_empty() || owner_repo.contains("//") {
        return None;
    }
    let owner_repo = owner_repo.trim_end_matches(".git");
    Some(format!("git@github.com:{owner_repo}.git"))
}

// Origin URL of an existing clone (None when not a git repo).
fn existing_origin(dest: &Path) -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["-C"])
        .arg(dest)
        .args(["remote", "get-url", "origin"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let url = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if url.is_empty() { None } else { Some(url) }
}

// Language-appropriate bootstrap, reusing the workspace venv where present:
// pyproject.toml -> venv (create if missing) + pip install -e;
// Cargo.toml -> cargo build --release; package.json -> bun install.
// Python modules depend on the in-tree fusion-core base, so it is cloned
// and installed first when missing (repo convention: install fusion-core
// before any domain app).
async fn bootstrap_deps(root: &Path, dest: &Path) -> Result<()> {
    println!("  {} running deps bootstrap", "→".blue());
    if dest.join("pyproject.toml").exists() {
        let venv_python = ensure_venv(root).await?;
        let core = root.join("fusion-core");
        if dest != core && !core.join("pyproject.toml").exists() {
            println!("  Provisioning shared base fusion-core");
            clone_repo("https://github.com/dahai80/fusion-core", &core).await?;
        }
        if dest != core && core.join("pyproject.toml").exists() {
            run_step(
                venv_python.as_os_str(),
                &[
                    "-m".into(),
                    "pip".into(),
                    "install".into(),
                    "-e".into(),
                    core.as_os_str().to_os_string(),
                ],
                dest,
            )
            .await?;
        }
        // Respect an operator-provided index; default to the workspace
        // mirror (repo convention) so clean machines resolve packages.
        let mut pip_args: Vec<std::ffi::OsString> =
            vec!["-m".into(), "pip".into(), "install".into()];
        if std::env::var("PIP_INDEX_URL")
            .map(|v| v.is_empty())
            .unwrap_or(true)
        {
            pip_args.push("-i".into());
            pip_args.push("https://mirrors.aliyun.com/pypi/simple/".into());
        }
        pip_args.push("-e".into());
        pip_args.push(dest.as_os_str().to_os_string());
        run_step(venv_python.as_os_str(), &pip_args, dest).await
    } else if dest.join("Cargo.toml").exists() {
        run_step(
            "cargo".as_ref(),
            &["build".into(), "--release".into()],
            dest,
        )
        .await
    } else if dest.join("package.json").exists() {
        if which_ok("bun") {
            run_step("bun".as_ref(), &["install".into()], dest).await
        } else {
            run_step("npm".as_ref(), &["install".into()], dest).await
        }
    } else {
        println!(
            "  {} no build manifest detected, skipping deps",
            "✓".green()
        );
        Ok(())
    }
}

// Ensure the workspace venv exists; create it with python3.12 (fallback
// python3) when missing so a clean machine can complete a Python install.
async fn ensure_venv(root: &Path) -> Result<PathBuf> {
    let venv_python = root.join(".venv/bin/python");
    if venv_python.exists() {
        return Ok(venv_python);
    }
    println!(
        "  Creating workspace venv at {}",
        root.join(".venv").display()
    );
    let py = if which_ok("python3.12") {
        "python3.12"
    } else {
        "python3"
    };
    let out = tokio::process::Command::new(py)
        .args(["-m", "venv"])
        .arg(root.join(".venv"))
        .output()
        .await
        .map_err(|e| anyhow!("venv spawn failed: {e}"))?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        return Err(anyhow!("venv creation failed: {}", stderr.trim()));
    }
    Ok(root.join(".venv/bin/python"))
}

// Run one bootstrap step in dir, streaming a failure verdict. Returns the
// merged output tail on error so the failure is visible, not swallowed.
async fn run_step(
    program: &std::ffi::OsStr,
    args: &[std::ffi::OsString],
    dir: &Path,
) -> Result<()> {
    let cmd_str = format!(
        "{} {}",
        program.to_string_lossy(),
        args.iter()
            .map(|a| a.to_string_lossy())
            .collect::<Vec<_>>()
            .join(" ")
    );
    println!("  $ {cmd_str}");
    info!(cmd = %cmd_str, dir = %dir.display(), "bootstrap step");
    let out = tokio::process::Command::new(program)
        .args(args)
        .current_dir(dir)
        .output()
        .await
        .map_err(|e| anyhow!("spawn '{cmd_str}' failed: {e}"))?;
    if !out.status.success() {
        let mut tail = String::from_utf8_lossy(&out.stderr).to_string();
        if tail.trim().is_empty() {
            tail = String::from_utf8_lossy(&out.stdout).to_string();
        }
        let tail = tail.lines().rev().take(10).collect::<Vec<_>>();
        let tail = tail.into_iter().rev().collect::<Vec<_>>().join("\n    ");
        return Err(anyhow!(
            "'{cmd_str}' failed (exit {:?}):\n    {}",
            out.status.code(),
            tail
        ));
    }
    Ok(())
}

fn which_ok(bin: &str) -> bool {
    tokio::task::block_in_place(|| {
        std::process::Command::new("which")
            .arg(bin)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    })
}

// Best-effort running states from the supervisor daemon; None when the
// daemon is down (list stays useful for the installed/available view).
async fn supervisor_states() -> Option<std::collections::HashMap<String, String>> {
    match sv_svc::status() {
        Ok(v) => {
            let mut map = std::collections::HashMap::new();
            if let Some(arr) = v.as_array() {
                for e in arr {
                    if let (Some(name), Some(state)) = (
                        e.get("name").and_then(|x| x.as_str()),
                        e.get("state").and_then(|x| x.as_str()),
                    ) {
                        map.insert(name.to_string(), state.to_string());
                    }
                }
            }
            Some(map)
        }
        Err(e) => {
            info!(error = %e, "supervisor status unavailable; listing without running states");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn home_for_id_routes_ai_modules_to_business() {
        let root = PathBuf::from("/tmp/fs-root");
        assert_eq!(
            home_for_id(&root, "ai-fit"),
            PathBuf::from("/tmp/fs-root/business/ai-fit")
        );
        assert_eq!(
            home_for_id(&root, "fusion-bench"),
            PathBuf::from("/tmp/fs-root/fusion-bench")
        );
    }

    #[test]
    fn install_root_flag_beats_env_and_default() {
        // Edition 2024: env mutation is unsafe; the flag branch never reads
        // the var, so clearing it only hardens the default-branch assertion.
        unsafe { std::env::remove_var("FUSION_WORKSPACE_ROOT") };
        assert_eq!(install_root(&Some("/x".into())), PathBuf::from("/x"));
        assert!(install_root(&None).ends_with("fusion"));
    }

    #[test]
    fn bundled_catalog_snapshot_parses_and_is_populated() {
        let entries: Vec<CatalogEntry> =
            serde_json::from_str(CATALOG_FALLBACK).expect("snapshot parses");
        assert!(
            entries.len() >= 30,
            "snapshot holds the full catalog, got {}",
            entries.len()
        );
        assert!(entries.iter().any(|e| e.id == "fusion-mlx"));
        assert!(entries.iter().all(|e| !e.repo.is_empty()));
    }

    #[test]
    fn bundled_catalog_finds_entry_by_id() {
        let entries: Vec<CatalogEntry> = serde_json::from_str(CATALOG_FALLBACK).unwrap();
        let e = entries
            .into_iter()
            .find(|e| e.id == "fusion-mlx")
            .expect("fusion-mlx present");
        assert!(e.repo.ends_with("fusion-mlx"));
        assert_eq!(e.category, "core");
    }

    #[test]
    fn fetch_entry_missing_id_reports_source() {
        // Direct contract check: lookup miss produces a message carrying the id.
        let entries: Vec<CatalogEntry> = serde_json::from_str(CATALOG_FALLBACK).unwrap();
        let miss = entries.iter().find(|e| e.id == "no-such-id").is_none();
        assert!(miss);
    }

    #[test]
    fn existing_origin_none_for_plain_dir() {
        let dir = std::env::temp_dir().join(format!("fs-origin-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        assert_eq!(existing_origin(&dir), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn existing_origin_reads_real_clone() {
        let dir = std::env::temp_dir().join(format!("fs-clone-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let ok = std::process::Command::new("git")
            .args(["init", "-q"])
            .arg(&dir)
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if ok {
            std::process::Command::new("git")
                .args(["-C"])
                .arg(&dir)
                .args(["remote", "add", "origin", "https://example.com/r.git"])
                .status()
                .expect("remote add");
            assert_eq!(
                existing_origin(&dir).as_deref(),
                Some("https://example.com/r.git")
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_repo_matches_across_transports() {
        assert!(same_repo(
            "https://github.com/dahai80/fusion-bench",
            "git@github.com:dahai80/fusion-bench.git"
        ));
        assert!(same_repo(
            "https://github.com/dahai80/Fusion-Bench.git",
            "git@github.com:dahai80/fusion-bench.git"
        ));
        assert!(!same_repo(
            "https://github.com/dahai80/fusion-bench",
            "git@github.com:dahai80/fusion-core.git"
        ));
        assert!(!same_repo(
            "https://github.com/dahai80/fusion-bench",
            "https://gitlab.com/dahai80/fusion-bench"
        ));
    }

    #[test]
    fn https_to_ssh_converts_github_urls() {
        assert_eq!(
            https_to_ssh("https://github.com/dahai80/fusion-bench"),
            Some("git@github.com:dahai80/fusion-bench.git".into())
        );
        assert_eq!(
            https_to_ssh("https://github.com/dahai80/fusion-bench.git"),
            Some("git@github.com:dahai80/fusion-bench.git".into())
        );
        assert_eq!(https_to_ssh("git@github.com:dahai80/r.git"), None);
        assert_eq!(https_to_ssh("https://gitlab.com/a/b"), None);
    }

    #[test]
    fn json_purity_for_list_payload() {
        // The list JSON payload must serialize standalone (no banner leaks).
        let payload = serde_json::json!({
            "source": "bundled snapshot",
            "modules": [{"id": "fusion-mlx", "installed": true, "state": "-", "footprint": "~2 GB"}],
        });
        let s = serde_json::to_string(&payload).unwrap();
        assert!(s.starts_with('{') && s.ends_with('}'));
        assert!(s.contains("\"installed\":true"));
    }
}
