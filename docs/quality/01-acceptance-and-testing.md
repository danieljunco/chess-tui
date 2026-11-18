# Definition of done & testing strategy

## Product-level acceptance

1. A runs `vesper host`; B runs `vesper join <address> <code>` and connects **directly** to A; only these two obtain seats. No other service runs.
2. White/Black board orientations are correct and can be flipped visually.
3. Both can play a full **legal, untimed** match, including promotion and castling.
4. Illegal, out-of-turn, repeated and stale commands are rejected without game divergence.
5. Check/checkmate/stalemate and defined draw conditions have accurate results and copy.
6. Both peers see the same accepted move history, FEN and `version`.
7. A disconnected joining peer resumes its **same seat** with its private seat token while the host runs; the invite code alone never grants a seat after it is consumed. Host quitting ends the game predictably on the peer.
8. All gameplay actions are keyboard accessible; a terminal with no mouse is sufficient.
9. UI always shows turn, selection, status, errors, and connection state unambiguously.
10. Terminal restores on clean quit, common errors and supported panic handling.

## Test layers

| Layer | Coverage |
|---|---|
| Domain unit | Legal/illegal moves, turn, mate/stalemate, castling, en passant, all promotions, repetition/clock history draw semantics |
| Session unit | Seat ownership, host and peer through the same path, resignation, claims, finished state, version updates, duplicate request IDs |
| Protocol unit | Malformed/oversized/unknown frames, invalid invite/token, stale versions |
| Host⇄peer integration | Two in-process peers over `127.0.0.1`: join, simultaneous moves, disconnect, resume, snapshot replay, idempotency, bad codes/tokens, host quit |
| App integration | Input modes, move selection, promotion, focus, pending/rejected command states |
| TUI snapshots | Home, waiting, white/black turns, focus, legal moves, promotion, reconnecting, result, help |
| Manual visual QA | Sizes 120×36, 100×32, 90×28, 80×24, below-minimum, Unicode/ASCII fallback, dark terminal variants |

## Visual design gates

- **Visual gate 1 (CHESS-03):** Save snapshots/screenshots of mock UI in all primary states (`cargo run --example snapshots` → `docs/quality/snapshots/`). Compare to `docs/design/`; refine before connecting networking.
- **Visual gate 2 (CHESS-04):** Selection, legal markers, promotion and mate flows feel coherent across keyboard-only sessions.
- **Visual gate 3 (CHESS-07):** Review real terminal captures at supported widths; no clipping, no inconsistent color use, no unreadable glyphs, no overlapping panels.
- **Keyboard gate:** All actions reachable without a mouse; focus indication survives resizing.
- **Accessibility gate:** Validate useful contrast, distinguishability without color, alternate piece representation, and readable errors. Terminal rendering varies, so include manual testing.

## Runtime quality

- Input/render never waits on networking.
- Measure perceived input latency and redraw costs; avoid gratuitous full-screen flicker.
- Error paths preserve terminal integrity.
- Log commands/errors without seat tokens or invite codes; optional debug log goes to a file, never to the TUI's stdout.
- Validate input bounds and message size; do not permit unbounded session/event buffers.

## Suggested CI commands

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --workspace
cargo build --release --workspace
```

If CI or tooling is unavailable, report what was not run. Never assert success without execution.

## Manual full-game script

1. Terminal A: `vesper host`. Terminal B (same machine or LAN): `vesper join <address> <code>`.
2. Play normal moves from both sides; inspect SAN/history on both.
3. Attempt illegal move and out-of-turn move; verify rejection.
4. Trigger castling, en passant, promotion in test fixtures.
5. Interrupt B's network (or kill the TCP path) midgame, make A's move if it's A's turn, restore and verify B resyncs to the same seat.
6. Finish a game with checkmate; verify result and blocked new moves.
7. Start another game, resign; verify both terminals' results.
8. Quit the host midgame; verify B shows `Host closed the game` and stays usable.
9. Resize terminals during play; open help, command entry and modal; confirm no broken focus.
