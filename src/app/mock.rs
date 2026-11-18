//! In-process providers: pass-and-play (`Local`) and the fake network
//! provider used by the visual prototype (`DemoHost`, `DemoJoin`).
//! The demo opponent is a deterministic random mover, not an engine.

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use super::provider::*;
use crate::domain::{Color, Game, Role, Square};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MockMode {
    Local,
    DemoHost,
    DemoJoin,
}

const LATENCY: Duration = Duration::from_millis(180);
const BOT_THINK: Duration = Duration::from_millis(900);
const JOIN_DELAY: Duration = Duration::from_millis(2600);
const CONNECT_DELAY: Duration = Duration::from_millis(1200);
const DROP_FOR: Duration = Duration::from_millis(3500);
pub const DEMO_ADDR: &str = "192.168.1.20:4747";
pub const DEMO_INVITE: &str = "K7QF-M2XA";

pub struct MockProvider {
    mode: MockMode,
    game: Game,
    seat: Seat,
    link: Link,
    version: u64,
    started: bool,
    dirty: bool,
    addr: String,
    queue: VecDeque<(Instant, Scheduled)>,
    rng: u64,
}

enum Scheduled {
    Start,
    Command(GameCommand),
    BotMove,
    Reconnected,
}

impl MockProvider {
    pub fn new(mode: MockMode, now: Instant) -> Self {
        let seat = match mode {
            MockMode::Local => Seat::Both,
            MockMode::DemoHost => Seat::Player(Color::White),
            MockMode::DemoJoin => Seat::Player(Color::Black),
        };
        let link = match mode {
            MockMode::Local => Link::Local,
            _ => Link::Live { rtt_ms: Some(24) },
        };
        let mut provider = Self {
            mode,
            game: Game::new(),
            seat,
            link,
            version: 1,
            started: false,
            dirty: false,
            addr: DEMO_ADDR.into(),
            queue: VecDeque::new(),
            rng: 0x9E37_79B9_7F4A_7C15,
        };
        let delay = match mode {
            MockMode::Local => Duration::ZERO,
            MockMode::DemoHost => JOIN_DELAY,
            MockMode::DemoJoin => CONNECT_DELAY,
        };
        provider.schedule(now + delay, Scheduled::Start);
        provider
    }

    /// Start from an existing game (fixtures, tests).
    pub fn with_game(mut self, game: Game) -> Self {
        self.game = game;
        self
    }

    /// Address shown for the simulated table.
    pub fn with_addr(mut self, addr: impl Into<String>) -> Self {
        self.addr = addr.into();
        self
    }

    pub fn initial_event(&self) -> Option<ProviderEvent> {
        match self.mode {
            MockMode::Local => None,
            MockMode::DemoHost => Some(ProviderEvent::Waiting(demo_waiting())),
            MockMode::DemoJoin => Some(ProviderEvent::Connecting {
                addr: self.addr.clone(),
            }),
        }
    }

    fn schedule(&mut self, at: Instant, item: Scheduled) {
        let idx = self.queue.iter().position(|(t, _)| *t > at);
        match idx {
            Some(i) => self.queue.insert(i, (at, item)),
            None => self.queue.push_back((at, item)),
        }
    }

    fn view(&self) -> ProviderEvent {
        let (white, black) = match self.mode {
            MockMode::Local => ("White", "Black"),
            MockMode::DemoHost => ("You", "Ben"),
            MockMode::DemoJoin => ("Ana", "You"),
        };
        let table = match self.mode {
            MockMode::Local => Table::Local,
            MockMode::DemoHost => Table::Hosting {
                addr: self.addr.clone(),
            },
            MockMode::DemoJoin => Table::Joined {
                addr: self.addr.clone(),
            },
        };
        ProviderEvent::View(Box::new(GameView {
            game: self.game.clone(),
            seat: self.seat,
            white: white.into(),
            black: black.into(),
            table,
            link: self.link,
            version: self.version,
        }))
    }

    fn bot_color(&self) -> Option<Color> {
        match self.seat {
            Seat::Both => None,
            Seat::Player(c) => Some(!c),
        }
    }

    fn maybe_schedule_bot(&mut self, now: Instant) {
        if !self.game.is_over() && self.bot_color() == Some(self.game.turn()) {
            self.schedule(now + BOT_THINK, Scheduled::BotMove);
        }
    }

