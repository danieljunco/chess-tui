//! Wire frames for the direct host ⇄ peer connection. One JSON object per line.
//! See `docs/architecture/02-realtime-protocol.md`.

use serde::{Deserialize, Serialize};

use crate::domain::{Color, DrawReason, Game, Outcome, WinReason};

pub const PROTOCOL_VERSION: u32 = 1;
pub const MAX_FRAME: usize = 64 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireColor {
    White,
    Black,
}

impl From<Color> for WireColor {
    fn from(color: Color) -> Self {
        match color {
            Color::White => Self::White,
            Color::Black => Self::Black,
        }
    }
}

impl From<WireColor> for Color {
    fn from(color: WireColor) -> Self {
        match color {
            WireColor::White => Color::White,
            WireColor::Black => Color::Black,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    Active,
    Finished,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RefuseCode {
    BadVersion,
    BadInvite,
    BadToken,
    SeatTaken,
    GameOver,
    Malformed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectCode {
    NotYourTurn,
    IllegalMove,
    StaleVersion,
    GameOver,
    NotClaimable,
    Malformed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WireReason {
    Checkmate,
    Resignation,
    Stalemate,
    InsufficientMaterial,
    FivefoldRepetition,
    SeventyFiveMoves,
    ThreefoldRepetition,
    FiftyMoves,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WireOutcome {
    pub result: String,
    pub reason: WireReason,
}

impl WireOutcome {
    pub fn from_domain(outcome: Outcome) -> Self {
        let (result, reason) = match outcome {
            Outcome::Win {
                winner: Color::White,
                reason,
            } => ("1-0", win_reason(reason)),
            Outcome::Win {
                winner: Color::Black,
                reason,
            } => ("0-1", win_reason(reason)),
            Outcome::Draw(reason) => ("1/2-1/2", draw_reason(reason)),
        };
        Self {
            result: result.to_string(),
            reason,
        }
    }

    pub fn to_domain(&self) -> Option<Outcome> {
        let winner = match self.result.as_str() {
            "1-0" => Some(Color::White),
            "0-1" => Some(Color::Black),
            "1/2-1/2" => None,
            _ => return None,
        };
        match (self.reason, winner) {
            (WireReason::Checkmate, Some(winner)) => Some(Outcome::Win {
                winner,
                reason: WinReason::Checkmate,
            }),
            (WireReason::Resignation, Some(winner)) => Some(Outcome::Win {
                winner,
                reason: WinReason::Resignation,
            }),
            (WireReason::Stalemate, None) => Some(Outcome::Draw(DrawReason::Stalemate)),
            (WireReason::InsufficientMaterial, None) => {
                Some(Outcome::Draw(DrawReason::InsufficientMaterial))
            }
            (WireReason::FivefoldRepetition, None) => {
                Some(Outcome::Draw(DrawReason::FivefoldRepetition))
            }
            (WireReason::SeventyFiveMoves, None) => {
                Some(Outcome::Draw(DrawReason::SeventyFiveMoves))
            }
            (WireReason::ThreefoldRepetition, None) => {
                Some(Outcome::Draw(DrawReason::ThreefoldRepetition))
            }
            (WireReason::FiftyMoves, None) => Some(Outcome::Draw(DrawReason::FiftyMoves)),
            _ => None,
        }
    }
}

fn win_reason(reason: WinReason) -> WireReason {
    match reason {
        WinReason::Checkmate => WireReason::Checkmate,
        WinReason::Resignation => WireReason::Resignation,
    }
}

fn draw_reason(reason: DrawReason) -> WireReason {
    match reason {
        DrawReason::Stalemate => WireReason::Stalemate,
        DrawReason::InsufficientMaterial => WireReason::InsufficientMaterial,
        DrawReason::FivefoldRepetition => WireReason::FivefoldRepetition,
        DrawReason::SeventyFiveMoves => WireReason::SeventyFiveMoves,
        DrawReason::ThreefoldRepetition => WireReason::ThreefoldRepetition,
        DrawReason::FiftyMoves => WireReason::FiftyMoves,
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Snapshot {
    pub version: u64,
    pub phase: Phase,
    pub white: String,
    pub black: String,
    pub start_fen: String,
    pub moves: Vec<String>,
    pub fen: String,
    pub outcome: Option<WireOutcome>,
    pub opponent_online: bool,
}

impl Snapshot {
    /// Rebuild the position from the host's move list and check it matches.
    pub fn replay(&self) -> Option<Game> {
        let mut game = Game::replay(&self.start_fen, &self.moves).ok()?;
        if game.fen() != self.fen {
            return None;
        }
        let declared = match self.outcome.as_ref() {
            Some(outcome) => Some(outcome.to_domain()?),
            None => None,
        };
        game.reconcile_outcome(declared).ok()?;
        Some(game)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
pub enum Frame {
    Hello {
        protocol_version: u32,
        invite_code: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
    Resume {
        protocol_version: u32,
        seat_token: String,
    },
    Welcome {
        seat: WireColor,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        seat_token: Option<String>,
        host_name: String,
    },
    Refused {
        code: RefuseCode,
        message: String,
    },
    Move {
        request_id: u64,
        expected_version: u64,
        uci: String,
    },
    Resign {
        request_id: u64,
        expected_version: u64,
    },
    ClaimDraw {
        request_id: u64,
        expected_version: u64,
    },
    Sync,
    Ping {
        nonce: u64,
    },
    Pong {
        nonce: u64,
    },
    Bye,
    Accepted {
        request_id: u64,
        version: u64,
    },
    Rejected {
        request_id: u64,
        code: RejectCode,
        message: String,
        version: u64,
    },
    Snapshot {
        version: u64,
        phase: Phase,
        white: String,
        black: String,
        start_fen: String,
        moves: Vec<String>,
        fen: String,
        outcome: Option<WireOutcome>,
        opponent_online: bool,
    },
    HostClosing {
        reason: String,
    },
}

impl Frame {
    pub fn snapshot(snap: Snapshot) -> Self {
        Self::Snapshot {
            version: snap.version,
            phase: snap.phase,
            white: snap.white,
            black: snap.black,
            start_fen: snap.start_fen,
            moves: snap.moves,
            fen: snap.fen,
            outcome: snap.outcome,
            opponent_online: snap.opponent_online,
        }
    }

    pub fn as_snapshot(&self) -> Option<Snapshot> {
        match self {
            Self::Snapshot {
                version,
                phase,
                white,
                black,
                start_fen,
                moves,
                fen,
                outcome,
                opponent_online,
            } => Some(Snapshot {
                version: *version,
                phase: *phase,
                white: white.clone(),
                black: black.clone(),
                start_fen: start_fen.clone(),
                moves: moves.clone(),
                fen: fen.clone(),
                outcome: outcome.clone(),
                opponent_online: *opponent_online,
            }),
            _ => None,
        }
    }
}

pub fn to_line(frame: &Frame) -> String {
    serde_json::to_string(frame).expect("frame serializes")
}

pub fn from_line(line: &str) -> Option<Frame> {
    serde_json::from_str(line).ok()
}

pub fn refuse(code: RefuseCode, message: impl Into<String>) -> Frame {
    Frame::Refused {
        code,
        message: message.into(),
    }
}

impl RefuseCode {
    pub fn message(self) -> &'static str {
        match self {
            Self::BadVersion => "This Vesper can't speak to that host",
            Self::BadInvite => "Invite code not accepted",
            Self::BadToken => "Reconnect secret not accepted",
            Self::SeatTaken => "That seat is already taken",
            Self::GameOver => "The game is already over",
            Self::Malformed => "Could not read that message",
        }
    }
}
