use std::time::{Duration, Instant};

use super::mock::{MockMode, MockProvider};
use super::*;
use crate::domain::{Outcome, WinReason};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn press(app: &mut App, codes: &[KeyCode]) {
    for c in codes {
        app.on_key(key(*c));
        app.tick(Instant::now());
    }
}

fn type_text(app: &mut App, text: &str) {
    for c in text.chars() {
        app.on_key(key(KeyCode::Char(c)));
    }
}

fn local_app_with(game: Game) -> App {
    let now = Instant::now();
    let factory: ProviderFactory = Box::new(move |_, now| {
        Box::new(MockProvider::new(MockMode::Local, now).with_game(game.clone()))
    });
    let mut app = App::new(factory, now);
    app.start(StartRequest::Local);
    app.tick(Instant::now());
    app
}

fn local_app() -> App {
    local_app_with(Game::new())
}

fn screen(app: &App) -> &GameScreen {
    match &app.screen {
        Screen::Game(g) => g,
        _ => panic!("expected game screen"),
    }
}

fn goto(app: &mut App, sq: Square) {
    if let Screen::Game(g) = &mut app.screen {
        g.cursor = sq;
    }
}

#[test]
fn home_menu_wraps_and_starts_local_game() {
    let mut app = App::new(fixtures::demo_factory(), Instant::now());
    press(&mut app, &[KeyCode::Up]);
    assert!(matches!(app.screen, Screen::Home { selected: 4 }));
    press(&mut app, &[KeyCode::Down, KeyCode::Down, KeyCode::Down]);
    assert!(matches!(app.screen, Screen::Home { selected: 2 }));
    press(&mut app, &[KeyCode::Enter]);
    let g = screen(&app);
    assert_eq!(g.view.seat, Seat::Both);
    assert_eq!(g.view.link, Link::Local);
}

#[test]
fn select_then_move_commits_through_provider() {
    let mut app = local_app();
    goto(&mut app, Square::E2);
    press(&mut app, &[KeyCode::Enter]);
    let g = screen(&app);
    assert_eq!(g.selected, Some(Square::E2));
    assert_eq!(g.targets.len(), 2);

    press(&mut app, &[KeyCode::Up, KeyCode::Up, KeyCode::Enter]);
    let g = screen(&app);
    assert_eq!(g.game().history().len(), 1);
    assert_eq!(g.game().history()[0].san, "e4");
    assert_eq!(g.selected, None);
    assert_eq!(g.pending, None);
    assert_eq!(g.game().turn(), Color::Black);
}

#[test]
fn illegal_destination_keeps_selection_and_board() {
    let mut app = local_app();
    goto(&mut app, Square::E2);
    press(&mut app, &[KeyCode::Enter]);
    goto(&mut app, Square::E5);
    press(&mut app, &[KeyCode::Enter]);
    let g = screen(&app);
    assert_eq!(g.selected, Some(Square::E2), "selection preserved");
    assert!(g.game().history().is_empty());
    assert!(
        g.status
            .as_ref()
            .is_some_and(|s| s.text.starts_with("Illegal move"))
    );
}

#[test]
fn selecting_opponent_piece_explains_why() {
    let mut app = local_app();
    goto(&mut app, Square::E7);
    press(&mut app, &[KeyCode::Enter]);
    let g = screen(&app);
    assert_eq!(g.selected, None);
    assert_eq!(g.status.as_ref().unwrap().text, "That's Black's piece");
}

#[test]
fn escape_clears_selection() {
    let mut app = local_app();
    goto(&mut app, Square::G1);
    press(&mut app, &[KeyCode::Enter, KeyCode::Esc]);
    assert_eq!(screen(&app).selected, None);
}

#[test]
fn coordinate_entry_moves_and_letters_do_not_trigger_shortcuts() {
    let mut app = local_app();
    press(&mut app, &[KeyCode::Char('/')]);
    type_text(&mut app, "zz");
    assert!(app.overlay.is_none());
    press(&mut app, &[KeyCode::Enter]);
    let g = screen(&app);
    assert!(g.command.as_ref().unwrap().error.is_some());
    assert!(g.game().history().is_empty());

    press(&mut app, &[KeyCode::Backspace, KeyCode::Backspace]);
    type_text(&mut app, "r");
    assert!(app.overlay.is_none(), "typing r must not open resign");
    press(&mut app, &[KeyCode::Backspace]);
    type_text(&mut app, "g1-f3");
    press(&mut app, &[KeyCode::Enter]);
    let g = screen(&app);
    assert_eq!(g.command, None);
    assert_eq!(g.game().history()[0].san, "Nf3");
}

#[test]
fn coordinate_entry_rejects_illegal_moves_without_mutation() {
    let mut app = local_app();
    press(&mut app, &[KeyCode::Char('/')]);
    type_text(&mut app, "e2e5");
    press(&mut app, &[KeyCode::Enter]);
    let g = screen(&app);
    assert_eq!(
        g.command.as_ref().unwrap().error.as_deref(),
        Some("e2e5 is not legal here")
    );
    assert!(g.game().history().is_empty());
}

