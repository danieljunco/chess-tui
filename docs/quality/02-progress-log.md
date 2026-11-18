# Implementation progress

> Update after each completed ticket. Do not mark work complete without verification.

| Ticket | Status | Evidence / test command | Notes |
|---|---|---|---|
| CHESS-01 | **Done** (2026-11-18) | ADR-001…006 in `docs/decisions/`; shakmaty 0.30.2 source inspected for draw coverage | Realigned from central server to embedded host-peer P2P; see "Architecture alignment" below |
| CHESS-02 | **Done** (2026-11-18) | `cargo test domain::` — 13 tests | shakmaty wrapper + repetition / 50 / 75-move rules, resign, claims, replay |
| CHESS-03 | **Visual gate 1 passed — pending real-terminal review** | `cargo test ui::` (9), `cargo test app::` (19), `cargo run --example snapshots`, `scripts/pty_smoke.py` (11/11) | Mock-provider prototype `vesper demo`; screenshots below. Not yet seen in a human-operated terminal emulator |
| CHESS-04 | In progress | `vesper local` playable; app tests cover select/move, coordinate entry, promotion (all 4 via dialog/keys), resign, checkmate overlay | Full manual pass-and-play game not yet played to the end by a human |
| CHESS-05 | **Done** (2026-11-18) | `cargo test session:: net::` | Host listens in-process. Invite is single-use; seat token resumes. No separate server |
| CHESS-06 | **Done, automated only** (2026-11-18) | `net::tests` over `127.0.0.1`: sync, illegal/out-of-turn, bad invite delay, resume, duplicate request, host quit, oversized frame | Two OS terminals were not opened by hand. Reconnect backoff is implemented; a multi-second silence test was not run |
| CHESS-07 | Not started | — | Visual gate required |
| CHESS-08 | Not started | — | |

## Architecture alignment (Milestone 1, 2026-11-18)

Centralized-server assumptions found and resolved:

| Where | Old assumption | Resolution |
|---|---|---|
| README, prompt, AGENTS.md | "server authority", "WebSocket transport", separate `server` binary | Hosting peer is authoritative; one `vesper` binary; direct TCP (ADR-001/002) |
| architecture/01 | `chess-server` crate, session registry, client/server diagram | Host-peer diagram, module boundaries, host & peer responsibilities, lifecycle |
| architecture/02 | `wss://` remote deployment, `create_game` on server, server restart threat | Handshake `hello`/`resume`, invite vs seat token, snapshot sync, host-quit threat, honest NAT limits |
| concept/01 | server returns join code; server restart ends games | Host shows address + invite code; host quitting ends the game |
| concept/02 | `GameId` server identifier, server sequence | `HostAddress`, `InviteCode`, host version |
| design/02, 03 | "New private game", share-code waiting room, `c` copy code | Host / Join / Local menu, join form, waiting room with `vesper join …`, `g`/`d` keys, offline/host-closed copy |
| quality/01 | "launch server", server integration tests | Two-peer tests over `127.0.0.1`, host-quit scenario |
| tickets 05/06 | Authoritative server / client | `CHESS-05-host-session.md`, `CHESS-06-peer-sync.md` |

Preserved unchanged: rules scope, keyboard model, visual system, seat-token protection, versions/idempotency, untimed MVP, non-goals.

## Visual review evidence

Process: `cargo run --example snapshots` renders 22 named scenes (`app::fixtures::scenes`) through the real `ui::draw` into ratatui `TestBackend` buffers at 120×36, 100×32, 90×28, 80×24 and 70×20; HTML is rasterised by headless Chrome via `scripts/screenshot.sh`. Text buffers for 120×36 and 80×24 are in [`snapshots/`](snapshots/).

**Caveat:** Chrome substitutes a fallback font for chess glyphs, so pieces look smaller than in most terminals. Layout, colour, hierarchy and clipping are representative; glyph size/weight is not.

| Scene | Screenshot |
|---|---|
| Home 120×36 | [home](screenshots/home-120x36.png) |
| Waiting room | [waiting](screenshots/waiting-120x36.png) |
| Your move, piece selected, legal targets | [game-your-move](screenshots/game-your-move-120x36.png) |
| Cursor on capture target | [game-cursor-capture](screenshots/game-cursor-capture-120x36.png) |
| Black orientation, in check | [game-black-check](screenshots/game-black-check-120x36.png) |
| Promotion dialog | [promotion](screenshots/promotion-120x36.png) |
| Reconnecting (peer) | [reconnecting](screenshots/reconnecting-120x36.png) |
| Checkmate result | [result-checkmate](screenshots/result-checkmate-120x36.png) |
| Help overlay | [help](screenshots/help-120x36.png) |
| ASCII fallback 100×32 | [ascii](screenshots/ascii-100x32.png) |
| Minimum size 80×24 | [game-your-move-80x24](screenshots/game-your-move-80x24.png) |
| Join form error 80×24 | [join-error](screenshots/join-error-80x24.png) |

Issues found during review and fixed: clipped home hint, promotion "Knight", rail outcome text, help footnote and join hints; rail panels stretched with empty space; TABLE address clipped; duplicate "Checkmate" wording in result dialog; captured pieces and TABLE glyphs ignored ASCII mode; demo join ignored the typed address; light squares lightened white-piece contrast too much (darkened `#9AA1B0 → #8C94A4`).

Automated layout guards (`src/ui/tests.rs`): every scene renders at 7 sizes in colour and mono; all ranks/files, header and MOVES visible at supported sizes; key copy present per state; promotion lists all four pieces; mono marks legal squares (`•`) and cursor (`┏┛`); ASCII mode contains no Unicode pieces; no "token" text ever rendered; below 80×24 shows the resize screen.

### Not yet verified
- Two terminal windows played by hand (the same host/peer code is covered in-process over TCP).
- Viewing in real terminal emulators (iTerm2, Terminal.app, Alacritty, WezTerm, kitty, Windows Terminal), including `♟` emoji presentation in some fonts.
- Contrast measured on real terminals; light-theme variant (post-MVP).
- Panic-path terminal restore (ratatui's panic hook is installed; not exercised by a test).
- Resize *during* interaction (layout recomputes every frame; render tests cover each size statically).

## Gate results (2026-11-18, Rust 1.100.0, macOS)

```text
cargo fmt --check                                          ok
cargo clippy --all-targets --all-features -- -D warnings   ok
cargo test --workspace                                     53 passed, 0 failed
python3 scripts/pty_smoke.py <debug vesper> local          11/11 PASS
```

P2P is the default path as of this entry. `vesper host` / `vesper join` / the home menu open a direct TCP connection. `vesper demo` is the only mock. Verified by `src/net/tests.rs` (two peers on `127.0.0.1`), not by two hand-driven terminal windows.

## Decisions / scope changes

- ADR-001…006 accepted. Scope change: central server replaced by embedded host peer (required by product owner). No MVP requirement removed.
- Deviation from visual spec: filled glyphs for both sides in colour mode (ADR-006), with outline White in mono.
- Draw by agreement is not in MVP; claimable draws use the simplified current-position rule (ADR-005).
- Move numbers assume games start from move 1 (fixtures do); custom-FEN numbering is a known gap.
