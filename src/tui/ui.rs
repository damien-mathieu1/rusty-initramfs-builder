use crate::tui::app::{App, Screen};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(3),
        ])
        .split(area);

    draw_header(frame, chunks[0], app);
    draw_content(frame, chunks[1], app);
    draw_footer(frame, chunks[2], app);
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let step = match app.screen {
        Screen::Language => "1/7",
        Screen::Image => "2/7",
        Screen::Architecture => "3/7",
        Screen::Inject => "4/7",
        Screen::Init => "5/7",
        Screen::Compression => "6/7",
        Screen::Summary => "7/7",
        Screen::Building => "Building...",
    };

    let title = format!(" initramfs-builder interactive [{}] ", step);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).bold());

    frame.render_widget(block, area);
}

fn draw_content(frame: &mut Frame, area: Rect, app: &App) {
    match app.screen {
        Screen::Language => crate::tui::screens::language::draw(frame, area, &app.language_screen),
        Screen::Image => crate::tui::screens::image::draw(frame, area, &app.image_screen),
        Screen::Architecture => crate::tui::screens::arch::draw(frame, area, &app.arch_screen),
        Screen::Inject => crate::tui::screens::inject::draw(frame, area, &app.inject_screen),
        Screen::Init => crate::tui::screens::init::draw(frame, area, &app.init_screen),
        Screen::Compression => {
            crate::tui::screens::compress::draw(frame, area, &app.compress_screen)
        }
        Screen::Summary => crate::tui::screens::summary::draw(frame, area, app),
        Screen::Building => draw_building(frame, area, app),
    }
}

fn draw_building(frame: &mut Frame, area: Rect, app: &App) {
    let text = if let Some(err) = &app.build_error {
        Paragraph::new(err.as_str()).style(Style::default().fg(Color::Red))
    } else if let Some(progress) = &app.build_progress {
        Paragraph::new(progress.as_str()).style(Style::default().fg(Color::Green))
    } else {
        Paragraph::new("Building...").style(Style::default().fg(Color::Yellow))
    };

    let block = Block::default().borders(Borders::ALL).title(" Build ");
    frame.render_widget(text.block(block), area);
}

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let hints = match app.screen {
        Screen::Language => "↑↓ Select  ←→ Version  Enter Next  q Quit",
        Screen::Image => "Type to edit  Enter Next  Esc Back",
        Screen::Architecture => "↑↓ Select  Enter Next  Esc Back",
        Screen::Inject => "a Add  d Delete  ↑↓ Select  Enter Next  Esc Back",
        Screen::Init => "↑↓ Select  Enter Next  Esc Back",
        Screen::Compression => "↑↓ Select  Enter Next  Esc Back",
        Screen::Summary => "Enter Build  Esc Back  q Quit",
        Screen::Building => "Please wait...",
    };

    let paragraph = Paragraph::new(hints)
        .style(Style::default().fg(Color::DarkGray))
        .alignment(Alignment::Center);

    let block = Block::default().borders(Borders::ALL);
    frame.render_widget(paragraph.block(block), area);
}
