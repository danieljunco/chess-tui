# CHESS-02 — Chess domain & game rules

**Priority:** P0 · **Depends on:** CHESS-01 · **Type:** domain

## Outcome

Correct, reusable game rules independent of UI and network.

## Concepts

Squares, piece colors, legal vs pseudo-legal moves, castling rights, en passant, promotion, check/mate, FEN, SAN, repetition tracking, automatic vs claimable draw rules.

## Work

- Wrap the selected rules library behind a small domain API.
- Model position, legal moves, turn, applied move, history and outcomes.
- Implement adjudication gaps identified in CHESS-01; history-based repetition where required.
- Support UCI submission and human-readable SAN output.
- Make illegal and completed-game operations non-mutating with typed errors.
- Add fixtures for normal moves, mate, stalemate, special moves and draw cases.

## Acceptance

- [ ] Legal moves apply; illegal/out-of-turn moves do not mutate state.
- [ ] Castling, en passant and promotion (all 4 outcomes) correct.
- [ ] Check, mate, stalemate, and documented draw rules correct.
- [ ] FEN and move history accurate; SAN representation tested.
- [ ] Unit tests are deterministic and cover edge cases.
