# CHESS-06 — Joining peer & live synchronization

**Priority:** P1 · **Depends on:** CHESS-04, CHESS-05 · **Type:** client/network

> Replaces the former "Live multiplayer client" ticket: the "client" is now the joining peer running the same binary.

## Outcome

Two terminals — host and peer — reliably play the same game over a direct connection without blocking either TUI.

## Concepts

Async effects, pending command state, snapshot replay + FEN verification, version reconciliation, reconnect/backoff, idempotent retries, connection UX.

## Work

- `vesper join <addr> [code]` and the in-app Join form (address + code validation).
- Keep the seat token in memory only; never log or render it.
- Send move intentions with `request_id`/`expected_version`, show pending UX, commit only from host snapshots.
- Replay snapshots with the shared domain, verify FEN, drop stale versions.
- Detect loss (heartbeat), show `Reconnecting…`, capped backoff, resume with token, reconcile pending moves from the snapshot.
- Show opponent/host connection state truthfully; `Host closed the game` terminal state.
- Two-process manual test and an automated two-peer in-process test.

## Acceptance

- [ ] Two terminals stay synchronized after every accepted move.
- [ ] Network waits never block keyboard navigation/rendering on either side.
- [ ] Rejected moves give useful feedback, no phantom board changes.
- [ ] Disconnect/reconnect restores correct board, turn, history, seat.
- [ ] Both terminals receive the result and cannot play afterwards.
