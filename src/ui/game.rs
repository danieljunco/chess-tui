//! In-game layout: hero board, player strips, status/moves/actions panels.

use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};

use super::board;
use super::theme::Theme;
use super::{section_title, spinner};
use crate::app::{App, Focus, GameScreen, Link, Seat, Table, Tone};
use crate::domain::{Color, Outcome, Role, color_name};

pub fn draw(frame: &mut Frame, area: Rect, g: &GameScreen, app: &App, theme: &Theme) {
    let wide = area.width >= 120;
    let rail_w = if wide { 26 } else { 0 };
    let side_w = if wide || area.width < 100 { 30 } else { 32 };

    let [rail, center, side] = Layout::horizontal([
        Constraint::Length(rail_w),
        Constraint::Fill(1),
        Constraint::Length(side_w),
    ])
    .areas(area);

    if wide {
        let actions_h = g.actions().len() as u16 + 2;
        let [status, actions, _] = Layout::vertical([
            Constraint::Length(11),
            Constraint::Length(actions_h),
            Constraint::Fill(1),
        ])
        .areas(rail.inner(ratatui::layout::Margin::new(1, 0)));
        status_panel(frame, status, g, theme);
        actions_panel(frame, actions, g, theme);

        let [table, moves] = Layout::vertical([Constraint::Length(8), Constraint::Fill(1)])
            .areas(side.inner(ratatui::layout::Margin::new(1, 0)));
        table_panel(frame, table, g, app.ascii, theme);
        moves_panel(frame, moves, g, theme);
    } else {
        let actions_h = g.actions().len() as u16 + 2;
        let [status, moves, actions] = Layout::vertical([
            Constraint::Length(9),
            Constraint::Fill(1),
            Constraint::Length(actions_h),
        ])
        .areas(side.inner(ratatui::layout::Margin::new(1, 0)));
        status_panel(frame, status, g, theme);
        moves_panel(frame, moves, g, theme);
        actions_panel(frame, actions, g, theme);
    }

    board_area(frame, center, g, app, theme);
}

fn board_area(frame: &mut Frame, area: Rect, g: &GameScreen, app: &App, theme: &Theme) {
    let Some(size) = board::fit(area.width, area.height.saturating_sub(2)) else {
        return;
    };
    let (bw, bh) = size.outer();
    let spare = area.height.saturating_sub(bh + 2);
    let gap = u16::from(spare >= 4);
    let total_h = bh + 2 + gap * 2;
    let x = area.x + (area.width - bw) / 2;
    let y = area.y + (area.height.saturating_sub(total_h)) / 2;

    let top_color = !g.orientation;
    let strip_w = size.w * 8;
    let strip_x = x + 3;
    player_strip(
        frame,
        Rect::new(strip_x, y, strip_w, 1),
        g,
        top_color,
        app.ascii,
        theme,
    );
    let board_y = y + 1 + gap;
    board::render(
        frame.buffer_mut(),
        (x, board_y),
        size,
        g,
        app.ascii,
        g.focus == Focus::Board && app.overlay.is_none(),
        theme,
    );
    player_strip(
        frame,
        Rect::new(strip_x, board_y + bh + gap, strip_w, 1),
        g,
        g.orientation,
        app.ascii,
        theme,
    );
}

fn value(role: Role) -> u32 {
    match role {
        Role::Pawn => 1,
        Role::Knight | Role::Bishop => 3,
        Role::Rook => 5,
        Role::Queen => 9,
        Role::King => 0,
    }
}

/// Pieces captured *by* `color`, sorted most valuable first, plus material lead.
fn captures_by(g: &GameScreen, color: Color) -> (Vec<Role>, i32) {
    let mut taken = g.game().captured(!color);
    taken.sort_by_key(|r| std::cmp::Reverse(value(*r)));
    let lost: u32 = g.game().captured(color).iter().map(|r| value(*r)).sum();
    let won: u32 = taken.iter().map(|r| value(*r)).sum();
    (taken, won as i32 - lost as i32)
}

