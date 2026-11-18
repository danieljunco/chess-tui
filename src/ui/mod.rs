//! Ratatui rendering. Reads `App` state; never mutates it.

pub mod board;
mod game;
mod overlay;
mod screens;
pub mod theme;

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};

use crate::app::{App, Link, Screen, Table, Tone};
use theme::Theme;

pub const MIN_WIDTH: u16 = 80;
pub const MIN_HEIGHT: u16 = 24;
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
const SPINNER: [&str; 8] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧"];

pub fn draw(frame: &mut Frame, app: &App, theme: &Theme) {
    let area = frame.area();
    frame.render_widget(Block::new().style(theme.base()), area);

    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        screens::too_small(frame, area, theme);
        return;
    }

    let [header, body, status, footer] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .areas(area);

    draw_header(frame, header, app, theme);
    match &app.screen {
        Screen::Home { selected } => screens::home(frame, body, *selected, theme),
        Screen::Join(form) => screens::join(frame, body, form, theme),
        Screen::Waiting(info) => screens::waiting(frame, body, info, spinner(app), theme),
        Screen::Connecting { addr } => screens::connecting(frame, body, addr, spinner(app), theme),
        Screen::Game(g) => game::draw(frame, body, g, app, theme),
    }
    draw_status(frame, status, app, theme);
    draw_footer(frame, footer, &hints(app), theme);

    if let Some(ov) = app.overlay {
        overlay::draw(frame, body, ov, app, theme);
    }
}

pub fn spinner(app: &App) -> &'static str {
    SPINNER[app.frame() % SPINNER.len()]
}

fn draw_header(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    frame.render_widget(Block::new().style(theme.panel()), area);
    let crumb = match &app.screen {
        Screen::Home { .. } => "HOME".to_string(),
        Screen::Join(_) => "JOIN A GAME".to_string(),
        Screen::Waiting(_) => "HOSTING".to_string(),
        Screen::Connecting { addr } => format!("JOINING · {addr}"),
        Screen::Game(g) => match &g.view.table {
            Table::Local => "LOCAL GAME".to_string(),
            Table::Hosting { addr } => format!("HOSTING · {addr}"),
            Table::Joined { addr } => format!("JOINED · {addr}"),
        },
    };
    let mut left = vec![
        Span::styled(" ♞ VESPER", theme.accent()),
        Span::styled("  /  ", theme.faint()),
        Span::styled(crumb, theme.muted().add_modifier(Modifier::BOLD)),
    ];
    if app.demo {
        left.push(Span::styled("   DEMO DATA", theme.faint()));
    }
    frame.render_widget(Paragraph::new(Line::from(left)), area);

    let (text, tone) = match &app.screen {
        Screen::Game(g) => link_badge(g.view.link),
        Screen::Waiting(_) => ("● LISTENING".to_string(), Tone::Accent),
        Screen::Connecting { .. } => ("◌ CONNECTING".to_string(), Tone::Warning),
        _ => (format!("v{VERSION}"), Tone::Muted),
    };
    let style = if tone == Tone::Muted {
        theme.faint()
    } else {
        theme.toned(tone).add_modifier(Modifier::BOLD)
    };
    frame.render_widget(
        Paragraph::new(Line::from(Span::styled(format!("{text} "), style)).right_aligned()),
        area,
    );
}

pub fn link_badge(link: Link) -> (String, Tone) {
    match link {
        Link::Local => ("◆ PASS & PLAY".into(), Tone::Muted),
        Link::Live { rtt_ms: Some(ms) } => (format!("● LIVE · {ms} ms"), Tone::Positive),
        Link::Live { rtt_ms: None } => ("● LIVE".into(), Tone::Positive),
        Link::Reconnecting { attempt } => {
            (format!("○ RECONNECTING · try {attempt}"), Tone::Warning)
        }
        Link::OpponentOffline => ("○ OPPONENT OFFLINE".into(), Tone::Warning),
        Link::HostClosed => ("■ HOST CLOSED".into(), Tone::Danger),
    }
}

