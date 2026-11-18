# CHESS-05 — Embedded authoritative host session

**Priority:** P0 · **Depends on:** CHESS-01, CHESS-02 · **Type:** session/network

> Replaces the former "Authoritative multiplayer server" ticket. There is no separate server: the hosting player's `vesper` process embeds the session and a TCP listener ([ADR-001](../decisions/ADR-001-p2p-host-peer-topology.md)).

## Outcome

`vesper host` opens a direct listener, admits exactly one peer with the invite code, and authoritatively validates and broadcasts every move — for its own player and the remote one alike.

## Concepts

Embedded tokio listener, NDJSON framing, two-seat session, invite code vs seat token, command idempotency, optimistic concurrency via versions, single-task serialization, bounded queues.

## Work

- Pure `Session` (no I/O): seats, phase, version, `apply(seat, command)` for move/resign/claim draw, request-id result cache, typed rejections, snapshots.
- Host provider: the local player's commands go through `Session::apply` exactly like the peer's.
- `net::host`: bind (`--bind`, `--port`, default `0.0.0.0:4747`), detect and display LAN addresses, one handshake at a time with 5 s deadline, ≥1 s failure penalty, consume invite on join, resume with seat token, replace half-open connections.
- Heartbeat (2 s ping / 6 s timeout), opponent-online status in the host view, bounded outbound queue, graceful `host_closing` on quit.
- Integration tests over `127.0.0.1`: join, bad invite, second joiner refused, stale version, duplicate request ID, wrong turn, malformed/oversized frames, resume with good/bad token, finished game rejects moves.

## Acceptance

- [ ] Two seats maximum; invite code consumed on join; no invite-only takeover after join.
- [ ] Illegal/out-of-turn/stale/duplicate commands cannot corrupt state.
- [ ] Every accepted mutation broadcast with a monotonic version.
- [ ] Resume with the private token restores the seat and snapshot.
- [ ] Finished games reject moves; frame/queue bounds and structured errors implemented.
- [ ] No central server, relay or external service involved.