fn player_strip(
    frame: &mut Frame,
    area: Rect,
    g: &GameScreen,
    color: Color,
    ascii: bool,
    theme: &Theme,
) {
    let to_move = !g.game().is_over() && g.game().turn() == color;
    let name = g.view.name(color).to_string();
    let marker = if to_move { "▶ " } else { "  " };
    let name_style = if to_move {
        theme.text().add_modifier(Modifier::BOLD)
    } else {
        theme.muted()
    };
    let mut left = vec![
        Span::styled(marker, theme.accent()),
        Span::styled(name, name_style),
        Span::styled(
            format!("  {}", color_name(color).to_uppercase()),
            theme.faint(),
        ),
    ];
    if let (Seat::Player(me), Link::OpponentOffline) = (g.view.seat, g.view.link)
        && me != color
    {
        left.push(Span::styled("  ○ offline", theme.toned(Tone::Warning)));
    }
    frame.render_widget(Paragraph::new(Line::from(left)), area);

    let (taken, lead) = captures_by(g, color);
    let glyphs: String = taken
        .iter()
        .map(|r| {
            board::glyph(
                crate::domain::Piece {
                    color: !color,
                    role: *r,
                },
                ascii,
                theme,
            )
        })
        .collect();
    let mut right = vec![Span::styled(glyphs, theme.muted())];
    if lead > 0 {
        right.push(Span::styled(format!(" +{lead}"), theme.text()));
    }
    frame.render_widget(Paragraph::new(Line::from(right).right_aligned()), area);
}

fn panel<'a>(title: &str, focused: bool, theme: &Theme) -> Block<'a> {
    Block::new()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme.border(focused))
        .title(section_title(title, focused, theme))
        .style(Style::new().bg(theme.canvas))
}

pub fn turn_pill(g: &GameScreen) -> (String, Tone) {
    let game = g.game();
    if let Some(outcome) = game.outcome() {
        let tone = match (outcome, g.view.seat) {
            (Outcome::Draw(_), _) => Tone::Muted,
            (Outcome::Win { winner, .. }, Seat::Player(me)) if winner != me => Tone::Danger,
            _ => Tone::Positive,
        };
        return ("GAME OVER".into(), tone);
    }
    if !g.view.link.can_play() {
        return ("PAUSED".into(), Tone::Warning);
    }
    if g.pending.is_some() {
        return ("SENDING".into(), Tone::Accent);
    }
    if g.view.my_turn() {
        if game.is_check() {
            return ("CHECK".into(), Tone::Danger);
        }
        let label = match g.view.seat {
            Seat::Both => format!("{} TO MOVE", color_name(game.turn()).to_uppercase()),
            Seat::Player(_) => "YOUR MOVE".into(),
        };
        return (label, Tone::Accent);
    }
    ("OPPONENT THINKING".into(), Tone::Muted)
}

