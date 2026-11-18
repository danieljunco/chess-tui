//! The authoritative game. It lives in the hosting process and is the only
//! place a multiplayer move is committed. The host's own seat goes through
//! `apply`, same as the remote peer.

use std::collections::VecDeque;

use crate::domain::{Color, Game, Outcome};
use crate::identity::{clean_name, ct_eq, new_token, normalize_invite};

const CACHE: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Host,
    Peer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Waiting,
    Active,
    Finished,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Intent {
    Move(String),
    Resign,
    ClaimDraw,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fail {
    NotYourTurn,
    IllegalMove,
    StaleVersion,
    GameOver,
    NotClaimable,
    Malformed,
    NotStarted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Applied {
    pub request_id: u64,
    pub accepted: bool,
    pub version: u64,
    pub fail: Option<Fail>,
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JoinError {
    BadInvite,
    BadToken,
    SeatTaken,
    GameOver,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Joined {
    pub seat: Color,
    pub token: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Authoritative {
    pub version: u64,
    pub phase: Phase,
    pub white: String,
    pub black: String,
    pub start_fen: String,
    pub moves: Vec<String>,
    pub fen: String,
    pub outcome: Option<Outcome>,
}

pub struct Session {
    game: Game,
    phase: Phase,
    version: u64,
    host_color: Color,
    host_name: String,
    white: String,
    black: String,
    invite: String,
    invite_open: bool,
    token: Option<String>,
    cache: VecDeque<(Side, u64, Applied)>,
}

impl Session {
    pub fn new(host_color: Color, host_name: &str, invite: String) -> Self {
        let host_name = clean_name(host_name, "You");
        let (white, black) = match host_color {
            Color::White => (host_name.clone(), "Guest".to_string()),
            Color::Black => ("Guest".to_string(), host_name.clone()),
        };
        Self {
            game: Game::new(),
            phase: Phase::Waiting,
            version: 0,
            host_color,
            host_name,
            white,
            black,
            invite,
            invite_open: true,
            token: None,
            cache: VecDeque::new(),
        }
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }

    pub fn host_color(&self) -> Color {
        self.host_color
    }

    pub fn host_name(&self) -> &str {
        &self.host_name
    }

    pub fn game(&self) -> &Game {
        &self.game
    }

    pub fn white(&self) -> &str {
        &self.white
    }

    pub fn black(&self) -> &str {
        &self.black
    }

    pub fn invite(&self) -> &str {
        &self.invite
    }

    /// Admit the peer once. A correct code after that is `SeatTaken`.
    pub fn join(&mut self, code: &str, peer_name: &str) -> Result<Joined, JoinError> {
        let Some(code) = normalize_invite(code) else {
            return Err(JoinError::BadInvite);
        };
        if !ct_eq(&code, &self.invite) {
            return Err(JoinError::BadInvite);
        }
        if !self.invite_open {
            return Err(if self.game.is_over() {
                JoinError::GameOver
            } else {
                JoinError::SeatTaken
            });
        }
        let peer_name = clean_name(peer_name, "Guest");
        match self.host_color {
            Color::White => self.black = peer_name,
            Color::Black => self.white = peer_name,
        }
        let token = new_token();
        self.token = Some(token.clone());
        self.invite_open = false;
        self.phase = Phase::Active;
        self.version = 1;
        Ok(Joined {
            seat: !self.host_color,
            token,
        })
    }

    pub fn resume(&self, token: &str) -> Result<Color, JoinError> {
        match &self.token {
            Some(expected) if ct_eq(expected, token.trim()) => Ok(!self.host_color),
            _ => Err(JoinError::BadToken),
        }
    }

    pub fn apply(
        &mut self,
        side: Side,
        request_id: u64,
        expected_version: u64,
        intent: Intent,
    ) -> Applied {
        if let Some((_, _, cached)) = self
            .cache
            .iter()
            .find(|(cached_side, id, _)| *cached_side == side && *id == request_id)
        {
            return cached.clone();
        }
        let result = self.apply_fresh(side, request_id, expected_version, intent);
        self.cache.push_back((side, request_id, result.clone()));
        while self.cache.len() > CACHE {
            self.cache.pop_front();
        }
        result
    }

    pub fn authoritative(&self) -> Authoritative {
        Authoritative {
            version: self.version,
            phase: self.phase,
            white: self.white.clone(),
            black: self.black.clone(),
            start_fen: self.game.start_fen().to_string(),
            moves: self.game.history().iter().map(|m| m.uci.clone()).collect(),
            fen: self.game.fen(),
            outcome: self.game.outcome(),
        }
    }

    fn apply_fresh(
        &mut self,
        side: Side,
        request_id: u64,
        expected_version: u64,
        intent: Intent,
    ) -> Applied {
        if self.phase == Phase::Waiting {
            return fail(
                request_id,
                self.version,
                Fail::NotStarted,
                "Waiting for an opponent",
            );
        }
        if self.game.is_over() {
            return fail(request_id, self.version, Fail::GameOver, "The game is over");
        }
        if expected_version != self.version {
            return fail(
                request_id,
                self.version,
                Fail::StaleVersion,
                "The board moved · synced to the latest position",
            );
        }
        let color = match side {
            Side::Host => self.host_color,
            Side::Peer => !self.host_color,
        };
        let played = match intent {
            Intent::Move(uci) => {
                if self.game.turn() != color {
                    return fail(request_id, self.version, Fail::NotYourTurn, "Not your turn");
                }
                match self.game.play_uci(&uci) {
                    Ok(_) => true,
                    Err(crate::domain::MoveError::Malformed) => {
                        return fail(
                            request_id,
                            self.version,
                            Fail::Malformed,
                            "Use coordinates like e2e4 or e7e8q",
                        );
                    }
                    Err(crate::domain::MoveError::Illegal) => {
                        return fail(
                            request_id,
                            self.version,
                            Fail::IllegalMove,
                            "Illegal move · choose a highlighted square",
                        );
                    }
                    Err(crate::domain::MoveError::GameOver) => {
                        return fail(request_id, self.version, Fail::GameOver, "The game is over");
                    }
                }
            }
            Intent::Resign => match self.game.resign(color) {
                Ok(_) => true,
                Err(_) => {
                    return fail(request_id, self.version, Fail::GameOver, "The game is over");
                }
            },
            Intent::ClaimDraw => {
                if self.game.turn() != color {
                    return fail(
                        request_id,
                        self.version,
                        Fail::NotClaimable,
                        "No draw can be claimed now",
                    );
                }
                match self.game.claim_draw(color) {
                    Ok(_) => true,
                    Err(crate::domain::GameError::NotClaimable) => {
                        return fail(
                            request_id,
                            self.version,
                            Fail::NotClaimable,
                            "No draw can be claimed now",
                        );
                    }
                    Err(_) => {
                        return fail(request_id, self.version, Fail::GameOver, "The game is over");
                    }
                }
            }
        };
        if played {
            self.version += 1;
            if self.game.is_over() {
                self.phase = Phase::Finished;
            }
        }
        Applied {
            request_id,
            accepted: true,
            version: self.version,
            fail: None,
            message: String::new(),
        }
    }
}

fn fail(request_id: u64, version: u64, code: Fail, message: &str) -> Applied {
    Applied {
        request_id,
        accepted: false,
        version,
        fail: Some(code),
        message: message.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::WinReason;

    fn session() -> Session {
        Session::new(Color::White, "Ana", "K7QF-M2XA".into())
    }

    fn joined() -> Session {
        let mut s = session();
        s.join("k7qf m2xa", "Ben").unwrap();
        s
    }

    #[test]
    fn invite_admits_one_peer_and_is_then_useless() {
        let mut s = session();
        assert_eq!(s.join("nope-nope", "Ben"), Err(JoinError::BadInvite));
        assert!(s.invite_open);
        let joined = s.join("K7QFM2XA", "Ben").unwrap();
        assert_eq!(joined.seat, Color::Black);
        assert_eq!(joined.token.len(), 32);
        assert_eq!(s.version(), 1);
        assert_eq!(s.join("K7QF-M2XA", "Cleo"), Err(JoinError::SeatTaken));
        assert_eq!(s.resume(&joined.token), Ok(Color::Black));
        assert_eq!(s.resume("not-the-token"), Err(JoinError::BadToken));
        assert_eq!(s.black(), "Ben");
    }

    #[test]
    fn moves_turns_duplicates_and_stale_versions() {
        let mut s = joined();
        let rejected = s.apply(Side::Peer, 1, 1, Intent::Move("e7e5".into()));
        assert_eq!(rejected.fail, Some(Fail::NotYourTurn));
        assert_eq!(s.version(), 1);
        assert!(s.game().history().is_empty());

        let accepted = s.apply(Side::Host, 1, 1, Intent::Move("e2e4".into()));
        assert!(accepted.accepted);
        assert_eq!(accepted.version, 2);
        assert_eq!(s.game().history()[0].san, "e4");

        let again = s.apply(Side::Host, 1, 99, Intent::Move("d2d4".into()));
        assert_eq!(again, accepted, "same request id is not applied twice");
        assert_eq!(s.game().history().len(), 1);

        let stale = s.apply(Side::Peer, 2, 1, Intent::Move("e7e5".into()));
        assert_eq!(stale.fail, Some(Fail::StaleVersion));
        assert_eq!(s.version(), 2);

        let reply = s.apply(Side::Peer, 3, 2, Intent::Move("e7e5".into()));
        assert!(reply.accepted);
        assert_eq!(
            s.game().fen(),
            Game::replay(s.game().start_fen(), &["e2e4", "e7e5"])
                .unwrap()
                .fen()
        );
    }

    #[test]
    fn resignation_finishes_and_blocks_moves() {
        let mut s = joined();
        let done = s.apply(Side::Host, 1, 1, Intent::Resign);
        assert!(done.accepted);
        assert_eq!(
            s.game().outcome(),
            Some(Outcome::Win {
                winner: Color::Black,
                reason: WinReason::Resignation
            })
        );
        assert_eq!(s.phase(), Phase::Finished);
        let late = s.apply(Side::Peer, 2, done.version, Intent::Move("e7e5".into()));
        assert_eq!(late.fail, Some(Fail::GameOver));
    }

    #[test]
    fn illegal_move_does_not_change_version() {
        let mut s = joined();
        let bad = s.apply(Side::Host, 1, 1, Intent::Move("e2e5".into()));
        assert_eq!(bad.fail, Some(Fail::IllegalMove));
        assert_eq!(s.version(), 1);
        assert!(s.game().history().is_empty());
    }
}