fn draw_status(frame: &mut Frame, area: Rect, app: &App, theme: &Theme) {
    match &app.screen {
        Screen::Game(g) => game::status_strip(frame, area, g, app, theme),
        _ => {
            if let Some(n) = &app.notice {
                let line = Line::from(vec![
                    Span::styled(" ● ", theme.toned(n.tone)),
                    Span::styled(n.text.clone(), theme.toned(n.tone)),
                ]);
                frame.render_widget(Paragraph::new(line), area);
            }
        }
    }
}

pub type Hint = (&'static str, &'static str);

fn hints(app: &App) -> Vec<Hint> {
    use crate::app::{Focus, Overlay};
    if let Some(ov) = app.overlay {
        return match ov {
            Overlay::Help => vec![("Esc", "Close")],
            Overlay::Promotion { .. } => vec![
                ("←→", "Choose"),
                ("Q R B N", "Pick"),
                ("↵", "Confirm"),
                ("Esc", "Cancel"),
            ],
            Overlay::Confirm(_) => vec![("↵/y", "Confirm"), ("Esc/n", "Cancel")],
            Overlay::Result => vec![("↵", "Review board"), ("n", "Home"), ("q", "Quit")],
        };
    }
    match &app.screen {
        Screen::Home { .. } => vec![
            ("↑↓", "Navigate"),
            ("↵", "Select"),
            ("?", "Help"),
            ("q", "Quit"),
        ],
        Screen::Join(_) => vec![("Tab", "Next field"), ("↵", "Connect"), ("Esc", "Back")],
        Screen::Waiting(_) => vec![("Esc", "Stop hosting"), ("?", "Help")],
        Screen::Connecting { .. } => vec![("Esc", "Cancel")],
        Screen::Game(g) => {
            if g.command.is_some() {
                return vec![("↵", "Send move"), ("Esc", "Cancel")];
            }
            if g.game().is_over() {
                return vec![
                    ("↑↓←→", "Cursor"),
                    ("Tab", "Focus"),
                    ("o", "Flip"),
                    ("n", "Home"),
                    ("q", "Quit"),
                ];
            }
            let mut h = match g.focus {
                Focus::Board => vec![("↑↓←→", "Cursor"), ("↵", "Select")],
                Focus::History => vec![("↑↓", "Scroll moves")],
                Focus::Actions => vec![("↑↓", "Choose"), ("↵", "Run")],
            };
            if g.selected.is_some() {
                h.push(("Esc", "Clear"));
            }
            h.extend([
                ("/", "Type move"),
                ("Tab", "Focus"),
                ("o", "Flip"),
                ("?", "Help"),
                ("r", "Resign"),
            ]);
            if app.demo {
                h.push(("x", "Simulate drop"));
            }
            h
        }
    }
}

pub fn hint_spans(hints: &[Hint], width: u16, theme: &Theme) -> Line<'static> {
    let mut spans = vec![Span::raw(" ")];
    let mut used = 1usize;
    for (key, label) in hints {
        let len = key.chars().count() + label.chars().count() + 4;
        if used + len > width as usize {
            break;
        }
        spans.push(Span::styled(
            key.to_string(),
            theme.key().add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::styled(format!(" {label}   "), theme.muted()));
        used += len;
    }
    Line::from(spans)
}

fn draw_footer(frame: &mut Frame, area: Rect, hints: &[Hint], theme: &Theme) {
    frame.render_widget(Block::new().style(theme.panel()), area);
    frame.render_widget(Paragraph::new(hint_spans(hints, area.width, theme)), area);
}

pub fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let w = width.min(area.width);
    let h = height.min(area.height);
    Rect::new(
        area.x + (area.width - w) / 2,
        area.y + (area.height - h) / 2,
        w,
        h,
    )
}

pub fn section_title(text: &str, focused: bool, theme: &Theme) -> Line<'static> {
    if focused {
        Line::from(vec![
            Span::styled("▍", Style::new().fg(theme.accent)),
            Span::styled(format!("{text} "), theme.accent()),
        ])
    } else {
        Line::from(Span::styled(format!(" {text} "), theme.label()))
    }
}

#[cfg(test)]
mod tests;
