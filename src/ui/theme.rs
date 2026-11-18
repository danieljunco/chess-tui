//! Semantic theme tokens (docs/design/01-visual-system.md).

use ratatui::style::{Color, Modifier, Style};

pub use crate::app::Tone;

#[derive(Clone, Copy, Debug)]
pub struct Theme {
    pub color: bool,
    pub canvas: Color,
    pub panel: Color,
    pub raised: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub faint: Color,
    pub accent: Color,
    pub positive: Color,
    pub warning: Color,
    pub danger: Color,
    pub square_light: Color,
    pub square_dark: Color,
    pub square_selected: Color,
    pub legal_light: Color,
    pub legal_dark: Color,
    pub last_light: Color,
    pub last_dark: Color,
    pub check: Color,
    pub piece_white: Color,
    pub piece_black: Color,
    pub on_accent: Color,
}

const fn rgb(hex: u32) -> Color {
    Color::Rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Theme {
    pub const fn dark() -> Self {
        Self {
            color: true,
            canvas: rgb(0x14151C),
            panel: rgb(0x1A1C25),
            raised: rgb(0x262A37),
            border: rgb(0x2F3342),
            text: rgb(0xE8E9F1),
            muted: rgb(0x9DA4B7),
            faint: rgb(0x5D6377),
            accent: rgb(0xAFA1FF),
            positive: rgb(0x91CDA4),
            warning: rgb(0xE8C17C),
            danger: rgb(0xEC8E9D),
            square_light: rgb(0x8C94A4),
            square_dark: rgb(0x5B6475),
            square_selected: rgb(0xB8A8FF),
            legal_light: rgb(0x98BFA1),
            legal_dark: rgb(0x6F9479),
            last_light: rgb(0xB9B08F),
            last_dark: rgb(0x7F7A5E),
            check: rgb(0xE07A8B),
            piece_white: rgb(0xFBFAF5),
            piece_black: rgb(0x12131A),
            on_accent: rgb(0x14151C),
        }
    }

    /// Honors `NO_COLOR`: every state must still be readable through
    /// glyphs, brackets and text attributes alone.
    pub const fn mono() -> Self {
        let r = Color::Reset;
        Self {
            color: false,
            canvas: r,
            panel: r,
            raised: r,
            border: r,
            text: r,
            muted: r,
            faint: r,
            accent: r,
            positive: r,
            warning: r,
            danger: r,
            square_light: r,
            square_dark: r,
            square_selected: r,
            legal_light: r,
            legal_dark: r,
            last_light: r,
            last_dark: r,
            check: r,
            piece_white: r,
            piece_black: r,
            on_accent: r,
        }
    }

    pub fn base(&self) -> Style {
        Style::new().fg(self.text).bg(self.canvas)
    }

    pub fn panel(&self) -> Style {
        Style::new().fg(self.text).bg(self.panel)
    }

    pub fn text(&self) -> Style {
        Style::new().fg(self.text)
    }

    pub fn muted(&self) -> Style {
        Style::new().fg(self.muted)
    }

    pub fn faint(&self) -> Style {
        Style::new().fg(self.faint)
    }

    pub fn label(&self) -> Style {
        Style::new().fg(self.muted).add_modifier(Modifier::BOLD)
    }

    pub fn accent(&self) -> Style {
        Style::new().fg(self.accent).add_modifier(Modifier::BOLD)
    }

    pub fn key(&self) -> Style {
        Style::new().fg(self.accent)
    }

    pub fn border(&self, focused: bool) -> Style {
        if focused {
            Style::new().fg(self.accent)
        } else {
            Style::new().fg(self.border)
        }
    }

    /// Filled status pill, e.g. ` YOUR MOVE `.
    pub fn pill(&self, tone: Tone) -> Style {
        let bg = self.tone(tone);
        if self.color {
            Style::new()
                .fg(self.on_accent)
                .bg(bg)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD)
        }
    }

    pub fn tone(&self, tone: Tone) -> Color {
        match tone {
            Tone::Accent => self.accent,
            Tone::Positive => self.positive,
            Tone::Warning => self.warning,
            Tone::Danger => self.danger,
            Tone::Muted => self.muted,
        }
    }

    pub fn toned(&self, tone: Tone) -> Style {
        Style::new().fg(self.tone(tone))
    }

    pub fn selected_row(&self) -> Style {
        if self.color {
            Style::new().bg(self.raised).fg(self.text)
        } else {
            Style::new().add_modifier(Modifier::REVERSED)
        }
    }
}
