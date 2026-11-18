# ADR-006: Single package, module boundaries, terminal baseline
Status: Accepted
Date: 2026-11-18

## Decision
**One Cargo package** (`vesper-chess`, binary `vesper`) with strict module boundaries instead of a multi-crate workspace — there is now only one executable, so crates would add ceremony without isolation benefits. Boundaries are enforced by dependency direction:

```text
src/
  domain/    pure chess (shakmaty wrapper). Depends on nothing else in the crate.   [built]
  session/   authoritative two-seat session: versions, idempotency, outcomes.      [built]
  protocol/  wire DTOs + validation (serde). Depends on domain types only.         [built]
  net/       tokio TCP host/peer tasks, heartbeats, reconnect.                     [built]
  app/       UI state machine (screens, focus, selection) + GameProvider seam       [built]
             (local and demo mocks; host/peer are the direct connection). Never touches sockets.
  ui/        Ratatui rendering: theme tokens, board, panels, overlays.              [built]
  main.rs    CLI (clap), terminal setup/restore, event loop.                        [built]
```

`GameProvider` is a non-blocking `send` + `poll(now)` pair, so network providers can wrap bounded Tokio channels (`try_send` / `try_recv`) without the UI loop ever awaiting.

`app` talks to a game via a channel-based **provider** interface (commands in, `GameView` updates out). The mock provider serves scripted fixture states for the visual prototype; later the local, host and peer providers plug into the same seam. The UI never knows whether a game is local or remote except through view-level facts (connection status, seat).

## Terminal baseline
- Ratatui 0.30 + Crossterm 0.29. Alternate screen + raw mode, restored on normal exit, error and panic (panic hook).
- Minimum **80×24**; below that a designed "terminal too small" screen. Preferred ≥100×32.
- Board squares use the largest of **7×3**, **5×2**, **3×1** cells that fits (7×3 at ≥100×32, 5×2 at 80×24–90×28). Pieces are single glyphs centred in the square.
- Unicode pieces by default, drawn as **filled silhouettes for both sides** distinguished by near-white / near-black foreground (outline glyphs were too thin on mid-tone squares). Without colour (`NO_COLOR` / `--no-color`) White switches to outline glyphs. `--ascii` (or `g` in game) uses `K Q R B N P` / `k q r b n p`.
- State redundancy without colour: cursor = corner brackets `┏┓┗┛` (or `[ ]` on 3×1), legal empty square = `•`, capture target = `×`, check = `!`, selection/check = reverse video in mono.
- Visual review: `cargo run --example snapshots` renders every named scene (`app::fixtures::scenes`) at 120×36, 100×32, 90×28, 80×24 and 70×20 with `TestBackend` into text buffers plus true-colour HTML; `scripts/screenshot.sh` rasterises the HTML with headless Chrome. Caveat: the browser falls back to a non-terminal font for chess glyphs, so piece size/weight in screenshots is only indicative. Layout regressions are guarded by assertion-based render tests (`src/ui/tests.rs`) rather than golden files, to avoid churn while the design is tuned.
- Real-terminal smoke: `scripts/pty_smoke.py` drives the binary in a pseudo-terminal and checks alternate-screen entry/exit, a keyboard move and clean exit.

## Consequences
Low build complexity; if a headless host daemon is wanted later, `domain/session/protocol/net` can be split into a library crate without code changes.
