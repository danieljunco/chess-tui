# CHESS-03 — Premium Ratatui shell & mock board

**Priority:** P0 · **Depends on:** CHESS-01 · **Type:** TUI/UX

## Outcome

**A beautiful, navigable terminal prototype before any networking.** This is a visual milestone, not a placeholder shell.

## Concepts

Ratatui composition, terminal buffer geometry, keyboard focus, stable render loop, reusable visual tokens, state-driven widgets, responsive layouts, Unicode/ASCII support.

## Work

- Build home, create/join form, waiting room, in-game layout, help, promotion and result overlay from `docs/design/`.
- Implement semantic theme tokens, restrained dark palette, board square rendering and file/rank labels.
- Use consistent mock chess state and meaningful SAN history; no contradictory fixture data.
- Implement board cursor, selected square, legal destination hints, last move, and focus indicators with mock data.
- Add keyboard navigation and resize behavior; no UI freezes or text clipping at supported sizes.
- Implement safe terminal enter/restore, panic/error cleanup where practical.
- Capture / snapshot major states; review aesthetics and iterate **before connecting networking**.

## Acceptance

- [ ] Mock app launches and keyboard navigation works.
- [ ] Board is visually dominant, legible and proportionate.
- [ ] Selected/cursor/legal/last move states distinguishable without color alone.
- [ ] Home, waiting, active, promotion, reconnecting, result and help visually reviewed.
- [ ] 120×36, 100×32, 90×28, 80×24 behavior validated; no clipped panels.
- [ ] ASCII fallback works; terminal cleans up on exit.
- [ ] Visual gate evidence logged in `docs/quality/02-progress-log.md`.
