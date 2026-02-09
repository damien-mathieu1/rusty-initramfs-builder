use crate::tui::app::{App, InitMode, WizardMode};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(0),
            Constraint::Length(6),
            Constraint::Length(3),
        ])
        .split(area);

    let init_desc = match &app.config.init_mode {
        InitMode::Default => "Default (auto-generated)".to_string(),
        InitMode::CustomFile(path) => format!("Custom: {}", path.display()),
    };

    let injections_desc = if app.config.injections.is_empty() {
        "  (none)".to_string()
    } else {
        app.config
            .injections
            .iter()
            .map(|i| format!("  {} → {}", i.src, i.dest))
            .collect::<Vec<_>>()
            .join("\n")
    };

    let mode_indicator = match app.mode {
        WizardMode::Quick => " [Quick mode - press 'a' for Advanced]",
        WizardMode::Advanced => " [Advanced mode]",
    };

    let validation_line = if let Some(err) = &app.validation_error {
        format!("\n  ⚠ {}", err)
    } else {
        String::new()
    };

    let summary = format!(
        r#"
  Image:        {}
  Architecture: {}
  Compression:  {}
  Output:       {}
  Init:         {}

  Injections:
{}{}
"#,
        if app.config.image.is_empty() {
            "⚠ MISSING"
        } else {
            &app.config.image
        },
        app.config.arch,
        app.config.compression,
        app.config.output,
        init_desc,
        injections_desc,
        validation_line,
    );

    let border_style = if app.validation_error.is_some() || app.config.image.is_empty() {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Green)
    };

    let summary_widget = Paragraph::new(summary)
        .style(Style::default().fg(Color::White))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(format!(" Summary{} ", mode_indicator))
                .border_style(border_style),
        );
    frame.render_widget(summary_widget, chunks[0]);

    let cli_cmd = app.generate_cli_command();
    let cli_widget = Paragraph::new(cli_cmd)
        .style(Style::default().fg(Color::Cyan))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Equivalent CLI "),
        );
    frame.render_widget(cli_widget, chunks[1]);

    let can_build = app.is_config_valid();
    let hint = if can_build {
        " Enter → Build | Esc → Back | 'a' → Advanced options"
    } else {
        " ⚠ Fix errors before building | Esc → Back | 'a' → Advanced options"
    };
    let hint_style = if can_build {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::Red)
    };
    let hint_widget = Paragraph::new(hint).style(hint_style);
    frame.render_widget(hint_widget, chunks[2]);
}
