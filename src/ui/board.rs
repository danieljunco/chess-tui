//! The hero chessboard. Draws straight into the buffer so every square has
//! an exact, consistent cell geometry.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};

use super::theme::Theme;
use crate::app::GameScreen;
use crate::domain::{self, File, Piece, Rank, Role, Square};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SquareSize {
    pub w: u16,
    pub h: u16,
}

pub const SIZES: [SquareSize; 3] = [
    SquareSize { w: 7, h: 3 },
    SquareSize { w: 5, h: 2 },
    SquareSize { w: 3, h: 1 },
];

const RANK_GUTTER: u16 = 3;

impl SquareSize {
    /// Board incl. rank/file labels.
    pub fn outer(self) -> (u16, u16) {
        (self.w * 8 + RANK_GUTTER * 2, self.h * 8 + 2)
    }
}

/// Largest square size whose labelled board fits in `area`.
pub fn fit(width: u16, height: u16) -> Option<SquareSize> {
    SIZES.into_iter().find(|s| {
        let (w, h) = s.outer();
        w <= width && h <= height
    })
}

pub fn glyph(piece: Piece, ascii: bool, theme: &Theme) -> char {
    if ascii {
        let c = piece.role.char();
        return match piece.color {
            domain::Color::White => c.to_ascii_uppercase(),
            domain::Color::Black => c,
        };
    }
    // Filled silhouettes read best as coloured shapes; without colour, the
    // outline/filled distinction carries the side.
    let filled = theme.color || piece.color == domain::Color::Black;
    match (piece.role, filled) {
        (Role::King, true) => '♚',
        (Role::Queen, true) => '♛',
        (Role::Rook, true) => '♜',
        (Role::Bishop, true) => '♝',
        (Role::Knight, true) => '♞',
        (Role::Pawn, true) => '♟',
        (Role::King, false) => '♔',
        (Role::Queen, false) => '♕',
        (Role::Rook, false) => '♖',
        (Role::Bishop, false) => '♗',
        (Role::Knight, false) => '♘',
        (Role::Pawn, false) => '♙',
    }
}

pub fn piece_style(color: domain::Color, theme: &Theme) -> Style {
    let fg = match color {
        domain::Color::White => theme.piece_white,
        domain::Color::Black => theme.piece_black,
    };
    Style::new().fg(fg).add_modifier(Modifier::BOLD)
}

/// Screen position (col, row from top) → square, honouring orientation.
fn square_at(col: u16, row: u16, white_bottom: bool) -> Square {
    let (f, r) = if white_bottom {
        (col, 7 - row)
    } else {
        (7 - col, row)
    };
    Square::from_coords(File::new(f as u32), Rank::new(r as u32))
}

