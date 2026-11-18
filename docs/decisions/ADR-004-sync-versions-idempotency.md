# ADR-004: Snapshot synchronization, versions and idempotency
Status: Accepted
Date: 2026-11-18

## Context
The peer must never show a move the host did not commit, must recover after a reconnect, and must not double-apply retried commands.

## Options
- Event deltas with gap detection — compact but needs replay buffers and gap repair.
- **Full snapshot on every committed change** — a whole game is a few KB (≤ ~600 plies × short UCI strings), well under the 64 KiB frame bound.

## Decision
- The host's `Session` holds a monotonically increasing `version: u64` (starts at 1 when both seats are filled; +1 per accepted mutation: move, resignation, draw claim).
- After every accepted mutation, and in response to `hello`/`resume`/`sync`, the host sends a **full snapshot**: version, phase, seats, start FEN, the UCI move list, current FEN, outcome, and peer-connectivity facts.
- The peer **replays** the UCI list with the shared domain from the start FEN and checks the resulting FEN equals the snapshot FEN. Mismatch ⇒ protocol error, drop connection, reconnect (never silently trust a broken state).
- Peer applies a snapshot only if `snapshot.version >= local.version`; older snapshots are stale and ignored.
- Every peer command carries `request_id` (u64, unique per peer process) and `expected_version`. The host:
  - answers a repeated `request_id` with the **same cached result** (accepted or rejected), never re-applying (cache of last 64 results per seat);
  - rejects `expected_version != version` as `stale_version` and attaches a fresh snapshot.
- The peer keeps at most **one pending command**. On disconnect the pending command stays "unresolved" in the UI; after `resume`, the fresh snapshot decides whether it was applied (pending move is cleared either way; if the move appears in history it was accepted).
- **Heartbeat:** each side sends `ping` every 2 s; 6 s without any inbound frame ⇒ connection considered lost. RTT is shown in the header.
- **Disconnect policy (untimed MVP):** no automatic forfeits. Host shows `Opponent offline`; the game is retained while the host process lives. The host may still make its move if it is its turn.

## Consequences
Simple, self-healing synchronization at the cost of slightly larger frames — irrelevant for chess. Ordering within a connection is TCP's; across reconnects it is the version.
