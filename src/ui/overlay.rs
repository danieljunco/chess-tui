//! Modal overlays: help, promotion, confirmations, result.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph};

use super::board::glyph;
use super::centered;
use super::theme::Theme;
use crate::app::{App, Confirm, Overlay, PROMOTION_ROLES, Screen, Seat, Tone};
use crate::domain::{Outcome, Piece, Role, WinReason, color_name};

fn modal(frame: &mut Frame, area: Rect, title: &str, tone: Tone, theme: &Theme) -> Rect {
    frame.render_widget(Clear, area);
    let border = if theme.color {
        Style::new().fg(theme.tone(tone))
    } else {
        Style::new()
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Line::from(Span::styled(
            format!(" {title} "),
            theme.toned(tone).add_modifier(Modifier::BOLD),
        )))
        .style(Style::new().bg(theme.raised).fg(theme.text));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    inner.inner(ratatui::layout::Margin::new(2, 1))
}

pub fn draw(frame: &mut Frame, body: Rect, overlay: Overlay, app: &App, theme: &Theme) {
    match overlay {
        Overlay::Help => help(frame, body, app, theme),
        Overlay::Promotion { from, to, choice } => {
            let color = match &app.screen {
                Screen::Game(g) => g.game().turn(),
                _ => crate::domain::Color::White,
            };
            let area = centered(body, 60, 10);
            let inner = modal(frame, area, "PROMOTE YOUR PAWN", Tone::Accent, theme);
            let mut lines = vec![
                Line::styled(format!("Choose a piece for {from} → {to}"), theme.muted()),
                Line::raw(""),
            ];
            let mut spans = Vec::new();
            for (i, role) in PROMOTION_ROLES.iter().enumerate() {
                let p = Piece { color, role: *role };
                let name = role_name(*role);
                let key = role.char().to_ascii_uppercase();
                let text = format!(" {key} {} {name} ", glyph(p, app.ascii, theme));
                if i == choice {
                    spans.push(Span::styled(text, theme.pill(Tone::Accent)));
                } else {
                    spans.push(Span::styled(text, theme.text()));
                }
                spans.push(Span::raw(" "));
            }
            lines.push(Line::from(spans));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("↵", theme.key()),
                Span::styled(" Confirm    ", theme.muted()),
                Span::styled("Esc", theme.key()),
                Span::styled(" Cancel", theme.muted()),
            ]));
            frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
        }
        Overlay::Confirm(kind) => {
            let opponent = match &app.screen {
                Screen::Game(g) => match g.view.seat {
                    Seat::Player(me) => g.view.name(!me).to_string(),
                    Seat::Both => color_name(!g.game().turn()).to_string(),
                },
                _ => "Your opponent".into(),
            };
            let (title, body_text, yes, tone) = match kind {
                Confirm::Resign => (
                    "RESIGN THIS GAME?",
                    format!("{opponent} will be declared the winner."),
                    "Resign",
                    Tone::Danger,
                ),
                Confirm::ClaimDraw(reason) => (
                    "CLAIM A DRAW?",
                    format!("{} · the game ends as a draw.", reason.label()),
                    "Claim draw",
                    Tone::Accent,
                ),
                Confirm::Leave => (
                    "LEAVE THE TABLE?",
                    "The game stays open only while the host runs.".to_string(),
                    "Leave",
                    Tone::Warning,
                ),
            };
            let area = centered(body, 52, 8);
            let inner = modal(frame, area, title, tone, theme);
            let lines = vec![
                Line::styled(body_text, theme.text()),
                Line::raw(""),
                Line::from(vec![
                    Span::styled(format!(" y {yes} "), theme.pill(tone)),
                    Span::raw("    "),
                    Span::styled("Esc", theme.key()),
                    Span::styled(" Keep playing", theme.muted()),
                ]),
            ];
            frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
        }
        Overlay::Result => result(frame, body, app, theme),
    }
}

fn role_name(role: Role) -> &'static str {
    match role {
        Role::Queen => "Queen",
        Role::Rook => "Rook",
        Role::Bishop => "Bishop",
        Role::Knight => "Knight",
        Role::King => "King",
        Role::Pawn => "Pawn",
    }
}

fn result(frame: &mut Frame, body: Rect, app: &App, theme: &Theme) {
    let Screen::Game(g) = &app.screen else {
        return;
    };
    let Some(outcome) = g.game().outcome() else {
        return;
    };
    let (title, tone, headline, personal) = match outcome {
        Outcome::Win { winner, reason } => {
            let title = match reason {
                WinReason::Checkmate => "CHECKMATE",
                WinReason::Resignation => "RESIGNATION",
            };
            let (tone, personal) = match g.view.seat {
                Seat::Player(me) if me == winner => (Tone::Positive, "You won"),
                Seat::Player(_) => (Tone::Danger, "You lost"),
                Seat::Both => (Tone::Positive, "Well played"),
            };
            let headline = format!("{} wins", color_name(winner));
            (title, tone, headline, personal)
        }
        Outcome::Draw(reason) => (
            "DRAW",
            Tone::Muted,
            reason.label().to_string(),
            "Drawn game",
        ),
    };
    let moves = g.game().history().len().div_ceil(2);
    let area = centered(body, 44, 11);
    let inner = modal(frame, area, "GAME OVER", tone, theme);
    let lines = vec![
        Line::styled(title, theme.toned(tone).add_modifier(Modifier::BOLD)),
        Line::raw(""),
        Line::styled(headline, theme.text().add_modifier(Modifier::BOLD)),
        Line::styled(
            format!("{personal} · {} · {moves} moves", outcome.score()),
            theme.muted(),
        ),
        Line::raw(""),
        Line::from(vec![
            Span::styled(" ↵ Review board ", theme.pill(Tone::Accent)),
            Span::raw("   "),
            Span::styled("n", theme.key()),
            Span::styled(" Home   ", theme.muted()),
            Span::styled("q", theme.key()),
            Span::styled(" Quit", theme.muted()),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), inner);
}

fn help(frame: &mut Frame, body: Rect, app: &App, theme: &Theme) {
    let area = centered(body, 66, 22);
    let inner = modal(frame, area, "HOW TO PLAY", Tone::Accent, theme);
    let row = |k: &str, v: &str| {
        Line::from(vec![
            Span::styled(
                format!("  {k:<14}"),
                theme.key().add_modifier(Modifier::BOLD),
            ),
            Span::styled(v.to_string(), theme.text()),
        ])
    };
    let head = |t: &str| Line::styled(t.to_string(), theme.label());
    let mut lines = vec![
        head("BOARD"),
        row("↑↓←→  hjkl", "Move the cursor"),
        row("↵  Space", "Select a piece, then a highlighted square"),
        row("/", "Type a move: e2e4 · e7e8q · e1g1 castles"),
        row("Esc", "Clear selection / close dialog"),
        Line::raw(""),
        head("GAME"),
        row("Tab", "Focus board → moves → actions"),
        row("o  g", "Flip board · switch Unicode / ASCII pieces"),
        row("r  d", "Resign · claim a draw (when available)"),
        row("q", "Leave the table"),
        Line::raw(""),
        head("PLAYING ONLINE"),
        Line::styled(
            "  Host: vesper host   ·   Join: address, code, and the host key",
            theme.text(),
        ),
        Line::styled(
            "  Direct connection — same network, VPN or SSH tunnel.",
            theme.muted(),
        ),
    ];
    if app.demo {
        lines.push(Line::raw(""));
        lines.push(Line::styled(
            "  Demo: simulated opponent · x previews a dropped connection",
            theme.faint(),
        ));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}