pub fn render(
    buf: &mut Buffer,
    origin: (u16, u16),
    size: SquareSize,
    g: &GameScreen,
    ascii: bool,
    focused: bool,
    theme: &Theme,
) {
    let (x0, y0) = origin;
    let white_bottom = g.orientation == domain::Color::White;
    let game = g.game();
    let last = game.last_move().map(|m| (m.from, m.to));
    let checked = game.checked_king();
    let label = if focused {
        theme.muted()
    } else {
        theme.faint()
    };

    // File labels, top and bottom.
    for col in 0..8u16 {
        let sq = square_at(col, 0, white_bottom);
        let ch = sq.file().char().to_string();
        let x = x0 + RANK_GUTTER + col * size.w + size.w / 2;
        buf.set_string(x, y0, &ch, label);
        buf.set_string(x, y0 + 1 + size.h * 8, &ch, label);
    }

    for row in 0..8u16 {
        let sq_row_y = y0 + 1 + row * size.h;
        let rank = square_at(0, row, white_bottom).rank().char().to_string();
        let mid = sq_row_y + size.h / 2;
        buf.set_string(x0 + 1, mid, &rank, label);
        buf.set_string(x0 + RANK_GUTTER + size.w * 8 + 1, mid, &rank, label);

        for col in 0..8u16 {
            let sq = square_at(col, row, white_bottom);
            let area = Rect::new(x0 + RANK_GUTTER + col * size.w, sq_row_y, size.w, size.h);
            draw_square(buf, area, sq, g, last, checked, ascii, theme);
        }
    }

    // Focus rail: a thin accent bar alongside the board.
    if focused && theme.color {
        for y in y0 + 1..y0 + 1 + size.h * 8 {
            buf.set_string(x0, y, "▎", Style::new().fg(theme.accent));
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_square(
    buf: &mut Buffer,
    area: Rect,
    sq: Square,
    g: &GameScreen,
    last: Option<(Square, Square)>,
    checked: Option<Square>,
    ascii: bool,
    theme: &Theme,
) {
    let light = sq.is_light();
    let piece = g.game().piece_at(sq);
    let is_cursor = g.cursor == sq;
    let is_selected = g.selected == Some(sq);
    let is_target = g.targets.contains(&sq);
    let is_last = last.is_some_and(|(f, t)| f == sq || t == sq);
    let is_check = checked == Some(sq);

    let pick = |l: Color, d: Color| if light { l } else { d };
    let bg = if is_selected {
        theme.square_selected
    } else if is_target {
        pick(theme.legal_light, theme.legal_dark)
    } else if is_check {
        theme.check
    } else if is_last {
        pick(theme.last_light, theme.last_dark)
    } else {
        pick(theme.square_light, theme.square_dark)
    };

    let mut base = Style::new().bg(bg);
    if !theme.color && (is_selected || is_check) {
        base = base.add_modifier(Modifier::REVERSED);
    }
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            buf[(x, y)].reset();
            buf[(x, y)].set_style(base);
            buf[(x, y)].set_char(' ');
        }
    }

    let cx = area.x + area.width / 2;
    let cy = area.y + (area.height.saturating_sub(1)) / 2;

    match piece {
        Some(p) => {
            let mut style = base.patch(piece_style(p.color, theme));
            if !theme.color && p.color == domain::Color::White {
                style = style.add_modifier(Modifier::BOLD);
            }
            buf[(cx, cy)]
                .set_char(glyph(p, ascii, theme))
                .set_style(style);
        }
        None if is_target => {
            let dot = if ascii { '*' } else { '•' };
            let fg = if theme.color {
                Color::Rgb(0x2E, 0x5A, 0x3B)
            } else {
                Color::Reset
            };
            buf[(cx, cy)]
                .set_char(dot)
                .set_style(base.fg(fg).add_modifier(Modifier::BOLD));
        }
        None if is_last && !theme.color => {
            buf[(cx, cy)].set_char('·');
        }
        None => {}
    }

    // Non-colour redundancy for captures / check on wide squares.
    if area.width >= 5 {
        let marker = if is_target && piece.is_some() {
            Some('×')
        } else if is_check {
            Some('!')
        } else {
            None
        };
        if let Some(m) = marker {
            let fg = if theme.color {
                if is_check {
                    theme.piece_white
                } else {
                    Color::Rgb(0x1F, 0x45, 0x2B)
                }
            } else {
                Color::Reset
            };
            buf[(area.right() - 2, area.y)]
                .set_char(m)
                .set_style(base.fg(fg).add_modifier(Modifier::BOLD));
        }
    }

    if is_cursor {
        draw_cursor(buf, area, light, theme);
    }
}

fn draw_cursor(buf: &mut Buffer, area: Rect, light: bool, theme: &Theme) {
    let fg = if !theme.color {
        Color::Reset
    } else if light {
        theme.canvas
    } else {
        theme.piece_white
    };
    let style = Style::new().fg(fg).add_modifier(Modifier::BOLD);
    let (l, r, t, b) = (area.left(), area.right() - 1, area.top(), area.bottom() - 1);
    if area.height >= 2 {
        buf[(l, t)].set_char('┏').set_style(style);
        buf[(r, t)].set_char('┓').set_style(style);
        buf[(l, b)].set_char('┗').set_style(style);
        buf[(r, b)].set_char('┛').set_style(style);
    } else {
        buf[(l, t)].set_char('[').set_style(style);
        buf[(r, t)].set_char(']').set_style(style);
    }
}
