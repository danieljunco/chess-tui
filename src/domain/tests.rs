use super::*;

fn play_all(game: &mut Game, moves: &[&str]) {
    for m in moves {
        game.play_uci(m).unwrap_or_else(|e| panic!("{m}: {e}"));
    }
}

#[test]
fn start_position_accepts_e2e4_and_rejects_black_reply_from_white() {
    let mut game = Game::new();
    assert_eq!(game.turn(), Color::White);
    assert!(
        game.play_uci("e7e5").is_err(),
        "white cannot move black pawn"
    );
    assert_eq!(game.fen(), START_FEN, "rejected move must not mutate");
    let rec = game.play_uci("e2e4").unwrap();
    assert_eq!(rec.san, "e4");
    assert_eq!(game.turn(), Color::Black);
}

#[test]
fn malformed_and_illegal_moves_do_not_mutate() {
    let mut game = Game::new();
    assert_eq!(game.play_uci("zz99"), Err(MoveError::Malformed));
    assert_eq!(game.play_uci("e2e5"), Err(MoveError::Illegal));
    assert_eq!(game.play_uci("0000"), Err(MoveError::Malformed));
    assert!(game.history().is_empty());
    assert_eq!(game.fen(), START_FEN);
}

#[test]
fn legal_targets_for_knight_and_pawn() {
    let game = Game::new();
    assert_eq!(game.legal_targets(Square::G1), vec![Square::F3, Square::H3]);
    let mut pawn = game.legal_targets(Square::E2);
    pawn.sort();
    assert_eq!(pawn, vec![Square::E3, Square::E4]);
    assert!(
        game.legal_targets(Square::E7).is_empty(),
        "not side to move"
    );
}

#[test]
fn castling_clear_blocked_and_attacked() {
    let clear = Game::from_fen("r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1").unwrap();
    let targets = clear.legal_targets(Square::E1);
    assert!(targets.contains(&Square::G1) && targets.contains(&Square::C1));

    let blocked = Game::from_fen("r3k2r/8/8/8/8/8/8/R3KB1R w KQkq - 0 1").unwrap();
    assert!(!blocked.legal_targets(Square::E1).contains(&Square::G1));

    // Black rook on f8 attacks f1: cannot castle through it.
    let attacked = Game::from_fen("r3kr2/8/8/8/8/8/8/R3K2R w KQq - 0 1").unwrap();
    assert!(!attacked.legal_targets(Square::E1).contains(&Square::G1));
    assert!(attacked.legal_targets(Square::E1).contains(&Square::C1));

    let mut game = clear;
    let rec = game.play(Square::E1, Square::G1, None).unwrap();
    assert_eq!(rec.san, "O-O");
    assert_eq!(rec.uci, "e1g1");
    assert_eq!(game.piece_at(Square::F1).map(|p| p.role), Some(Role::Rook));
}

#[test]
fn en_passant_only_on_immediate_reply() {
    let mut game = Game::new();
    play_all(&mut game, &["e2e4", "a7a6", "e4e5", "d7d5"]);
    assert!(game.legal_targets(Square::E5).contains(&Square::D6));
    let rec = game.play_uci("e5d6").unwrap();
    assert_eq!(rec.captured, Some(Role::Pawn));
    assert_eq!(game.piece_at(Square::D5), None);

    let mut late = Game::new();
    play_all(&mut late, &["e2e4", "a7a6", "e4e5", "d7d5", "h2h3", "h7h6"]);
    assert!(!late.legal_targets(Square::E5).contains(&Square::D6));
}

#[test]
fn promotion_to_all_four_pieces() {
    let fen = "8/4P3/8/8/8/8/k7/4K3 w - - 0 1";
    for (suffix, role, san) in [
        ("q", Role::Queen, "e8=Q"),
        ("r", Role::Rook, "e8=R"),
        ("b", Role::Bishop, "e8=B"),
        ("n", Role::Knight, "e8=N"),
    ] {
        let mut game = Game::from_fen(fen).unwrap();
        assert!(game.needs_promotion(Square::E7, Square::E8));
        assert_eq!(
            game.play(Square::E7, Square::E8, None),
            Err(MoveError::Illegal),
            "promotion piece must be explicit"
        );
        let rec = game.play_uci(&format!("e7e8{suffix}")).unwrap();
        assert!(rec.san.starts_with(san), "{} vs {san}", rec.san);
        assert_eq!(game.piece_at(Square::E8).map(|p| p.role), Some(role));
    }
}

