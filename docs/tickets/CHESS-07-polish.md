# CHESS-07 — Visual polish, usability & accessibility

**Priority:** P1 · **Depends on:** CHESS-04, CHESS-06 · **Type:** UX/UI quality

## Outcome

A **deliberately aesthetic, coherent, and delightful** production-quality terminal experience—not a generic colored grid.

## Concepts

Terminal contrast and color capabilities, visual hierarchy, typography scale through text attributes, pane focus, noncolor state communication, board orientation, latency feedback.

## Work

- Audit every screen against `docs/design/01-visual-system.md` and wireframes.
- Tune board proportions, whitespace, panel widths, accents, iconography, separators, and status pills.
- Ensure consistent highlight hierarchy for cursor, selected, legal, last move and check.
- Refine SAN history, captured pieces, connection/pending indicators and overlays.
- Improve help, empty states, invalid move copy, resign confirmation and responsive mode.
- Add/review Unicode glyph and ASCII/no-color fallback.
- Capture screenshots at target sizes, compare to visual spec, document changes.

## Acceptance

- [ ] No unreadable/misaligned board or panel at supported sizes.
- [ ] All interaction states legible and distinguishable without relying on color alone.
- [ ] Piece colors and terminal symbols readable in realistic terminal emulators.
- [ ] Keyboard workflow discoverable; no accidental sends/resignations.
- [ ] Visual reviews documented; no placeholders remain in core flows.
