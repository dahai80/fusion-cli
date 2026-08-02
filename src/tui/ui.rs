use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
};

use super::app::{App, Tab};

pub fn draw(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(f.area());

    draw_header(f, app, chunks[0]);
    draw_content(f, app, chunks[1]);
    draw_status_bar(f, app, chunks[2]);
    draw_help_bar(f, chunks[3]);
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            if *t == app.tab {
                Line::from(Span::styled(
                    format!(" {} ", t.title()),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    format!(" {} ", t.title()),
                    Style::default().fg(Color::DarkGray),
                ))
            }
        })
        .collect();

    let tab_idx = app.tab.index();
    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" ⚡ Fusion-CLI v{} ", env!("CARGO_PKG_VERSION"))),
        )
        .select(tab_idx)
        .divider(Span::raw("│"));

    f.render_widget(tabs, area);
}

fn draw_content(f: &mut Frame, app: &App, area: Rect) {
    match app.tab {
        Tab::Services => draw_services(f, app, area),
        Tab::Models => draw_models(f, app, area),
        Tab::System => draw_system(f, app, area),
        Tab::Logs => draw_logs(f, app, area),
    }
}

fn draw_services(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .services
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let status_icon = if s.alive { "✅ UP  " } else { "⬜ DOWN" };
            let latency = match s.latency_ms {
                Some(ms) => format!("{:>4}ms", ms),
                None => "    -".to_string(),
            };
            let port = if s.port > 0 {
                format!(":{:<6}", s.port)
            } else {
                "      ".to_string()
            };
            let line = Line::from(vec![
                Span::styled(
                    format!(" {} ", status_icon),
                    if s.alive {
                        Style::default().fg(Color::Green)
                    } else {
                        Style::default().fg(Color::Red)
                    },
                ),
                Span::styled(
                    format!("{:<10}", s.name),
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::raw(port),
                Span::styled(latency, Style::default().fg(Color::Cyan)),
            ]);
            let style = if i == app.selected {
                Style::default().bg(Color::DarkGray)
            } else {
                Style::default()
            };
            ListItem::new(line).style(style)
        })
        .collect();

    let list = List::new(items).block(Block::default().borders(Borders::ALL).title(" Services "));
    f.render_widget(list, area);
}

fn draw_models(f: &mut Frame, app: &App, area: Rect) {
    if app.models.is_empty() {
        let para = Paragraph::new("No models loaded (MLX service may be down)")
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::ALL).title(" Models "))
            .wrap(Wrap { trim: true });
        f.render_widget(para, area);
        return;
    }

    let items: Vec<ListItem> = app
        .models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let style = if i == app.selected {
                Style::default()
                    .bg(Color::DarkGray)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(" 📦 ", Style::default()),
                Span::styled(m.clone(), style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(format!(" Models ({}) ", app.models.len())),
    );
    f.render_widget(list, area);
}

fn draw_system(f: &mut Frame, app: &App, area: Rect) {
    let sys = &app.system;
    let mem_total_gb = sys.mem_total as f64 / 1_073_741_824.0;
    let mem_used_gb = sys.mem_used as f64 / 1_073_741_824.0;
    let mem_pct = if sys.mem_total > 0 {
        sys.mem_used as f64 / sys.mem_total as f64 * 100.0
    } else {
        0.0
    };

    let cpu_bar = {
        let filled = (sys.cpu_usage / 100.0 * 20.0) as usize;
        let empty = 20 - filled;
        format!("{}{}", "█".repeat(filled), "░".repeat(empty),)
    };

    let mem_bar = {
        let filled = (mem_pct / 100.0 * 20.0) as usize;
        let empty = 20 - filled;
        format!("{}{}", "█".repeat(filled), "░".repeat(empty),)
    };

    let temp_str = match sys.cpu_temp {
        Some(t) => format!("{:.1}°C", t),
        None => "N/A".to_string(),
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(" CPU   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!("{:>5.1}% ", sys.cpu_usage),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(cpu_bar),
        ]),
        Line::from(vec![
            Span::styled(" Memory", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(
                format!(" {:.1}/{:.1}GB ", mem_used_gb, mem_total_gb),
                Style::default().fg(Color::Cyan),
            ),
            Span::raw(mem_bar),
        ]),
        Line::from(vec![
            Span::styled(" Temp  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(" {}", temp_str)),
        ]),
        Line::from(vec![
            Span::styled(" Arch  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!(
                " {} / {}",
                std::env::consts::OS,
                std::env::consts::ARCH
            )),
        ]),
    ];

    let para = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title(" System "))
        .wrap(Wrap { trim: true });
    f.render_widget(para, area);
}

fn draw_logs(f: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .logs
        .iter()
        .map(|l| ListItem::new(Line::from(Span::raw(l.clone()))))
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Recent Logs "),
    );
    f.render_widget(list, area);
}

fn draw_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let up = app.services_up();
    let total = app.services.len();
    let mem_gb = app.system.mem_used as f64 / 1_073_741_824.0;
    let mem_total_gb = app.system.mem_total as f64 / 1_073_741_824.0;

    let line = Line::from(vec![
        Span::styled(
            format!(" CPU: {:.0}%", app.system.cpu_usage),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("Mem: {:.1}/{:.1}GB", mem_gb, mem_total_gb),
            Style::default().fg(Color::Cyan),
        ),
        Span::raw(" │ "),
        Span::styled(
            format!("{}:{}", app.services_up(), total),
            if up == total && total > 0 {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            },
        ),
        Span::raw(" UP "),
        Span::raw("│ "),
        Span::styled(
            format!("Refresh: {}", app.last_refresh),
            Style::default().fg(Color::DarkGray),
        ),
    ]);

    let para = Paragraph::new(line).style(Style::default().bg(Color::Black));
    f.render_widget(para, area);
}

fn draw_help_bar(f: &mut Frame, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" q", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":Quit "),
        Span::styled("1-4", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":Tab "),
        Span::styled(" ↑↓", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":Nav "),
        Span::styled(" r", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":Refresh "),
        Span::styled(" s", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":Start "),
        Span::styled(" x", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(":Stop "),
    ]);
    let para = Paragraph::new(line).style(Style::default().bg(Color::DarkGray));
    f.render_widget(para, area);
}
