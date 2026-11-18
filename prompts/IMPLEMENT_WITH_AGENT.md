# Copy/paste agent prompt — implement Vesper Chess

You are a senior Rust engineer, multiplayer systems architect, and terminal UX/UI designer. Implement **Vesper Chess**, a premium keyboard-first multiplayer chess TUI, using the documentation in this repository as the source of truth.

## First actions

1. Read `AGENTS.md`, `README.md`, all of `docs/concept/`, `docs/design/`, `docs/architecture/`, `docs/quality/01-acceptance-and-testing.md`, and `docs/tickets/README.md`.
2. Review ticket documents in dependency order. Check actual repo state before creating files; don't overwrite existing work blindly.
3. Identify existing code, build tooling, and any deviations from docs. Validate library API suitability before adopting a crate.
4. Start from **CHESS-01**. Maintain `docs/quality/02-progress-log.md` with evidence.

## Mission

Create a playable, beautiful Rust/Ratatui **peer-to-peer** terminal chess application where one human hosts a game from their terminal (`vesper host`) and another connects directly (`vesper join <address> <code>`). There is no central server: the hosting peer is authoritative. Rules must be correct, synchronization resilient, and the terminal experience must have outstanding visual craft.

## NON-NEGOTIABLE UX DIRECTION

The product should feel like a polished, deliberate developer tool—not a proof-of-concept CLI. Take inspiration from the **information density, subtle borders, distinctive focus states, and keyboard-first ergonomics of JiraTUI**, adapted to chess. Follow `docs/design/01-visual-system.md`, `02-interaction-model.md`, and `03-screens-and-wireframes.md`.

- Board is the focal point; panels are secondary.
- Implement semantic theme tokens and restrained dark palette.
- The board is visually legible with consistent square width, rank/file labels, clear selection, legal destinations, last move, check, and turn status.
- Handle terminal resize, Unicode differences, ASCII fallback, pending/error/reconnect states, and modal focus.
- Create a mock-data TUI milestone, **visually review screenshots or terminal buffers and iterate** before networking.
- Do not declare a ticket complete if the UI is merely functional but visually unfinished.

## Engineering constraints

- Rust, Ratatui, Crossterm, Tokio; shakmaty for rules; direct TCP with NDJSON frames (see `docs/decisions/`).
- Keep chess rules/domain isolated from presentation/transport.
- The hosting peer owns all committed game state and legal move validation; its own player and the joining peer both submit intentions through the same session path.
- Keep network operations async; never block rendering/input.
- A public join code must not be a reconnect/authorization secret. Protect tokens and rate-limit joins.
- Implement versioned snapshots, stale request handling and request-id idempotency.
- Support untimed games only for MVP; no AI/accounts/matchmaking unless scope is explicitly revised.
- Honor the differences between automatic and claimable draws; document exact supported behavior.
- Prefer simple, testable modules and avoid unjustified abstractions.

## Execution order

1. **CHESS-01:** Verify dependencies, decide minimal architecture/protocol and write ADRs.
2. **CHESS-03 (visual mock milestone):** Build home/waiting/in-game/help/dialog/result screens using coherent mock chess fixtures. Make layout and interactions excellent. Capture terminal screenshots or render snapshots. Fix visible flaws.
3. **CHESS-02 + CHESS-04:** Build rules/domain, then playable local pass-and-play mode using same components.
4. **CHESS-05:** Build the embedded authoritative host session + listener and integration tests.
5. **CHESS-06:** Wire the joining peer, join/resume/sync; prove two terminal sessions playing.
6. **CHESS-07:** UX polish, readability, responsive layouts, error states, board orientation.
7. **CHESS-08:** Final automated/manual QA, installation docs, release readiness.

CHESS-02 and CHESS-03 may be developed in parallel if your tooling permits; this list emphasizes a visual-first demo, not a code dependency ordering change.

## Milestone behavior

At the end of each ticket:
- Run relevant formatting/lints/tests and report actual results (not assumptions).
- Demonstrate runnable behavior where possible.
- Update progress log and link visual evidence.
- Record significant decisions and scope deviations in `docs/decisions/`.
- Fix blockers before moving to dependent tickets.

## Definition of done

Meet product-level criteria in `docs/quality/01-acceptance-and-testing.md`. In particular, two terminals must play a full legal game, stay synchronized across reconnect, show accurate end conditions, and remain pleasant to use across supported terminal sizes.

## Output expectations

Produce working code, tests, concise setup instructions, and accurate progress documentation. If the environment prevents visual validation, say what was not checked, retain a visual QA task as open, and do **not** claim aesthetic acceptance.

**Begin by inspecting the repository and completing CHESS-01. Continue through executable vertical slices in this session as far as the environment permits; do not skip verification or invent test results.**
