# ADR-005: shakmaty for chess rules; explicit draw adjudication
Status: Accepted
Date: 2026-11-18

## Context
Legal move generation must come from an established crate. Draw semantics need verification.

## Options
- **shakmaty 0.30** — mature (used by lila-adjacent tooling), legal move generation, FEN, SAN/SAN+, UCI, `outcome()`, Zobrist hashing. Actively maintained.
- `chess` crate — fast movegen, less complete SAN/draw support, less active.
- `cozy-chess` — excellent movegen, minimal notation support.

## Verified shakmaty coverage (source inspected, v0.30.2)
- `legal_moves()`, `is_legal()`, `play()` (returns error and leaves state untouched), castling (incl. attacked-path rules), en passant, promotion to any role.
- `outcome()` covers **checkmate, stalemate, insufficient material** (both sides; conservative "dead position" subset only).
- `halfmoves()` clock and `zobrist_hash(EnPassantMode::Legal)` are available, but **no repetition or move-count rules**.

## Decision — supported draw rules (domain `Game` wrapper implements the gaps)
| Rule | Kind | Implementation |
|---|---|---|
| Stalemate | automatic | shakmaty |
| Insufficient material (K vs K, K+minor vs K, same-colour bishops) | automatic | shakmaty |
| Fivefold repetition | automatic | our Zobrist history |
| 75-move rule (150 halfmoves) | automatic, unless the 150th move mates (mate is checked first) | our check on `halfmoves()` |
| Threefold repetition | **claimable** by side to move | our Zobrist history; `claim_draw` command |
| 50-move rule (100 halfmoves) | **claimable** | `claim_draw` command |
| Draw by agreement | **not in MVP** (no draw offers) | — |
| General dead positions beyond insufficient material | **not detected** | documented limitation |

Claims are simplified: the player may claim when the *current* position satisfies the condition (we do not support claiming on an intended move as FIDE 9.2.1 allows).

## Consequences
Chess correctness rests on a well-tested crate; our own code is limited to history-based rules, which we unit-test.
