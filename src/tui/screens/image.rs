use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub struct ImageScreen {
    pub input: String,
    pub error: Option<String>,
}

impl ImageScreen {
    pub fn new() -> Self {
        Self {
            input: String::new(),
            error: None,
        }
    }

    pub fn sync_from_config(&mut self, image: &str) {
        if self.input.is_empty() {
            self.input = image.to_string();
        }
        self.error = None;
    }

    pub fn sync_to_config(&self) -> String {
        self.input.trim().to_string()
    }

    pub fn set_error(&mut self, msg: &str) {
        self.error = Some(msg.to_string());
    }
}

pub fn draw(frame: &mut Frame, area: Rect, screen: &ImageScreen) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Min(0),
        ])
        .split(area);

    let label =
        Paragraph::new(" OCI Image Reference (e.g., python:3.12-alpine, ghcr.io/org/image:tag)")
            .style(Style::default().fg(Color::Gray));
    frame.render_widget(label, chunks[0]);

    let border_style = if screen.error.is_some() {
        Style::default().fg(Color::Red)
    } else {
        Style::default()
    };

    let input_display = format!("{}_", screen.input);
    let input = Paragraph::new(input_display)
        .style(Style::default().fg(Color::Yellow))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Image ")
                .border_style(border_style),
        );
    frame.render_widget(input, chunks[1]);

    if let Some(err) = &screen.error {
        let error_msg =
            Paragraph::new(format!(" ⚠ {}", err)).style(Style::default().fg(Color::Red));
        frame.render_widget(error_msg, chunks[2]);
    }

    let help = Paragraph::new(" Type to edit. Press Enter to continue, Esc to go back.")
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[3]);
}
