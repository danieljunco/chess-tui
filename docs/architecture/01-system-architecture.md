# Architecture — embedded host-peer

> Supersedes the earlier client/server proposal. See [ADR-001](../decisions/ADR-001-p2p-host-peer-topology.md) for why, and [ADR-006](../decisions/ADR-006-layout-terminal-and-crates.md) for the module layout.
>
> **Non-negotiable.** There is no server binary and no hosted backend. `vesper host` listens inside the player's process; `vesper join` dials that address. `vesper demo` is the only mock, and it is not the default.

## Shape: one binary, two roles

Both players run the same `vesper` binary. The player who **hosts** embeds a TCP listener and the authoritative game session inside their terminal process. The player who **joins** connects directly to that address. There is no separate server, no cloud service, no accounts, no database.

```text
           PLAYER A — `vesper host`                         PLAYER B — `vesper join 192.168.1.20:4747 K7QF-M2XA`
┌──────────────────────────────────────────┐            ┌──────────────────────────────────────────┐
│ Crossterm input → App.update → Ratatui   │            │ Crossterm input → App.update → Ratatui   │
    10|│            │ commands     ▲ GameView    │            │            │ commands     ▲ GameView    │
│            ▼              │               │            │            ▼              │               │
│        HostProvider ──────┘               │            │        PeerProvider ──────┘               │
│            │ (same path as remote seat)   │            │            │ replays snapshot with domain │
│            ▼                              │            │            │ (hints/SAN only, no commits) │
│   Session  ◄── authoritative ──┐          │            │            │                              │
│   (domain: shakmaty wrapper)   │          │            │            │                              │
│            ▲                   │          │   TCP      │            ▼                              │
│   net::host task (tokio) ◄─────┴──────────┼◄──NDJSON──►┼── net::peer task (tokio, reconnects)      │
└──────────────────────────────────────────┘            └──────────────────────────────────────────┘
    20|```

## Module boundaries

| Module | Responsibility | May depend on |
|---|---|---|
| `domain` | Position, legal moves, SAN/FEN/UCI, outcomes, repetition/move-count draws | `shakmaty` |
| `session` | Seats, phase, version, request-id cache, resign/claim, authorization. Pure, synchronous, deterministic | `domain` |
| `protocol` | Wire DTOs, `deny_unknown_fields`, bounds | `domain` types |
| `net` | Listener, handshake, heartbeats, reconnect/backoff, bounded channels | `session`, `protocol`, tokio |
| `app` | Screens, focus, selection, dialogs, toasts; `GameProvider` seam (Mock/Local/Host/Peer) | `domain` (read-only helpers) |
    30|| `ui` | Rendering only, from `app` state + theme | `app`, ratatui |

The UI never sees sockets; it only receives `GameView` values (board, history, outcome, seat, connection facts) and emits `GameCommand`s.

## Event loop (both roles)

```text
Key / Resize / Tick(60 ms) / ProviderEvent
        │
        ▼
    40|  App.update(event)  ──► effects: GameCommand → provider (non-blocking channel send)
        │
        ▼
  Ratatui draw (only when state changed or on tick animation)
```

- The terminal thread never awaits network I/O. Providers run on a Tokio runtime and communicate through bounded `mpsc` channels; the event loop polls crossterm with a short timeout and drains provider events with `try_recv`.
- Optimistic UI is limited to `Sending e2 → e4…`; the board changes only when a committed `GameView` arrives.
- Peer applies snapshots with `version >= current`; older ones are stale and dropped.
    50|- Cursor, selection, orientation and focus are UI state and survive snapshot refreshes.

## Host responsibilities

- Generate invite code + seat tokens (ADR-003); accept one peer for the open seat.
- Own the `Session`: validate seat, phase, turn, `expected_version`, legality; increment version once per accepted mutation; evaluate outcome.
- Treat its local player exactly like the remote one (same `Session::apply(seat, command)`).
- Broadcast a full snapshot after every accepted mutation; answer `sync` requests.
- Track peer liveness (heartbeat) and expose `opponent_online` in the host's own view.
- Bound resources: one peer connection, one pending handshake, 64 KiB frames, bounded outbound queue (on overflow, drop the connection; the peer resumes and resyncs).
    60|
## Peer responsibilities

- Connect, perform `hello` (invite code) or `resume` (seat token) handshake.
- Submit intentions (`move`, `resign`, `claim_draw`) with `request_id` + `expected_version`; at most one pending.
- Replay snapshots with the domain and verify the FEN; derive hints/SAN/captures locally.
- Detect loss (6 s silence), show `Reconnecting…`, retry with capped backoff (0.5 s → 5 s), resume with seat token, resync.

## Connection lifecycle

```text
    70|HOST:  Listening(invite shown) ──peer hello ok──► Active ──peer silent/closed──► Active(opponent offline)
                                                       ▲                                   │
                                                       └──────────── resume(token) ────────┘
       any state ──host quits──► game ends (peers see "Host closed the game")

PEER:  Connecting ──welcome──► Active ──timeout/EOF──► Reconnecting ──resumed──► Active
           │                                 │
           └─ bad code / full / version ──► Failed (designed error screen)        Finished: no more moves
```

## Reachability (be honest)
    80|
MVP supports **direct reachable addresses**: same LAN, VPN/overlay networks (Tailscale, WireGuard, ZeroTier), SSH tunnels, or manual port forwarding. Arbitrary internet peers behind NAT are **not** supported; NAT traversal, relays and public matchmaking are out of scope unless explicitly approved. Traffic is unencrypted in the MVP (ADR-002).

## Technology

Rust 2024, Ratatui 0.30, Crossterm 0.29, Tokio, tokio-util (LinesCodec), Serde/serde_json, shakmaty 0.30, clap. No database; game state lives in the host process.
