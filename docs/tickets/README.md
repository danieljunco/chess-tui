# Epic CHESS-00 — Build Vesper Chess

**Type:** Epic · **Goal:** A beautiful, live, multiplayer chess TUI built in Rust/Ratatui. Two humans, private untimed games, keyboard-first gameplay.

**Product acceptance:** See `docs/concept/01-product-vision.md` and `docs/quality/01-acceptance-and-testing.md`.

## Task graph

```text
CHESS-01 Discovery
   ├── CHESS-02 Chess domain ─────────┐
   └── CHESS-03 Mock TUI shell ──────┼── CHESS-04 Local play ─┐
                                    └── CHESS-05 Host session ┼── CHESS-06 Peer sync
                                                                └── CHESS-07 UX polish
                                                                         └── CHESS-08 Release
```

CHESS-02 and CHESS-03 can progress in parallel after CHESS-01. CHESS-05 can start after CHESS-02 while CHESS-04 proceeds. Quality tests accompany **every** ticket, not only CHESS-08.

## Milestones

**M0 — Concept validated:** CHESS-01 docs and decisions complete.

**M1 — Visually compelling prototype:** CHESS-03 runs with mock state; primary visual gate passed.

**M2 — Local chess:** CHESS-02 + CHESS-04 support pass-and-play.

**M3 — Live P2P chess:** CHESS-05 + CHESS-06 let two terminals play directly (one hosts, one joins), no central server.

**M4 — Release candidate:** CHESS-07 + CHESS-08 pass visual and correctness gates.

## Ticket conventions

Each ticket is a standalone handoff: objective, concepts, scope, tasks, acceptance criteria, test evidence, and dependencies. Implement smallest runnable slice; update progress log.
