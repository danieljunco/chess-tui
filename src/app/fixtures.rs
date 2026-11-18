//! Coherent sample games and named UI scenes for the visual prototype,
//! snapshot rendering and tests. Every board is derived from real moves.

use std::time::Instant;

use super::mock::{DEMO_ADDR, MockMode, MockProvider, demo_waiting};
use super::*;
use crate::domain::{Game, Square};

/// 1.e4 e5 2.Nf3 Nc6 3.Bc4 Nf6 4.d4 exd4 5.O-O Nxe4 6.Re1 d5 7.Bxd5 Qxd5
pub const ITALIAN: [&str; 14] = [
    "e2e4", "e7e5", "g1f3", "b8c6", "f1c4", "g8f6", "d2d4", "e5d4", "e1g1", "f6e4", "f1e1", "d7d5",
    "c4d5", "d8d5",
];

pub fn game(moves: &[&str]) -> Game {
    let mut g = Game::new();
    for m in moves {
        g.play_uci(m).expect("fixture moves are legal");
    }
    g
}

pub fn italian() -> Game {
    game(&ITALIAN)
}

/// 1.e4 f6 2.Qh5+ — Black to move, in check.
pub fn check() -> Game {
    game(&["e2e4", "f7f6", "d1h5"])
}

pub fn promotion() -> Game {
    let mut g = Game::from_fen("3r2k1/1P3ppp/8/8/8/2N5/5PPP/6K1 w - - 0 1").expect("valid");
    g.play_uci("g1f1").expect("legal");
    g.play_uci("g8f8").expect("legal");
    g
}

/// Scholar's mate: 1.e4 e5 2.Bc4 Nc6 3.Qh5 Nf6 4.Qxf7#
pub fn scholars_mate() -> Game {
    game(&["e2e4", "e7e5", "f1c4", "b8c6", "d1h5", "g8f6", "h5f7"])
}

pub fn view(game: Game, seat: Seat, table: Table, link: Link) -> GameView {
    let (white, black) = match seat {
        Seat::Both => ("White", "Black"),
        Seat::Player(Color::White) => ("You", "Ben"),
        Seat::Player(Color::Black) => ("Ana", "You"),
    };
    GameView {
        version: game.history().len() as u64 + 1,
        game,
        seat,
        white: white.into(),
        black: black.into(),
        table,
        link,
    }
}

pub fn hosting() -> Table {
    Table::Hosting {
        addr: DEMO_ADDR.into(),
    }
}

pub fn joined() -> Table {
    Table::Joined {
        addr: DEMO_ADDR.into(),
    }
}

pub fn live() -> Link {
    Link::Live { rtt_ms: Some(24) }
}

/// Factory used by `vesper demo`: every table is backed by the mock provider.
pub fn demo_factory() -> ProviderFactory {
    Box::new(|request, now| {
        let provider = match request {
            StartRequest::Host => MockProvider::new(MockMode::DemoHost, now),
            StartRequest::Join { addr, .. } => {
                MockProvider::new(MockMode::DemoJoin, now).with_addr(addr)
            }
            StartRequest::Local => MockProvider::new(MockMode::Local, now),
        };
        Box::new(DemoBoot::new(provider))
    })
}

/// Emits the mock's initial waiting/connecting event on first poll.
struct DemoBoot {
    inner: MockProvider,
    booted: bool,
}

impl DemoBoot {
    fn new(inner: MockProvider) -> Self {
        Self {
            inner,
            booted: false,
        }
    }
}

impl GameProvider for DemoBoot {
    fn send(&mut self, command: GameCommand) {
        self.inner.send(command);
    }

    fn poll(&mut self, now: Instant) -> Vec<ProviderEvent> {
        let mut out = Vec::new();
        if !self.booted {
            self.booted = true;
            out.extend(self.inner.initial_event());
        }
        out.extend(self.inner.poll(now));
        out
    }

    fn simulate_drop(&mut self, now: Instant) {
        self.inner.simulate_drop(now);
    }
}

fn app(now: Instant) -> App {
    let mut app = App::new(demo_factory(), now);
    app.demo = true;
    app
}

fn game_app(now: Instant, view: GameView, edit: impl FnOnce(&mut GameScreen)) -> App {
    let mut a = app(now);
    let mut screen = GameScreen::new(view);
    edit(&mut screen);
    let over = screen.game().is_over() && !screen.result_seen;
    a.screen = Screen::Game(Box::new(screen));
    if over {
        a.overlay = Some(Overlay::Result);
    }
    a
}

