//! The seam between the UI state machine and whatever runs the game
//! (mock, local, host or joining peer). The UI only sees `GameView`s.

use std::time::Instant;

use crate::domain::{Color, Game, Role, Square};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameCommand {
    Move {
        from: Square,
        to: Square,
        promotion: Option<Role>,
    },
    Resign,
    ClaimDraw,
    /// Leave the table (closing the listener / connection).
    Leave,
}

/// Which seat(s) this terminal controls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seat {
    /// Pass-and-play: whoever's turn it is.
    Both,
    Player(Color),
}

impl Seat {
    pub fn controls(self, color: Color) -> bool {
        match self {
            Seat::Both => true,
            Seat::Player(c) => c == color,
        }
    }
}

/// Where the game runs, from this terminal's point of view.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Table {
    Local,
    Hosting { addr: String },
    Joined { addr: String },
}

/// Connection facts derived locally (never a game mutation).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Link {
    Local,
    Live { rtt_ms: Option<u32> },
    Reconnecting { attempt: u32 },
    OpponentOffline,
    HostClosed,
}

impl Link {
    pub fn can_play(self) -> bool {
        matches!(
            self,
            Link::Local | Link::Live { .. } | Link::OpponentOffline
        )
    }
}

#[derive(Clone, Debug)]
pub struct GameView {
    pub game: Game,
    pub seat: Seat,
    pub white: String,
    pub black: String,
    pub table: Table,
    pub link: Link,
    pub version: u64,
}

impl GameView {
    pub fn name(&self, color: Color) -> &str {
        match color {
            Color::White => &self.white,
            Color::Black => &self.black,
        }
    }

    pub fn my_turn(&self) -> bool {
        !self.game.is_over() && self.seat.controls(self.game.turn())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WaitingInfo {
    pub addresses: Vec<String>,
    pub port: u16,
    pub invite: String,
    pub seat: Color,
    /// Noise static-key fingerprint. Empty until the host has generated one.
    pub fingerprint: String,
    /// Address seen by a public STUN server, when it differs from the LAN address.
    pub public_ip: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ProviderEvent {
    Waiting(WaitingInfo),
    Connecting { addr: String },
    View(Box<GameView>),
    Rejected { reason: String },
    Failed { reason: String },
}

pub trait GameProvider {
    /// Must never block; real providers forward to an async task.
    fn send(&mut self, command: GameCommand);
    /// Drain events that are ready at `now`.
    fn poll(&mut self, now: Instant) -> Vec<ProviderEvent>;
    /// Demo-only hook to preview the reconnecting state.
    fn simulate_drop(&mut self, _now: Instant) {}
}