fn status_panel(frame: &mut Frame, area: Rect, g: &GameScreen, theme: &Theme) {
    let block = panel("GAME", false, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);
    let game = g.game();

    let (pill, tone) = turn_pill(g);
    let mut lines = vec![Line::from(vec![
        Span::raw(" "),
        Span::styled(format!(" {pill} "), theme.pill(tone)),
    ])];

    let detail = if let Some(outcome) = game.outcome() {
        let (who, why) = match outcome {
            Outcome::Win { winner, reason } => (
                format!("{} wins", color_name(winner)),
                match reason {
                    crate::domain::WinReason::Checkmate => "by checkmate",
                    crate::domain::WinReason::Resignation => "by resignation",
                },
            ),
            Outcome::Draw(reason) => ("Drawn game".to_string(), reason.label()),
        };
        lines.push(Line::styled(
            format!(" {who}"),
            theme.text().add_modifier(Modifier::BOLD),
        ));
        Line::styled(format!(" {why}"), theme.muted())
    } else if game.is_check() && g.view.my_turn() {
        Line::styled(" Protect your king", theme.toned(Tone::Danger))
    } else {
        let mover = color_name(game.turn());
        let n = game.history().len() / 2 + 1;
        Line::from(vec![
            Span::styled(format!(" {mover}"), theme.text()),
            Span::styled(format!(" · move {n}"), theme.muted()),
        ])
    };
    lines.push(detail);
    lines.push(Line::raw(""));

    lines.push(Line::styled(" LAST MOVE", theme.label()));
    match game.last_move() {
        Some(m) => {
            let n = game.history().len().div_ceil(2);
            let num = if m.color == Color::White {
                format!("{n}.")
            } else {
                format!("{n}…")
            };
            lines.push(Line::from(vec![
                Span::styled(
                    format!(" {num} {}", m.san),
                    theme.text().add_modifier(Modifier::BOLD),
                ),
                Span::styled(format!("  {} → {}", m.from, m.to), theme.muted()),
            ]));
        }
        None => lines.push(Line::styled(" —", theme.faint())),
    }

    if let Some(reason) = game.claimable_draw()
        && g.view.my_turn()
    {
        lines.push(Line::raw(""));
        lines.push(Line::from(vec![
            Span::styled(" Draw claimable ", theme.toned(Tone::Accent)),
            Span::styled(format!("· {} ", reason.label()), theme.muted()),
            Span::styled("d", theme.key()),
        ]));
    } else if inner.height >= 9 {
        lines.push(Line::raw(""));
        lines.push(Line::styled(" YOU PLAY", theme.label()));
        let you = match g.view.seat {
            Seat::Both => "Both sides · pass & play".to_string(),
            Seat::Player(c) => color_name(c).to_string(),
        };
        lines.push(Line::styled(format!(" {you}"), theme.text()));
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn table_panel(frame: &mut Frame, area: Rect, g: &GameScreen, ascii: bool, theme: &Theme) {
    let block = panel("TABLE", false, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let presence = |color: Color| -> (String, Tone) {
        match (g.view.seat, g.view.link) {
            (Seat::Both, _) => ("● here".into(), Tone::Positive),
            (Seat::Player(me), _) if me == color => ("● you".into(), Tone::Accent),
            (_, Link::OpponentOffline) => ("○ offline".into(), Tone::Warning),
            (_, Link::Reconnecting { .. }) => ("○ unknown".into(), Tone::Warning),
            (_, Link::HostClosed) => ("■ gone".into(), Tone::Danger),
            _ => ("● online".into(), Tone::Positive),
        }
    };
    let w = inner.width as usize;
    let row = |color: Color| {
        let name = g.view.name(color);
        let glyph = match (color, ascii) {
            (Color::White, false) => "♔",
            (Color::Black, false) => "♚",
            (Color::White, true) => "K",
            (Color::Black, true) => "k",
        };
        let (p, tone) = presence(color);
        let left = format!(" {glyph} {name}");
        let pad = w.saturating_sub(left.chars().count() + p.chars().count() + 1);
        Line::from(vec![
            Span::styled(left, theme.text()),
            Span::raw(" ".repeat(pad)),
            Span::styled(p, theme.toned(tone)),
        ])
    };
    let (place, addr) = match &g.view.table {
        Table::Local => ("Played on this terminal", String::new()),
        Table::Hosting { addr } => ("Hosted on this terminal", addr.clone()),
        Table::Joined { addr } => ("Connected to host", addr.clone()),
    };
    let lines = vec![
        row(Color::White),
        row(Color::Black),
        Line::raw(""),
        Line::styled(format!(" {place}"), theme.muted()),
        Line::from(vec![
            Span::styled(format!(" {addr}"), theme.faint()),
            Span::styled(
                if addr.is_empty() {
                    String::new()
                } else {
                    format!(" · v{}", g.view.version)
                },
                theme.faint(),
            ),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines), inner);
}

fn moves_panel(frame: &mut Frame, area: Rect, g: &GameScreen, theme: &Theme) {
    let focused = g.focus == Focus::History;
    let block = panel("MOVES", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let history = g.game().history();
    if history.is_empty() {
        frame.render_widget(
            Paragraph::new(vec![
                Line::raw(""),
                Line::styled(" No moves yet", theme.muted()),
                Line::styled(" White starts the game", theme.faint()),
            ]),
            inner,
        );
        return;
    }

    let rows = history.len().div_ceil(2);
    let visible = inner.height as usize;
    let anchor = g.history_scroll.unwrap_or(rows - 1).min(rows - 1);
    let start = (anchor + 1).saturating_sub(visible);
    let last_ply = history.len() - 1;

    let mut lines = Vec::new();
    for row in start..rows.min(start + visible) {
        let is_anchor = g.history_scroll == Some(row) && focused;
        let num = Span::styled(format!(" {:>3}. ", row + 1), theme.faint());
        let cell = |ply: usize| -> Span<'static> {
            match history.get(ply) {
                Some(m) => {
                    let style = if ply == last_ply {
                        theme.accent()
                    } else {
                        theme.text()
                    };
                    Span::styled(format!("{:<9}", m.san), style)
                }
                None => Span::raw(" ".repeat(9)),
            }
        };
        let mut line = Line::from(vec![num, cell(row * 2), cell(row * 2 + 1)]);
        if is_anchor {
            line = line.style(theme.selected_row());
        }
        lines.push(line);
    }
    frame.render_widget(Paragraph::new(lines), inner);
}

fn actions_panel(frame: &mut Frame, area: Rect, g: &GameScreen, theme: &Theme) {
    let focused = g.focus == Focus::Actions;
    let block = panel("ACTIONS", focused, theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    let w = inner.width as usize;
    let lines: Vec<Line> = g
        .actions()
        .iter()
        .enumerate()
        .map(|(i, a)| {
            let selected = focused && i == g.action.min(g.actions().len() - 1);
            let label = if *a == crate::app::Action::Leave && g.game().is_over() {
                "Back to home"
            } else {
                a.label()
            };
            let left = format!("{}{label}", if selected { " ▸ " } else { "   " });
            let key = a.key().to_string();
            let pad = w.saturating_sub(left.chars().count() + key.len() + 1);
            let line = Line::from(vec![
                Span::styled(
                    left,
                    if selected {
                        theme.text()
                    } else {
                        theme.muted()
                    },
                ),
                Span::raw(" ".repeat(pad)),
                Span::styled(key, theme.key()),
            ]);
            if selected {
                line.style(theme.selected_row())
            } else {
                line
            }
        })
        .collect();
    frame.render_widget(Paragraph::new(lines), inner);
}

pub fn status_strip(frame: &mut Frame, area: Rect, g: &GameScreen, app: &App, theme: &Theme) {
    if let Some(input) = &g.command {
        let mut spans = vec![
            Span::styled(" MOVE › ", theme.pill(Tone::Accent)),
            Span::styled(
                format!(" {}", input.text),
                theme.text().add_modifier(Modifier::BOLD),
            ),
            Span::styled("▏", Style::new().fg(theme.accent)),
        ];
        match &input.error {
            Some(err) => spans.push(Span::styled(format!("   {err}"), theme.toned(Tone::Danger))),
            None => spans.push(Span::styled(
                "   e.g. e2e4 · e7e8q · castle with e1g1",
                theme.faint(),
            )),
        }
        frame.render_widget(Paragraph::new(Line::from(spans)), area);
        return;
    }

    let line = if let Some(pending) = &g.pending {
        Line::from(vec![
            Span::styled(format!(" {} ", spinner(app)), theme.toned(Tone::Accent)),
            Span::styled(pending.clone(), theme.toned(Tone::Accent)),
        ])
    } else if let Some(status) = &g.status {
        Line::from(vec![
            Span::styled(" ● ", theme.toned(status.tone)),
            Span::styled(status.text.clone(), theme.toned(status.tone)),
        ])
    } else {
        default_status(g, app, theme)
    };
    frame.render_widget(Paragraph::new(line), area);
}

fn default_status(g: &GameScreen, app: &App, theme: &Theme) -> Line<'static> {
    let game = g.game();
    if let Some(outcome) = game.outcome() {
        return Line::from(vec![
            Span::styled(format!(" {} ", outcome.score()), theme.pill(Tone::Muted)),
            Span::styled(format!("  {}", outcome.headline()), theme.text()),
            Span::styled("  · board is read-only", theme.faint()),
        ]);
    }
    match g.view.link {
        Link::Reconnecting { .. } => {
            return Line::from(vec![
                Span::styled(format!(" {} ", spinner(app)), theme.toned(Tone::Warning)),
                Span::styled(
                    "Connection lost · reconnecting · board is read-only",
                    theme.toned(Tone::Warning),
                ),
            ]);
        }
        Link::OpponentOffline => {
            return Line::styled(
                " ○ Opponent offline · the game is kept while you stay",
                theme.toned(Tone::Warning),
            );
        }
        Link::HostClosed => {
            return Line::styled(" ■ Host closed the game", theme.toned(Tone::Danger));
        }
        _ => {}
    }
    if g.view.my_turn() {
        if let Some(sel) = g.selected {
            let n = g.targets.len();
            return Line::from(vec![
                Span::styled(format!(" {sel} selected"), theme.text()),
                Span::styled(
                    format!(
                        " · {n} legal square{} marked •",
                        if n == 1 { "" } else { "s" }
                    ),
                    theme.muted(),
                ),
            ]);
        }
        Line::from(vec![
            Span::styled(
                format!(" Your move · {}", color_name(game.turn())),
                theme.text(),
            ),
            Span::styled(
                "  — select a piece with ↵, or press / to type e2e4",
                theme.faint(),
            ),
        ])
    } else {
        Line::from(vec![
            Span::styled(format!(" {} ", spinner(app)), theme.muted()),
            Span::styled(
                format!("Waiting for {}'s move", color_name(game.turn())),
                theme.muted(),
            ),
        ])
    }
}