#[test]
fn fools_mate_is_checkmate_and_blocks_further_moves() {
    let mut game = Game::new();
    play_all(&mut game, &["f2f3", "e7e5", "g2g4", "d8h4"]);
    assert_eq!(game.last_move().unwrap().san, "Qh4#");
    assert_eq!(
        game.outcome(),
        Some(Outcome::Win {
            winner: Color::Black,
            reason: WinReason::Checkmate
        })
    );
    assert_eq!(game.checked_king(), Some(Square::E1));
    assert_eq!(game.play_uci("a2a3"), Err(MoveError::GameOver));
    assert!(game.legal_targets(Square::A2).is_empty());
    assert_eq!(game.resign(Color::White), Err(GameError::GameOver));
}

#[test]
fn stalemate_and_insufficient_material_are_automatic() {
    let stalemate = Game::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert_eq!(
        stalemate.outcome(),
        Some(Outcome::Draw(DrawReason::Stalemate))
    );

    let mut kvk = Game::from_fen("8/8/8/3k4/8/8/3qK3/8 w - - 0 1").unwrap();
    kvk.play_uci("e2d2").unwrap();
    assert_eq!(
        kvk.outcome(),
        Some(Outcome::Draw(DrawReason::InsufficientMaterial))
    );
}

const KNIGHT_SHUFFLE: [&str; 4] = ["g1f3", "g8f6", "f3g1", "f6g8"];

#[test]
fn threefold_is_claimable_and_fivefold_is_automatic() {
    let mut game = Game::new();
    play_all(&mut game, &KNIGHT_SHUFFLE);
    assert_eq!(game.claimable_draw(), None, "start position seen twice");
    play_all(&mut game, &KNIGHT_SHUFFLE);
    assert_eq!(game.claimable_draw(), Some(DrawReason::ThreefoldRepetition));
    assert_eq!(game.claim_draw(Color::Black), Err(GameError::NotClaimable));
    assert!(!game.is_over());

    let mut fivefold = game.clone();
    play_all(&mut fivefold, &KNIGHT_SHUFFLE);
    play_all(&mut fivefold, &KNIGHT_SHUFFLE);
    assert_eq!(
        fivefold.outcome(),
        Some(Outcome::Draw(DrawReason::FivefoldRepetition))
    );

    assert_eq!(
        game.claim_draw(Color::White),
        Ok(Outcome::Draw(DrawReason::ThreefoldRepetition))
    );
    assert!(game.is_over());
}

#[test]
fn fifty_move_claimable_and_seventy_five_automatic() {
    let fifty = Game::from_fen("8/8/8/3k4/8/8/R3K3/8 w - - 100 80").unwrap();
    assert_eq!(fifty.claimable_draw(), Some(DrawReason::FiftyMoves));
    assert!(!fifty.is_over());

    let mut almost = Game::from_fen("8/8/8/3k4/8/8/R3K3/8 w - - 149 80").unwrap();
    almost.play_uci("e2e3").unwrap();
    assert_eq!(
        almost.outcome(),
        Some(Outcome::Draw(DrawReason::SeventyFiveMoves))
    );
}

#[test]
fn checkmate_on_the_150th_halfmove_wins_over_75_move_rule() {
    let mut game = Game::from_fen("k7/8/1K6/8/8/8/8/7R w - - 149 90").unwrap();
    game.play_uci("h1h8").unwrap();
    assert_eq!(
        game.outcome(),
        Some(Outcome::Win {
            winner: Color::White,
            reason: WinReason::Checkmate
        })
    );
}

#[test]
fn resignation_ends_game_for_opponent() {
    let mut game = Game::new();
    assert_eq!(
        game.resign(Color::White),
        Ok(Outcome::Win {
            winner: Color::Black,
            reason: WinReason::Resignation
        })
    );
    assert_eq!(game.play_uci("e2e4"), Err(MoveError::GameOver));
}

#[test]
fn replay_reproduces_fen_and_captures() {
    let mut game = Game::new();
    let moves = ["e2e4", "d7d5", "e4d5", "d8d5", "b1c3"];
    play_all(&mut game, &moves);
    let replayed = Game::replay(START_FEN, &moves).unwrap();
    assert_eq!(replayed.fen(), game.fen());
    assert_eq!(game.captured(Color::Black), vec![Role::Pawn]);
    assert_eq!(game.captured(Color::White), vec![Role::Pawn]);
    assert!(Game::replay(START_FEN, &["e2e4", "e2e4"]).is_err());
}
