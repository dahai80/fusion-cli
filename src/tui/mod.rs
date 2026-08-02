pub mod app;
pub mod service_fetcher;
pub mod ui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    terminal::{self, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::Terminal;
use ratatui::prelude::CrosstermBackend;
use std::io;
use std::time::Duration;
use tracing::info;

use app::App;
use service_fetcher::fetch_all;

const TICK_RATE_MS: u64 = 200;
const REFRESH_INTERVAL_TICKS: u64 = 10;

pub async fn run_dashboard() -> Result<()> {
    info!("Starting TUI dashboard");
    terminal::enable_raw_mode()?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new();
    let data = fetch_all().await;
    app.update_data(data);

    let result = run_app(&mut terminal, &mut app).await;

    terminal::disable_raw_mode()?;
    crossterm::execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> Result<()> {
    let mut tick_count: u64 = 0;

    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        let timeout = Duration::from_millis(TICK_RATE_MS);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Esc => {
                            app.quit();
                        }
                        KeyCode::Char('1') => {
                            app.tab = app::Tab::Services;
                            app.selected = 0;
                        }
                        KeyCode::Char('2') => {
                            app.tab = app::Tab::Models;
                            app.selected = 0;
                        }
                        KeyCode::Char('3') => {
                            app.tab = app::Tab::System;
                            app.selected = 0;
                        }
                        KeyCode::Char('4') => {
                            app.tab = app::Tab::Logs;
                            app.selected = 0;
                        }
                        KeyCode::Tab | KeyCode::Right => {
                            app.next_tab();
                        }
                        KeyCode::BackTab | KeyCode::Left => {
                            app.prev_tab();
                        }
                        KeyCode::Down | KeyCode::Char('j') => {
                            app.down();
                        }
                        KeyCode::Up | KeyCode::Char('k') => {
                            app.up();
                        }
                        KeyCode::Char('r') => {
                            let data = fetch_all().await;
                            app.update_data(data);
                            tick_count = 0;
                        }
                        KeyCode::Char('s') => {
                            if let Some(svc) = app.selected_service() {
                                let name = svc.name.to_lowercase();
                                info!(service = %name, "Starting service from dashboard");
                                start_service_from_dashboard(&name).await;
                                let data = fetch_all().await;
                                app.update_data(data);
                            }
                        }
                        KeyCode::Char('x') => {
                            if let Some(svc) = app.selected_service() {
                                let name = svc.name.to_lowercase();
                                info!(service = %name, "Stopping service from dashboard");
                                stop_service_from_dashboard(&name).await;
                                let data = fetch_all().await;
                                app.update_data(data);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        if !app.running {
            return Ok(());
        }

        tick_count += 1;
        if tick_count >= REFRESH_INTERVAL_TICKS {
            let data = fetch_all().await;
            app.update_data(data);
            tick_count = 0;
        }
    }
}

async fn start_service_from_dashboard(name: &str) {
    if name == "mlx" {
        let script = dirs::home_dir()
            .unwrap_or_default()
            .join("claude-home/fusion-mlx/start.sh");
        if script.exists() {
            let _ = tokio::process::Command::new(&script)
                .arg("start")
                .output()
                .await;
        }
    }
}

async fn stop_service_from_dashboard(name: &str) {
    if name == "mlx" {
        let script = dirs::home_dir()
            .unwrap_or_default()
            .join("claude-home/fusion-mlx/start.sh");
        if script.exists() {
            let _ = tokio::process::Command::new(&script)
                .arg("stop")
                .output()
                .await;
        }
    }
}