#[test]
fn promotion_dialog_offers_all_pieces() {
    let mut app = local_app_with(fixtures::promotion());
    goto(&mut app, Square::B7);
    press(&mut app, &[KeyCode::Enter]);
    goto(&mut app, Square::B8);
    press(&mut app, &[KeyCode::Enter]);
    assert!(matches!(
        app.overlay,
        Some(Overlay::Promotion {
            from: Square::B7,
            to: Square::B8,
            choice: 0
        })
    ));
    press(&mut app, &[KeyCode::Esc]);
    assert!(app.overlay.is_none());
    assert!(
        screen(&app).game().history().len() == 2,
        "cancel sends nothing"
    );
    assert_eq!(
        screen(&app).selected,
        Some(Square::B7),
        "cancel keeps selection"
    );

    goto(&mut app, Square::B8);
    press(&mut app, &[KeyCode::Enter, KeyCode::Char('n')]);
    let g = screen(&app);
    assert_eq!(
        g.game().piece_at(Square::B8).map(|p| p.role),
        Some(Role::Knight)
    );
    assert!(g.game().last_move().unwrap().san.starts_with("b8=N"));
}

#[test]
fn coordinate_promotion_without_suffix_opens_dialog() {
    let mut app = local_app_with(fixtures::promotion());
    press(&mut app, &[KeyCode::Char('/')]);
    type_text(&mut app, "b7b8");
    press(&mut app, &[KeyCode::Enter]);
    assert!(matches!(app.overlay, Some(Overlay::Promotion { .. })));
    press(&mut app, &[KeyCode::Right, KeyCode::Enter]);
    assert_eq!(
        screen(&app).game().piece_at(Square::B8).map(|p| p.role),
        Some(Role::Rook)
    );
}

#[test]
fn resign_requires_confirmation_then_shows_result() {
    let mut app = local_app();
    press(&mut app, &[KeyCode::Char('r')]);
    assert_eq!(app.overlay, Some(Overlay::Confirm(Confirm::Resign)));
    press(&mut app, &[KeyCode::Esc]);
    assert!(!screen(&app).game().is_over());

    press(&mut app, &[KeyCode::Char('r'), KeyCode::Char('y')]);
    assert_eq!(
        screen(&app).game().outcome(),
        Some(Outcome::Win {
            winner: Color::Black,
            reason: WinReason::Resignation
        })
    );
    assert_eq!(app.overlay, Some(Overlay::Result));
    press(&mut app, &[KeyCode::Enter]);
    assert!(app.overlay.is_none());

    goto(&mut app, Square::E2);
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(screen(&app).selected, None, "finished board is read-only");
}

#[test]
fn checkmate_opens_result_overlay() {
    let mut app = local_app_with(fixtures::game(&["f2f3", "e7e5", "g2g4"]));
    press(&mut app, &[KeyCode::Char('/')]);
    type_text(&mut app, "d8h4");
    press(&mut app, &[KeyCode::Enter]);
    assert_eq!(app.overlay, Some(Overlay::Result));
}

#[test]
fn orientation_flips_cursor_directions() {
    let mut app = local_app();
    goto(&mut app, Square::E4);
    press(&mut app, &[KeyCode::Char('o'), KeyCode::Up, KeyCode::Right]);
    let g = screen(&app);
    assert_eq!(g.orientation, Color::Black);
    assert_eq!(g.cursor, Square::D3);
}

#[test]
fn cursor_stops_at_board_edges() {
    let mut app = local_app();
    goto(&mut app, Square::A1);
    press(
        &mut app,
        &[KeyCode::Left, KeyCode::Down, KeyCode::Char('h')],
    );
    assert_eq!(screen(&app).cursor, Square::A1);
}

#[test]
fn tab_cycles_focus_without_clearing_selection() {
    let mut app = local_app();
    goto(&mut app, Square::G1);
    press(&mut app, &[KeyCode::Enter, KeyCode::Tab]);
    let g = screen(&app);
    assert_eq!(g.focus, Focus::History);
    assert_eq!(g.selected, Some(Square::G1));
    press(&mut app, &[KeyCode::Tab, KeyCode::Tab]);
    assert_eq!(screen(&app).focus, Focus::Board);
}

#[test]
fn demo_host_waits_then_starts_and_blocks_out_of_turn_moves() {
    let start = Instant::now();
    let mut app = App::new(fixtures::demo_factory(), start);
    app.start(StartRequest::Host);
    app.tick(start);
    match &app.screen {
        Screen::Waiting(info) => assert_eq!(info.invite, mock::DEMO_INVITE),
        _ => panic!("expected waiting room"),
    }
    app.tick(start + Duration::from_secs(3));
    let g = screen(&app);
    assert_eq!(g.view.seat, Seat::Player(Color::White));

    // White moves, then it's the simulated opponent's turn.
    press(&mut app, &[KeyCode::Char('/')]);
    type_text(&mut app, "e2e4");
    app.on_key(key(KeyCode::Enter));
    assert!(
        screen(&app).pending.is_some(),
        "pending until host confirms"
    );
    app.tick(Instant::now() + Duration::from_millis(400));
    let g = screen(&app);
    assert_eq!(g.pending, None);
    assert_eq!(g.game().turn(), Color::Black);

    goto(&mut app, Square::D2);
    app.on_key(key(KeyCode::Enter));
    let g = screen(&app);
    assert_eq!(g.selected, None);
    assert_eq!(g.status.as_ref().unwrap().text, "Waiting for Black's move");

    app.tick(Instant::now() + Duration::from_secs(2));
    assert_eq!(screen(&app).game().history().len(), 2, "opponent replied");
}

