# CHESS-04 — Keyboard-first local gameplay

**Priority:** P1 · **Depends on:** CHESS-02, CHESS-03 · **Type:** client/application

## Outcome

A full pass-and-play chess match on one terminal, with proper rules and polished interactions.

## Concepts

Unidirectional app state, UI vs domain state, selection state machine, promotion dialog, input modes, SAN move history, result presentation.

## Work

- Wire board cursor → select source → show legal destinations → target → move command.
- Support `/` coordinate entry (`e2e4`, `e7e8q`), valid input errors and cancellation.
- Implement promotion selection, turn status, check/checkmate/stalemate, resign confirmation and move log.
- Preserve navigation/focus on resize and after invalid moves.
- Make pass-and-play an explicit local mode (`vesper local`); host and peer modes later use the same widgets.
- Add input/state tests and full-game scenario fixtures.

## Acceptance

- [ ] Two people can play a full local match solely with keyboard.
- [ ] Bad moves and wrong turns give helpful errors and never change board.
- [ ] Promotion to Q/R/B/N and end states work.
- [ ] SAN history and board remain consistent.
- [ ] UX behavior follows `docs/design/02-interaction-model.md`.
