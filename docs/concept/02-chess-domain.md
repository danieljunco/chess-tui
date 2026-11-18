# Chess domain model & rules glossary

## Bounded contexts

**Chess Rules**: position, legal moves, turn, outcome, board history. No terminal and no network dependencies.

**Game Session**: owned by the **hosting peer**. Invite code, two seats, seat tokens, waiting/active/completed lifecycle, command authorization, authoritative sequence/version. The host's own commands go through the same authorization path as the remote peer's.

**Presentation**: board cursor, selection, modal, active pane, scroll, visual feedback. These must never mutate authoritative game state directly.

**Transport**: direct host⇄peer TCP connection lifecycle, serialized requests/events, reconnect, snapshot reconciliation. DTOs live at edges.

## Core concepts

| Term | Meaning / invariants |
|---|---|
| `HostAddress` | `ip:port` the host listens on; shared out-of-band with the invite code |
| `InviteCode` | Short single-use invitation to fill the open seat; consumed on join (ADR-003) |
| `Seat` | White or Black; assignment never changes during a game |
| `SeatToken` | Private, unguessable credential bound to a game and seat; never put in logs |
| `Position` | Pieces, side to move, castling rights, en-passant target, counters |
| `Square` | One of a1…h8 |
| `Move` | From, to, optional promotion choice, submitted as UCI-like coordinates |
| `MoveRecord` | Accepted move, SAN, resulting version, optionally timestamp |
| `GamePhase` | Waiting, Active, Finished |
| `GameOutcome` | Winner/reason or draw/reason, not interchangeable with network status |
| `PositionVersion` | Monotonic host sequence for move ordering and synchronization |

## Rules and edge cases

- Validate **legal** moves, not pseudo-legal moves; moving into or remaining in check is illegal.
- Support castling rights, rook/path/attacked-square restrictions; en passant and promotion.
- Choose promotion piece explicitly (Q/R/B/N), default only if UX specifies and confirms.
- SAN for human-readable history; FEN for position serialization/snapshots; UCI for move submission.
- Distinguish **automatic** draws (e.g. stalemate, dead position, fivefold repetition, 75-move rule subject to mate exception) from **claimable** draws (e.g. threefold repetition, 50-move rule). Track sufficient historical position information; a current FEN alone is not enough for repetition claims.
- Draw offers are optional post-MVP unless explicitly implemented in the final protocol; no fake `Draw` button.
- Resignation is a session command, not a chess move; only a seated player of an active game can resign.
- At terminal result, no more moves are accepted. An accepted move must yield exactly one authoritative state version.

## Illustrative app model (not literal crate API)

Supported draw rules and their automatic/claimable classification are fixed in [ADR-005](../decisions/ADR-005-rules-crate-and-draws.md).

```text
GameSession            (lives in the host process)
  invite_code, phase, version
  white_seat, black_seat
  position, move_history, outcome

ChessMove
  from: Square
  to: Square
  promotion?: Queen|Rook|Bishop|Knight

UiState
  focus, board_cursor, selected_square?
  candidate_moves, pending_move?, toast?, overlay?
```

## Test fixtures to prepare

- Start position: `e2e4` accepted for White, `e7e5` rejected for White.
- Castling with clear/blocked/attacked routes.
- En-passant capture on the immediately eligible reply only.
- Promotion to all four pieces (including underpromotion).
- Fool's mate or Scholar's mate for terminal checkmate.
- Stalemate FEN, insufficient material/dead position, repetitions, 50/75 move counts.
- Undo not allowed in authoritative multiplayer MVP.
