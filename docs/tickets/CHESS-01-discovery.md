# CHESS-01 — Technical discovery & architecture

**Priority:** P0 · **Depends on:** none · **Type:** research/decisions

## Outcome

Document the technology, architecture, wire protocol, and aesthetics strategy in enough detail to implement without guessing.

## Concepts

Ratatui render loop, terminal cell geometry, event-driven UI, chess legality vs adjudication, FEN/SAN/UCI, direct peer connections, reconnection and host-peer authority.

## Work

- Read all product/design specs; inspect JiraTUI UX reference **as inspiration** if source access is available.
- Compare maintained Rust chess libraries: complete legality, FEN, SAN, move list, history/repetition and draws. Record gaps explicitly.
- Choose Ratatui/Crossterm, Tokio, peer transport (TCP vs WebSocket vs QUIC) and test approach; verify latest compatible versions at implementation time.
- Review every centralized-server assumption and realign to the embedded host-peer model; document NAT/reachability limits honestly.
- Decide transport handshake, seat credentials, join-code lifetime, idempotency and state snapshot.
- Choose workspace layout and minimum supported terminal width/height.
- Write ADRs in `docs/decisions/`; update architecture/protocol specs if choices differ.
- Identify and document image/screenshot testing process for TUI aesthetics.

## Acceptance

- [ ] Dependencies chosen with documented rationale and constraints.
- [ ] Rules engine coverage/limitations checked (especially draw adjudication).
- [ ] Protocol + recovery semantics agreed.
- [ ] Architecture and directory layout agreed.
- [ ] Visual quality gates and terminal fallback defined.
- [ ] No application logic prematurely implemented as part of research.