/// Named scenes covering every primary state (visual gate 1).
pub fn scenes(now: Instant) -> Vec<(&'static str, App)> {
    let mut out = Vec::new();

    out.push(("home", app(now)));

    let mut a = app(now);
    a.screen = Screen::Join(JoinForm {
        address: "192.168.1.20".into(),
        code: "k7qf".into(),
        field: 1,
        error: None,
    });
    out.push(("join-form", a));

    let mut a = app(now);
    a.screen = Screen::Join(JoinForm {
        address: "192.168.1.20:99999".into(),
        code: "K7QF-M2XA".into(),
        field: 0,
        error: Some((0, "Port must be a number between 1 and 65535".into())),
    });
    out.push(("join-error", a));

    let mut a = app(now);
    a.screen = Screen::Waiting(demo_waiting());
    out.push(("waiting", a));

    let mut a = app(now);
    a.screen = Screen::Connecting {
        addr: DEMO_ADDR.into(),
    };
    out.push(("connecting", a));

    out.push((
        "game-your-move",
        game_app(
            now,
            view(italian(), Seat::Player(Color::White), hosting(), live()),
            |g| {
                g.cursor = Square::B1;
                g.selected = Some(Square::B1);
                g.targets = g.game().legal_targets(Square::B1);
            },
        ),
    ));

    out.push((
        "game-cursor-capture",
        game_app(
            now,
            view(italian(), Seat::Player(Color::White), hosting(), live()),
            |g| {
                g.selected = Some(Square::E1);
                g.targets = g.game().legal_targets(Square::E1);
                g.cursor = Square::E4;
                g.focus = Focus::Board;
            },
        ),
    ));

    let mut waiting_opponent = italian();
    waiting_opponent.play_uci("b1c3").expect("legal");
    out.push((
        "game-opponent-turn",
        game_app(
            now,
            view(
                waiting_opponent.clone(),
                Seat::Player(Color::White),
                hosting(),
                live(),
            ),
            |g| {
                g.focus = Focus::History;
            },
        ),
    ));

    out.push((
        "game-black-check",
        game_app(
            now,
            view(check(), Seat::Player(Color::Black), joined(), live()),
            |g| {
                g.cursor = Square::G7;
                g.selected = Some(Square::G7);
                g.targets = g.game().legal_targets(Square::G7);
            },
        ),
    ));

    out.push((
        "game-sending",
        game_app(
            now,
            view(italian(), Seat::Player(Color::White), hosting(), live()),
            |g| {
                g.cursor = Square::C3;
                g.pending = Some("Sending move b1 → c3…".into());
            },
        ),
    ));

    out.push((
        "game-command",
        game_app(
            now,
            view(italian(), Seat::Player(Color::White), hosting(), live()),
            |g| {
                g.command = Some(CommandInput {
                    text: "b1c3".into(),
                    error: None,
                });
            },
        ),
    ));

    out.push((
        "game-rejected",
        game_app(
            now,
            view(italian(), Seat::Player(Color::White), hosting(), live()),
            |g| {
                g.cursor = Square::C4;
                g.selected = Some(Square::B1);
                g.targets = g.game().legal_targets(Square::B1);
                g.status = Some(Status {
                    text: "Illegal move · choose a highlighted square".into(),
                    tone: Tone::Danger,
                    until: None,
                });
            },
        ),
    ));

    let mut a = game_app(
        now,
        view(promotion(), Seat::Player(Color::White), hosting(), live()),
        |g| {
            g.cursor = Square::B8;
        },
    );
    a.overlay = Some(Overlay::Promotion {
        from: Square::B7,
        to: Square::B8,
        choice: 0,
    });
    out.push(("promotion", a));

    out.push((
        "reconnecting",
        game_app(
            now,
            view(
                waiting_opponent.clone(),
                Seat::Player(Color::Black),
                joined(),
                Link::Reconnecting { attempt: 3 },
            ),
            |g| {
                g.status = Some(Status {
                    text: "Connection lost · reconnecting · last synced move 8. Nc3".into(),
                    tone: Tone::Warning,
                    until: None,
                });
            },
        ),
    ));

    out.push((
        "opponent-offline",
        game_app(
            now,
            view(
                italian(),
                Seat::Player(Color::White),
                hosting(),
                Link::OpponentOffline,
            ),
            |_| {},
        ),
    ));

    let mut a = game_app(
        now,
        view(italian(), Seat::Player(Color::White), hosting(), live()),
        |_| {},
    );
    a.overlay = Some(Overlay::Confirm(Confirm::Resign));
    out.push(("confirm-resign", a));

    let mut a = game_app(
        now,
        view(italian(), Seat::Player(Color::White), hosting(), live()),
        |_| {},
    );
    a.overlay = Some(Overlay::Help);
    out.push(("help", a));

    out.push((
        "result-checkmate",
        game_app(
            now,
            view(
                scholars_mate(),
                Seat::Player(Color::Black),
                joined(),
                live(),
            ),
            |_| {},
        ),
    ));

    out.push((
        "result-reviewed",
        game_app(
            now,
            view(
                scholars_mate(),
                Seat::Player(Color::White),
                hosting(),
                live(),
            ),
            |g| g.result_seen = true,
        ),
    ));

    out.push((
        "local",
        game_app(
            now,
            view(italian(), Seat::Both, Table::Local, Link::Local),
            |_| {},
        ),
    ));

    let mut a = game_app(
        now,
        view(italian(), Seat::Player(Color::White), hosting(), live()),
        |g| {
            g.selected = Some(Square::B1);
            g.targets = g.game().legal_targets(Square::B1);
        },
    );
    a.ascii = true;
    out.push(("ascii", a));

    out
}