    fn apply(&mut self, command: GameCommand, out: &mut Vec<ProviderEvent>) {
        let mover = match self.seat {
            Seat::Both => self.game.turn(),
            Seat::Player(c) => c,
        };
        let result = match command {
            GameCommand::Move {
                from,
                to,
                promotion,
            } => {
                if !self.seat.controls(self.game.turn()) {
                    Err("Not your turn".to_string())
                } else {
                    self.game
                        .play(from, to, promotion)
                        .map(|_| ())
                        .map_err(|e| e.to_string())
                }
            }
            GameCommand::Resign => self
                .game
                .resign(mover)
                .map(|_| ())
                .map_err(|e| e.to_string()),
            GameCommand::ClaimDraw => self
                .game
                .claim_draw(self.game.turn())
                .map(|_| ())
                .map_err(|e| e.to_string()),
            GameCommand::Leave => Ok(()),
        };
        match result {
            Ok(()) => {
                self.version += 1;
                out.push(self.view());
            }
            Err(reason) => out.push(ProviderEvent::Rejected { reason }),
        }
    }

    fn bot_move(&mut self) -> Option<(Square, Square, Option<Role>)> {
        let turn = self.game.turn();
        let mut moves = Vec::new();
        for sq in Square::ALL {
            if self.game.piece_at(sq).is_some_and(|p| p.color == turn) {
                for to in self.game.legal_targets(sq) {
                    let promo = self.game.needs_promotion(sq, to).then_some(Role::Queen);
                    let capture = self.game.piece_at(to).is_some();
                    moves.push((sq, to, promo, capture));
                }
            }
        }
        let captures: Vec<_> = moves.iter().filter(|m| m.3).cloned().collect();
        let pool = if !captures.is_empty() && !self.next_rand().is_multiple_of(3) {
            captures
        } else {
            moves
        };
        if pool.is_empty() {
            return None;
        }
        let pick = pool[(self.next_rand() as usize) % pool.len()];
        Some((pick.0, pick.1, pick.2))
    }

    fn next_rand(&mut self) -> u64 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 7;
        self.rng ^= self.rng << 17;
        self.rng
    }
}

impl GameProvider for MockProvider {
    fn send(&mut self, command: GameCommand) {
        let now = Instant::now();
        let delay = if self.mode == MockMode::Local {
            Duration::ZERO
        } else {
            LATENCY
        };
        self.schedule(now + delay, Scheduled::Command(command));
    }

    fn poll(&mut self, now: Instant) -> Vec<ProviderEvent> {
        let mut out = Vec::new();
        if std::mem::take(&mut self.dirty) {
            out.push(self.view());
        }
        while self.queue.front().is_some_and(|(t, _)| *t <= now) {
            let (_, item) = self.queue.pop_front().expect("checked");
            match item {
                Scheduled::Start => {
                    self.started = true;
                    out.push(self.view());
                    self.maybe_schedule_bot(now);
                }
                Scheduled::Command(cmd) if self.link.can_play() => {
                    self.apply(cmd, &mut out);
                    self.maybe_schedule_bot(now);
                }
                Scheduled::Command(_) => out.push(ProviderEvent::Rejected {
                    reason: "Connection lost · move not sent".into(),
                }),
                Scheduled::BotMove => {
                    if !self.link.can_play() {
                        self.schedule(now + BOT_THINK, Scheduled::BotMove);
                    } else if let Some((from, to, promotion)) = self.bot_move() {
                        let _ = self.game.play(from, to, promotion);
                        self.version += 1;
                        out.push(self.view());
                    }
                }
                Scheduled::Reconnected => {
                    self.link = Link::Live { rtt_ms: Some(31) };
                    out.push(self.view());
                }
            }
        }
        out
    }

    fn simulate_drop(&mut self, now: Instant) {
        if self.mode == MockMode::Local || !self.started {
            return;
        }
        self.link = match self.mode {
            MockMode::DemoHost => Link::OpponentOffline,
            _ => Link::Reconnecting { attempt: 2 },
        };
        self.dirty = true;
        self.schedule(now + DROP_FOR, Scheduled::Reconnected);
    }
}

pub fn demo_waiting() -> WaitingInfo {
    WaitingInfo {
        addresses: vec![DEMO_ADDR.split(':').next().unwrap_or_default().into()],
        port: 4747,
        invite: DEMO_INVITE.into(),
        seat: Color::White,
    }
}
