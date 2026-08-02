use anyhow::Result;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use colored::*;

pub fn run_completions(shell: &str) -> Result<()> {
    let shell = match shell.to_lowercase().as_str() {
        "bash" => Shell::Bash,
        "zsh" => Shell::Zsh,
        "fish" => Shell::Fish,
        "elvish" => Shell::Elvish,
        "powershell" => Shell::PowerShell,
        _ => {
            println!("{} Unsupported shell: {}", "❌".red(), shell);
            println!("  Supported: bash, zsh, fish, elvish, powershell");
            anyhow::bail!("Unsupported shell: {}", shell);
        }
    };

    println!(
        "{} Generating shell completions for {}...",
        "📝".bold(),
        shell.to_string().cyan()
    );

    let mut cmd = crate::Cli::command();
    let bin_name = "fusion".to_string();
    generate(shell, &mut cmd, bin_name, &mut std::io::stdout());

    Ok(())
}
