use crate::tui::app::App;
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

    let validation_line = if let Some(err) = &app.validation_error {
        format!("\n  ⚠ {}", err)
    } else {
        String::new()
    };

    let summary = format!(
        r#"
  Image:        {}
  Architecture: {} (default)
  Compression:  {} (default)
  Output:       {} (default)

{}
"#,
        if app.config.image.is_empty() {
            "⚠ MISSING"
        } else {
            &app.config.image
        },
        app.config.arch,
        app.config.compression,
        app.config.output,
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
                .title(" Summary ")
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
}
