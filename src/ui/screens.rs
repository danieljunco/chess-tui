//! Home, join form, waiting room, connecting and too-small screens.

use ratatui::Frame;
use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::theme::Theme;
use super::{MIN_HEIGHT, MIN_WIDTH, centered};
use crate::app::{JoinForm, MenuItem, WaitingInfo};
use crate::domain::color_name;

const ORNAMENT: &str = "♜  ♞  ♝  ♛  ♚  ♝  ♞  ♜";

pub fn home(frame: &mut Frame, area: Rect, selected: usize, theme: &Theme) {
    let card = centered(area, 60, 18);
    let mut lines = vec![
        Line::styled(ORNAMENT, theme.faint()),
        Line::raw(""),
        Line::styled("V E S P E R", theme.accent()),
        Line::styled("C H E S S", theme.muted()),
        Line::raw(""),
        Line::styled("─────────────────────", theme.faint()),
        Line::raw(""),
    ];
    for (i, item) in MenuItem::ALL.iter().enumerate() {
        let label = format!("{:<28}", item.label());
        if i == selected {
            lines.push(
                Line::from(vec![
                    Span::styled(" ▸  ", theme.accent()),
                    Span::styled(label, theme.text().add_modifier(Modifier::BOLD)),
                ])
                .style(theme.selected_row()),
            );
        } else {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(label, theme.muted()),
            ]));
        }
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        MenuItem::ALL[selected.min(MenuItem::ALL.len() - 1)].hint(),
        theme.faint(),
    ));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "PEER TO PEER · LIVE · IN YOUR TERMINAL",
        theme.faint(),
    ));
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), card);
}

#[allow(clippy::too_many_arguments)]
fn input_box(
    frame: &mut Frame,
    area: Rect,
    label: &str,
    value: &str,
    placeholder: &str,
    focused: bool,
    error: bool,
    theme: &Theme,
) {
    let border = if error {
        Style::new().fg(theme.danger)
    } else {
        theme.border(focused)
    };
    let title = if focused {
        Span::styled(format!(" {label} "), theme.accent())
    } else {
        Span::styled(format!(" {label} "), theme.label())
    };
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border)
        .title(Line::from(title));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let caret = Span::styled(
        if focused { "▏" } else { "" },
        Style::new().fg(theme.accent),
    );
    let content = if value.is_empty() {
        Line::from(vec![
            Span::raw(" "),
            caret,
            Span::styled(placeholder.to_string(), theme.faint()),
        ])
    } else {
        Line::from(vec![Span::styled(format!(" {value}"), theme.text()), caret])
    };
    frame.render_widget(Paragraph::new(content), inner);
}

pub fn join(frame: &mut Frame, area: Rect, form: &JoinForm, theme: &Theme) {
    let card = centered(area, 52, 18);
    let title = vec![
        Line::styled("JOIN A GAME", theme.accent()),
        Line::styled("Connect directly to your friend's terminal", theme.muted()),
    ];
    frame.render_widget(
        Paragraph::new(title).alignment(Alignment::Center),
        Rect::new(card.x, card.y, card.width, 2),
    );
    let err_field = form.error.as_ref().map(|(f, _)| *f);
    input_box(
        frame,
        Rect::new(card.x + 2, card.y + 3, card.width - 4, 3),
        "HOST ADDRESS",
        &form.address,
        "192.168.1.20:4747",
        form.field == 0,
        err_field == Some(0),
        theme,
    );
    input_box(
        frame,
        Rect::new(card.x + 2, card.y + 7, card.width - 4, 3),
        "INVITE CODE",
        &form.code,
        "K7QF-M2XA",
        form.field == 1,
        err_field == Some(1),
        theme,
    );
    let mut lines = Vec::new();
    match &form.error {
        Some((_, msg)) => lines.push(Line::styled(
            format!("● {msg}"),
            Style::new().fg(theme.danger),
        )),
        None => lines.push(Line::raw("")),
    }
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "The host sees both on their waiting screen.",
        theme.faint(),
    ));
    lines.push(Line::styled(
        "Port defaults to 4747. Works on the same network,",
        theme.faint(),
    ));
    lines.push(Line::styled("a VPN or an SSH tunnel.", theme.faint()));
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        Rect::new(card.x, card.y + 11, card.width, 7),
    );
}

