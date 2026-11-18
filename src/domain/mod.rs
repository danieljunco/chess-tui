//! Pure chess rules. No terminal, no networking.
//!
//! Wraps `shakmaty` for legality/notation and adds what it lacks:
//! repetition and move-count draws, resignation and draw claims (ADR-005).

use std::fmt;

use shakmaty::fen::Fen;
use shakmaty::san::SanPlus;
use shakmaty::uci::UciMove;
use shakmaty::zobrist::Zobrist64;
use shakmaty::{CastlingMode, Chess, EnPassantMode, Move, Position};

pub use shakmaty::{Color, File, Piece, Rank, Role, Square};

pub const START_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WinReason {
    Checkmate,
    Resignation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DrawReason {
    Stalemate,
    InsufficientMaterial,
    FivefoldRepetition,
    SeventyFiveMoves,
    ThreefoldRepetition,
    FiftyMoves,
}

impl DrawReason {
    pub fn label(self) -> &'static str {
        match self {
            DrawReason::Stalemate => "Stalemate",
            DrawReason::InsufficientMaterial => "Insufficient material",
            DrawReason::FivefoldRepetition => "Fivefold repetition",
            DrawReason::SeventyFiveMoves => "75-move rule",
            DrawReason::ThreefoldRepetition => "Threefold repetition",
            DrawReason::FiftyMoves => "50-move rule",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win { winner: Color, reason: WinReason },
    Draw(DrawReason),
}

impl Outcome {
    pub fn score(self) -> &'static str {
        match self {
            Outcome::Win {
                winner: Color::White,
                ..
            } => "1-0",
            Outcome::Win {
                winner: Color::Black,
                ..
            } => "0-1",
            Outcome::Draw(_) => "½-½",
        }
    }

    pub fn headline(self) -> String {
        match self {
            Outcome::Win { winner, reason } => {
                let reason = match reason {
                    WinReason::Checkmate => "Checkmate",
                    WinReason::Resignation => "Resignation",
                };
                format!("{reason} · {} wins", color_name(winner))
            }
            Outcome::Draw(reason) => format!("Draw · {}", reason.label()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MoveError {
    GameOver,
    Malformed,
    Illegal,
}

impl fmt::Display for MoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            MoveError::GameOver => "The game is over",
            MoveError::Malformed => "Use coordinates like e2e4 or e7e8q",
            MoveError::Illegal => "Illegal move · choose a highlighted square",
        })
    }
}

impl std::error::Error for MoveError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GameError {
    BadFen,
    GameOver,
    NotClaimable,
    ReplayMismatch,
    Move(MoveError),
}

impl fmt::Display for GameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GameError::BadFen => f.write_str("Invalid position"),
            GameError::GameOver => f.write_str("The game is over"),
            GameError::NotClaimable => f.write_str("No draw can be claimed now"),
            GameError::ReplayMismatch => f.write_str("Position does not match move history"),
            GameError::Move(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for GameError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveRecord {
    pub uci: String,
    pub san: String,
    pub color: Color,
    pub from: Square,
    pub to: Square,
    pub captured: Option<Role>,
}

#[derive(Clone, Debug)]
pub struct Game {
    start_fen: String,
    pos: Chess,
    history: Vec<MoveRecord>,
    hashes: Vec<Zobrist64>,
    outcome: Option<Outcome>,
}

impl Default for Game {
    fn default() -> Self {
        Self::new()
    }
}

impl Game {
    pub fn new() -> Self {
        Self::from_fen(START_FEN).expect("start position is valid")
    }

    pub fn from_fen(fen: &str) -> Result<Self, GameError> {
        let pos: Chess = fen
            .parse::<Fen>()
            .map_err(|_| GameError::BadFen)?
            .into_position(CastlingMode::Standard)
            .map_err(|_| GameError::BadFen)?;
        let mut game = Self {
            start_fen: fen.to_owned(),
            hashes: vec![hash(&pos)],
            pos,
            history: Vec::new(),
            outcome: None,
        };
        game.outcome = game.automatic_outcome();
        Ok(game)
    }

    /// Rebuild a game from a start position and UCI move list, as a peer does
    /// with a host snapshot.
    pub fn replay(start_fen: &str, moves: &[impl AsRef<str>]) -> Result<Self, GameError> {
        let mut game = Self::from_fen(start_fen)?;
        for uci in moves {
            game.play_uci(uci.as_ref()).map_err(GameError::Move)?;
        }
        Ok(game)
    }

    pub fn start_fen(&self) -> &str {
        &self.start_fen
    }

    pub fn fen(&self) -> String {
        Fen::from_position(&self.pos, EnPassantMode::Legal).to_string()
    }

    pub fn turn(&self) -> Color {
        self.pos.turn()
    }

    pub fn piece_at(&self, sq: Square) -> Option<Piece> {
        self.pos.board().piece_at(sq)
    }

    pub fn history(&self) -> &[MoveRecord] {
        &self.history
    }

    pub fn last_move(&self) -> Option<&MoveRecord> {
        self.history.last()
    }

    pub fn outcome(&self) -> Option<Outcome> {
        self.outcome
    }

    pub fn is_over(&self) -> bool {
        self.outcome.is_some()
    }

    pub fn is_check(&self) -> bool {
        self.pos.is_check()
    }

    /// Square of the side-to-move's king when it is in check.
    pub fn checked_king(&self) -> Option<Square> {
        if self.pos.is_check() {
            self.pos.board().king_of(self.pos.turn())
        } else {
            None
        }
    }

    /// Pieces of `color` that have been captured so far, in capture order.
    pub fn captured(&self, color: Color) -> Vec<Role> {
        self.history
            .iter()
            .filter(|m| m.color != color)
            .filter_map(|m| m.captured)
            .collect()
    }

    /// Legal destination squares for the piece on `from`. Castling is expressed
    /// as the king's destination (g1/c1/g8/c8).
    pub fn legal_targets(&self, from: Square) -> Vec<Square> {
        if self.is_over() {
            return Vec::new();
        }
        let mut targets: Vec<Square> = self
            .pos
            .legal_moves()
            .iter()
            .filter(|m| m.from() == Some(from))
            .filter_map(|m| uci_of(m).to())
            .collect();
        targets.dedup();
        targets
    }

    pub fn needs_promotion(&self, from: Square, to: Square) -> bool {
        self.find_move(from, to, Some(Role::Queen)).is_some()
    }

    pub fn play(
        &mut self,
        from: Square,
        to: Square,
        promotion: Option<Role>,
    ) -> Result<&MoveRecord, MoveError> {
        if self.is_over() {
            return Err(MoveError::GameOver);
        }
        let m = self
            .find_move(from, to, promotion)
            .ok_or(MoveError::Illegal)?;
        Ok(self.commit(m))
    }

    pub fn play_uci(&mut self, uci: &str) -> Result<&MoveRecord, MoveError> {
        if self.is_over() {
            return Err(MoveError::GameOver);
        }
        let parsed: UciMove = uci.trim().parse().map_err(|_| MoveError::Malformed)?;
        let UciMove::Normal {
            from,
            to,
            promotion,
        } = parsed
        else {
            return Err(MoveError::Malformed);
        };
        let m = self
            .find_move(from, to, promotion)
            .ok_or(MoveError::Illegal)?;
        Ok(self.commit(m))
    }

    pub fn resign(&mut self, color: Color) -> Result<Outcome, GameError> {
        if self.is_over() {
            return Err(GameError::GameOver);
        }
        let outcome = Outcome::Win {
            winner: !color,
            reason: WinReason::Resignation,
        };
        self.outcome = Some(outcome);
        Ok(outcome)
    }

    /// Draw the side to move may claim right now (threefold or 50-move rule).
    pub fn claimable_draw(&self) -> Option<DrawReason> {
        if self.is_over() {
            return None;
        }
        if self.repetitions() >= 3 {
            Some(DrawReason::ThreefoldRepetition)
        } else if self.pos.halfmoves() >= 100 {
            Some(DrawReason::FiftyMoves)
        } else {
            None
        }
    }

    /// Apply an outcome the move list cannot show (resignation, an accepted
    /// draw claim). Automatic results must already match `declared`.
    pub fn reconcile_outcome(&mut self, declared: Option<Outcome>) -> Result<(), GameError> {
        if self.outcome == declared {
            return Ok(());
        }
        match (self.outcome, declared) {
            (None, Some(outcome)) if is_declared(outcome) => {
                self.outcome = Some(outcome);
                Ok(())
            }
            _ => Err(GameError::ReplayMismatch),
        }
    }

    pub fn claim_draw(&mut self, claimant: Color) -> Result<Outcome, GameError> {
        if self.is_over() {
            return Err(GameError::GameOver);
        }
        if claimant != self.turn() {
            return Err(GameError::NotClaimable);
        }
        let reason = self.claimable_draw().ok_or(GameError::NotClaimable)?;
        let outcome = Outcome::Draw(reason);
        self.outcome = Some(outcome);
        Ok(outcome)
    }

    fn find_move(&self, from: Square, to: Square, promotion: Option<Role>) -> Option<Move> {
        self.pos.legal_moves().into_iter().find(|m| {
            let uci = uci_of(m);
            uci.from() == Some(from) && uci.to() == Some(to) && uci.promotion() == promotion
        })
    }

    fn commit(&mut self, m: Move) -> &MoveRecord {
        let color = self.pos.turn();
        let uci = uci_of(&m);
        let san = SanPlus::from_move_and_play_unchecked(&mut self.pos, m);
        self.hashes.push(hash(&self.pos));
        self.history.push(MoveRecord {
            uci: uci.to_string(),
            san: san.to_string(),
            color,
            from: uci.from().expect("normal move"),
            to: uci.to().expect("normal move"),
            captured: m.capture(),
        });
        self.outcome = self.automatic_outcome();
        self.history.last().expect("just pushed")
    }

    fn repetitions(&self) -> usize {
        let current = self.hashes.last().copied().unwrap_or_default();
        self.hashes.iter().filter(|h| **h == current).count()
    }

    fn automatic_outcome(&self) -> Option<Outcome> {
        if let Some(known) = self.pos.outcome().known() {
            return Some(match known.winner() {
                Some(winner) => Outcome::Win {
                    winner,
                    reason: WinReason::Checkmate,
                },
                None if self.pos.is_stalemate() => Outcome::Draw(DrawReason::Stalemate),
                None => Outcome::Draw(DrawReason::InsufficientMaterial),
            });
        }
        if self.repetitions() >= 5 {
            return Some(Outcome::Draw(DrawReason::FivefoldRepetition));
        }
        if self.pos.halfmoves() >= 150 {
            return Some(Outcome::Draw(DrawReason::SeventyFiveMoves));
        }
        None
    }
}

pub fn color_name(color: Color) -> &'static str {
    match color {
        Color::White => "White",
        Color::Black => "Black",
    }
}

pub fn parse_square(s: &str) -> Option<Square> {
    s.parse().ok()
}

pub fn format_uci(from: Square, to: Square, promotion: Option<Role>) -> String {
    let mut uci = format!("{from}{to}");
    if let Some(role) = promotion {
        uci.push(role.char());
    }
    uci
}

fn is_declared(outcome: Outcome) -> bool {
    matches!(
        outcome,
        Outcome::Win {
            reason: WinReason::Resignation,
            ..
        } | Outcome::Draw(DrawReason::ThreefoldRepetition | DrawReason::FiftyMoves)
    )
}

fn uci_of(m: &Move) -> UciMove {
    m.to_uci(CastlingMode::Standard)
}

fn hash(pos: &Chess) -> Zobrist64 {
    pos.zobrist_hash(EnPassantMode::Legal)
}

#[cfg(test)]
mod tests;
