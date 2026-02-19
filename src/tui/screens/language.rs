use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, ListState},
};

pub struct RuntimePreset {
    pub name: &'static str,
    pub versions: &'static [(&'static str, &'static str)],
}

pub const PRESETS: &[RuntimePreset] = &[
    RuntimePreset {
        name: "Python",
        versions: &[
            ("3.12", "python:3.12-alpine"),
            ("3.11", "python:3.11-alpine"),
            ("3.10", "python:3.10-alpine"),
        ],
    },
    RuntimePreset {
        name: "Node.js",
        versions: &[
            ("22", "node:22-alpine"),
            ("20", "node:20-alpine"),
            ("18", "node:18-alpine"),
        ],
    },
    RuntimePreset {
        name: "Go",
        versions: &[
            ("1.23", "golang:1.23-alpine"),
            ("1.22", "golang:1.22-alpine"),
            ("1.21", "golang:1.21-alpine"),
        ],
    },
    RuntimePreset {
        name: "Rust",
        versions: &[("1.85", "rust:1.85-alpine"), ("1.84", "rust:1.84-alpine")],
    },
    RuntimePreset {
        name: "Java",
        versions: &[
            ("21", "eclipse-temurin:21-alpine"),
            ("17", "eclipse-temurin:17-alpine"),
            ("11", "eclipse-temurin:11-alpine"),
        ],
    },
    RuntimePreset {
        name: "Custom",
        versions: &[],
    },
];

pub struct LanguageScreen {
    pub selected: usize,
    pub version_selected: usize,
    pub presets: &'static [RuntimePreset],
}

impl LanguageScreen {
    pub fn new() -> Self {
        Self {
            selected: 0,
            version_selected: 0,
            presets: PRESETS,
        }
    }

    pub fn next(&mut self) {
        self.selected = (self.selected + 1) % self.presets.len();
        self.version_selected = 0;
    }

    pub fn prev(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else {
            self.selected = self.presets.len() - 1;
        }
        self.version_selected = 0;
    }

    pub fn next_version(&mut self) {
        let versions = &self.presets[self.selected].versions;
        if !versions.is_empty() {
            self.version_selected = (self.version_selected + 1) % versions.len();
        }
    }

    pub fn prev_version(&mut self) {
        let versions = &self.presets[self.selected].versions;
        if !versions.is_empty() {
            if self.version_selected > 0 {
                self.version_selected -= 1;
            } else {
                self.version_selected = versions.len() - 1;
            }
        }
    }
}

pub fn draw(frame: &mut Frame, area: Rect, screen: &LanguageScreen) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let items: Vec<ListItem> = screen
        .presets
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let style = if i == screen.selected {
                Style::default().fg(Color::Yellow).bold()
            } else {
                Style::default()
            };
            ListItem::new(format!("  {}", p.name)).style(style)
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(screen.selected));

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Language/Runtime "),
        )
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(list, chunks[0], &mut state);

    let preset = &screen.presets[screen.selected];
    let version_items: Vec<ListItem> = if preset.versions.is_empty() {
        vec![ListItem::new("  (enter custom image in next step)")
            .style(Style::default().fg(Color::DarkGray))]
    } else {
        preset
            .versions
            .iter()
            .enumerate()
            .map(|(i, (ver, img))| {
                let style = if i == screen.version_selected {
                    Style::default().fg(Color::Cyan).bold()
                } else {
                    Style::default()
                };
                ListItem::new(format!("  {} → {}", ver, img)).style(style)
            })
            .collect()
    };

    let mut ver_state = ListState::default();
    ver_state.select(Some(screen.version_selected));

    let version_list = List::new(version_items)
        .block(Block::default().borders(Borders::ALL).title(" Version "))
        .highlight_style(Style::default().bg(Color::DarkGray))
        .highlight_symbol("▶ ");

    frame.render_stateful_widget(version_list, chunks[1], &mut ver_state);
}
