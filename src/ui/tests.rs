use std::time::Instant;

use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::buffer::Buffer;

use super::*;
use crate::app::fixtures;

const SUPPORTED: [(u16, u16); 4] = [(120, 36), (100, 32), (90, 28), (80, 24)];

fn render(app: &App, w: u16, h: u16, theme: &Theme) -> Buffer {
    let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
    t.draw(|f| draw(f, app, theme)).unwrap();
    t.backend().buffer().clone()
}

fn text(buf: &Buffer) -> String {
    let mut s = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            s.push_str(buf[(x, y)].symbol());
        }
        s.push('\n');
    }
    s
}

fn scene(name: &str) -> App {
    fixtures::scenes(Instant::now())
        .into_iter()
        .find(|(n, _)| *n == name)
        .unwrap_or_else(|| panic!("no scene {name}"))
        .1
}

#[test]
fn every_scene_renders_at_every_size_in_both_themes() {
    for theme in [Theme::dark(), Theme::mono()] {
        for (name, app) in fixtures::scenes(Instant::now()) {
            for (w, h) in SUPPORTED.into_iter().chain([(70, 20), (40, 10), (200, 60)]) {
                let buf = render(&app, w, h, &theme);
                assert_eq!(buf.area.width, w, "{name}");
            }
        }
    }
}

#[test]
fn whole_board_and_labels_visible_at_supported_sizes() {
    let theme = Theme::dark();
    for name in [
        "game-your-move",
        "game-black-check",
        "local",
        "reconnecting",
    ] {
        let app = scene(name);
        for (w, h) in SUPPORTED {
            let out = text(&render(&app, w, h, &theme));
            for rank in '1'..='8' {
                assert!(
                    out.lines().any(|l| l.contains(&format!(" {rank} "))),
                    "{name} {w}x{h}: rank {rank} missing"
                );
            }
            for file in ["a", "h"] {
                assert!(out.contains(file), "{name} {w}x{h}");
            }
            assert!(out.contains("VESPER"), "{name} {w}x{h}: header");
            assert!(out.contains("MOVES"), "{name} {w}x{h}: history panel");
        }
    }
}

#[test]
fn board_uses_largest_square_that_fits() {
    assert_eq!(board::fit(64, 33), Some(board::SquareSize { w: 7, h: 3 }));
    assert_eq!(board::fit(50, 21), Some(board::SquareSize { w: 5, h: 2 }));
    assert_eq!(board::fit(30, 11), Some(board::SquareSize { w: 3, h: 1 }));
    assert_eq!(board::fit(20, 8), None);
}

#[test]
fn below_minimum_shows_resize_hint() {
    let out = text(&render(&scene("game-your-move"), 70, 20, &Theme::dark()));
    assert!(out.contains("Terminal too small"));
    assert!(out.contains("70×20 · needs 80×24"));
}

#[test]
fn key_states_have_text_cues() {
    let theme = Theme::dark();
    let cases = [
        ("home", "Host a game"),
        ("waiting", "vesper join 192.168.1.20:4747 K7QF-M2XA"),
        ("connecting", "Connecting to 192.168.1.20:4747"),
        ("join-error", "Port must be a number"),
        ("game-your-move", "YOUR MOVE"),
        ("game-opponent-turn", "Waiting for Black's move"),
        ("game-black-check", "CHECK"),
        ("game-sending", "Sending move b1 → c3"),
        ("game-command", "MOVE ›"),
        ("promotion", "PROMOTE YOUR PAWN"),
        ("reconnecting", "RECONNECTING"),
        ("opponent-offline", "OPPONENT OFFLINE"),
        ("confirm-resign", "RESIGN THIS GAME?"),
        ("help", "HOW TO PLAY"),
        ("result-checkmate", "CHECKMATE"),
        ("result-reviewed", "board is read-only"),
        ("local", "PASS & PLAY"),
    ];
    for (name, needle) in cases {
        for (w, h) in SUPPORTED {
            let out = text(&render(&scene(name), w, h, &theme));
            assert!(out.contains(needle), "{name} at {w}x{h} lacks {needle:?}");
        }
    }
}

#[test]
fn promotion_dialog_lists_all_four_pieces_untruncated() {
    for (w, h) in SUPPORTED {
        let out = text(&render(&scene("promotion"), w, h, &Theme::dark()));
        for piece in ["Queen", "Rook", "Bishop", "Knight"] {
            assert!(out.contains(piece), "{piece} at {w}x{h}");
        }
    }
}

#[test]
fn mono_theme_marks_legal_squares_and_cursor_without_colour() {
    let out = text(&render(&scene("game-your-move"), 120, 36, &Theme::mono()));
    let board: String = out.lines().take(33).collect();
    assert_eq!(board.matches('•').count(), 3, "b1 knight has three targets");
    assert!(out.contains('┏') && out.contains('┛'), "cursor brackets");
}

#[test]
fn ascii_mode_uses_letters() {
    let out = text(&render(&scene("ascii"), 120, 36, &Theme::dark()));
    for glyph in ['♜', '♟', '♚', '♔', '♝'] {
        assert!(!out.contains(glyph), "unexpected {glyph} in ASCII mode");
    }
    assert!(out.contains(" N ") && out.contains(" k "));
}

#[test]
fn seat_token_never_rendered() {
    // The UI has no access to secrets by construction; guard the copy too.
    for (name, app) in fixtures::scenes(Instant::now()) {
        let out = text(&render(&app, 120, 36, &Theme::dark())).to_lowercase();
        assert!(!out.contains("token"), "{name}");
    }
}