pub fn waiting(frame: &mut Frame, area: Rect, info: &WaitingInfo, spin: &str, theme: &Theme) {
    let card = centered(area, 62, 19);
    if info.invite.is_empty() {
        connecting_card(frame, card, "Opening your table…", spin, theme);
        return;
    }
    let addr = info
        .addresses
        .first()
        .map(|a| format!("{a}:{}", info.port))
        .unwrap_or_else(|| format!("<your-ip>:{}", info.port));
    let mut lines = vec![
        Line::styled("YOUR TABLE IS READY", theme.accent()),
        Line::styled(
            format!(
                "You play {} · your friend gets the other side",
                color_name(info.seat)
            ),
            theme.muted(),
        ),
        Line::raw(""),
    ];
    frame.render_widget(
        Paragraph::new(std::mem::take(&mut lines)).alignment(Alignment::Center),
        Rect::new(card.x, card.y, card.width, 3),
    );

    let code_box = centered(Rect::new(card.x, card.y + 3, card.width, 3), 22, 3);
    let block = Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(theme.accent))
        .style(Style::new().bg(theme.raised));
    let inner = block.inner(code_box);
    frame.render_widget(block, code_box);
    frame.render_widget(
        Paragraph::new(Line::styled(
            info.invite.clone(),
            theme.text().add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        inner,
    );

    lines.push(Line::styled("INVITE CODE · single use", theme.faint()));
    lines.push(Line::raw(""));
    lines.push(Line::styled("Ask your friend to run", theme.muted()));
    let key = if info.fingerprint.is_empty() {
        String::new()
    } else {
        format!(" {}", info.fingerprint)
    };
    lines.push(Line::styled(
        format!("vesper join {addr} {}{key}", info.invite),
        theme.accent(),
    ));
    if info.addresses.len() > 1 {
        let others: Vec<String> = info.addresses[1..]
            .iter()
            .map(|a| format!("{a}:{}", info.port))
            .collect();
        lines.push(Line::styled(
            format!("also reachable at {}", others.join(" · ")),
            theme.faint(),
        ));
    }
    lines.push(Line::raw(""));
    lines.push(Line::from(vec![
        Span::styled(format!("{spin} "), theme.toned(crate::app::Tone::Accent)),
        Span::styled(
            format!("Waiting for {}…", color_name(!info.seat)),
            theme.text(),
        ),
    ]));
    lines.push(Line::raw(""));
    lines.push(Line::styled(
        "127.0.0.1 is this computer only. The address above is your network.",
        theme.faint(),
    ));
    if let Some(public) = &info.public_ip {
        lines.push(Line::styled(
            format!(
                "Outside: forward TCP {} to this machine, then join {public}",
                info.port
            ),
            theme.faint(),
        ));
    } else {
        lines.push(Line::styled(
            "Outside your network: forward this TCP port, or join with a VPN address.",
            theme.faint(),
        ));
    }
    lines.push(Line::styled(
        "The connection is encrypted. The host key pins who you joined.",
        theme.faint(),
    ));
    frame.render_widget(
        Paragraph::new(lines).alignment(Alignment::Center),
        Rect::new(
            card.x,
            card.y + 6,
            card.width,
            card.height.saturating_sub(6),
        ),
    );
}

fn connecting_card(frame: &mut Frame, area: Rect, text: &str, spin: &str, theme: &Theme) {
    let lines = vec![
        Line::raw(""),
        Line::raw(""),
        Line::raw(""),
        Line::from(vec![
            Span::styled(format!("{spin} "), theme.toned(crate::app::Tone::Accent)),
            Span::styled(text.to_string(), theme.text()),
        ]),
        Line::raw(""),
        Line::styled("Esc to cancel", theme.faint()),
    ];
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), area);
}

pub fn connecting(frame: &mut Frame, area: Rect, addr: &str, spin: &str, theme: &Theme) {
    let card = centered(area, 60, 12);
    let text = if addr == "local" {
        "Setting up the board…".to_string()
    } else {
        format!("Connecting to {addr}…")
    };
    connecting_card(frame, card, &text, spin, theme);
}

pub fn too_small(frame: &mut Frame, area: Rect, theme: &Theme) {
    let lines = vec![
        Line::styled("♞ VESPER", theme.accent()),
        Line::raw(""),
        Line::styled(
            "Terminal too small",
            theme.text().add_modifier(Modifier::BOLD),
        ),
        Line::styled(
            format!(
                "{}×{} · needs {MIN_WIDTH}×{MIN_HEIGHT}",
                area.width, area.height
            ),
            theme.muted(),
        ),
        Line::raw(""),
        Line::styled("Enlarge the window or reduce the font size", theme.faint()),
    ];
    let h = lines.len() as u16;
    let rect = Rect::new(
        area.x,
        area.y + area.height.saturating_sub(h) / 2,
        area.width,
        h.min(area.height),
    );
    frame.render_widget(Paragraph::new(lines).alignment(Alignment::Center), rect);
}