#[test]
fn demo_drop_pauses_moves_then_recovers() {
    let start = Instant::now();
    let mut app = App::new(fixtures::demo_factory(), start);
    app.demo = true;
    app.start(StartRequest::Join {
        addr: "192.168.1.20:4747".into(),
        code: "K7QF-M2XA".into(),
        fingerprint: None,
    });
    app.tick(start + Duration::from_secs(2));
    app.on_key(key(KeyCode::Char('x')));
    app.tick(Instant::now());
    assert!(matches!(screen(&app).view.link, Link::Reconnecting { .. }));
    app.tick(Instant::now() + Duration::from_secs(10));
    let g = screen(&app);
    assert!(matches!(g.view.link, Link::Live { .. }));
    assert_eq!(g.status.as_ref().unwrap().text, "Back online");
}

#[test]
fn join_form_validates_before_connecting() {
    let mut app = App::new(fixtures::demo_factory(), Instant::now());
    press(&mut app, &[KeyCode::Down, KeyCode::Enter]);
    assert!(matches!(app.screen, Screen::Join(_)));
    type_text(&mut app, "10.0.0.7");
    press(&mut app, &[KeyCode::Enter]);
    type_text(&mut app, "abc");
    press(&mut app, &[KeyCode::Enter]);
    match &app.screen {
        Screen::Join(form) => assert_eq!(form.error.as_ref().unwrap().0, 1),
        _ => panic!("should stay on form"),
    }
    type_text(&mut app, "d-efgh");
    press(&mut app, &[KeyCode::Enter]);
    assert!(matches!(app.screen, Screen::Connecting { ref addr } if addr == "10.0.0.7:4747"));
}

#[test]
fn join_form_fits_the_full_invite_and_host_key() {
    let mut app = App::new(fixtures::demo_factory(), Instant::now());
    press(&mut app, &[KeyCode::Down, KeyCode::Enter]);
    type_text(&mut app, "192.168.1.20:4747");
    press(&mut app, &[KeyCode::Tab]);
    type_text(&mut app, "K7QF-M2XA ");
    type_text(&mut app, "A1B2C3D4-E5F60718");
    match &app.screen {
        Screen::Join(form) => {
            assert_eq!(form.code, "K7QF-M2XA");
            assert_eq!(form.fingerprint, "A1B2C3D4-E5F60718");
            assert_eq!(form.field, 2);
        }
        _ => panic!("still on the join form"),
    }
    press(&mut app, &[KeyCode::Enter]);
    assert!(matches!(app.screen, Screen::Connecting { .. }));
}

#[test]
fn join_form_splits_the_line_the_host_shows() {
    let mut app = App::new(fixtures::demo_factory(), Instant::now());
    press(&mut app, &[KeyCode::Down, KeyCode::Enter]);
    type_text(
        &mut app,
        "vesper join 192.168.1.20:4747 K7QF-M2XA A1B2C3D4-E5F60718",
    );
    match &app.screen {
        Screen::Join(form) => {
            assert_eq!(form.address, "192.168.1.20:4747");
            assert_eq!(form.code, "K7QF-M2XA");
            assert_eq!(form.fingerprint, "A1B2C3D4-E5F60718");
        }
        _ => panic!("still on the join form"),
    }
}

#[test]
fn join_address_and_invite_normalization() {
    assert_eq!(
        validate_join("192.168.1.20", "k7qf m2xa"),
        Ok(("192.168.1.20:4747".into(), "K7QF-M2XA".into()))
    );
    assert_eq!(
        validate_join("host.local:5000", "K7QF-M2XA").unwrap().0,
        "host.local:5000"
    );
    assert!(validate_join("", "K7QF-M2XA").is_err());
    assert!(validate_join("1.2.3.4:0", "K7QF-M2XA").is_err());
    assert!(validate_join("1.2.3.4:70000", "K7QF-M2XA").is_err());
    assert_eq!(normalize_invite("ol1i-abcd"), Some("0111-ABCD".into()));
    assert_eq!(normalize_invite("UUUU-AAAA"), None, "U is not Crockford");
    assert_eq!(normalize_invite("short"), None);
}

#[test]
fn ctrl_c_always_quits() {
    let mut app = local_app();
    press(&mut app, &[KeyCode::Char('?')]);
    app.on_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(app.quit);
}
