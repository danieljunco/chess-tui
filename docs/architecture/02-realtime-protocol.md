# Peer protocol (design contract)

> Host ⇄ peer over direct TCP. Rationale: [ADR-002](../decisions/ADR-002-transport-tcp-ndjson.md) (transport), [ADR-003](../decisions/ADR-003-invite-code-and-seat-token.md) (credentials), [ADR-004](../decisions/ADR-004-sync-versions-idempotency.md) (synchronization).

## Transport

Plain TCP, default port `4747`. One UTF-8 JSON object per line (`\n`). Maximum line 64 KiB; longer frames close the connection. Unknown `type`s or fields are rejected (`deny_unknown_fields`). **No encryption in the MVP** — use trusted networks, VPN overlays, or SSH tunnels.

## Handshake (first frame from peer, within 5 s)
    10|
| Peer → host | Fields | Host reply |
|---|---|---|
| `hello` | `protocol_version`, `invite_code`, `name?` (≤ 24 chars) | `welcome {seat, seat_token, host_name}` + `snapshot`, or `refused {code, message}` |
| `resume` | `protocol_version`, `seat_token` | `welcome {seat, host_name}` (no token echo) + `snapshot`, or `refused` |

`refused.code`: `bad_version`, `bad_invite`, `bad_token`, `seat_taken`, `game_over`, `malformed`. Wrong credentials are answered after a ≥1 s delay.

## Peer → host commands (after handshake)

| Command | Fields | Result |
    20||---|---|---|
| `move` | `request_id`, `expected_version`, `uci` (`e2e4`, `e7e8q`) | `accepted {request_id, version}` + `snapshot`, or `rejected` |
| `resign` | `request_id`, `expected_version` | same |
| `claim_draw` | `request_id`, `expected_version` | same (only if threefold / 50-move currently claimable) |
| `sync` | — | `snapshot` |
| `ping` / `pong` | `nonce` | heartbeat, RTT |
| `bye` | — | graceful leave (host shows opponent offline) |

`rejected {request_id, code, message, version}` with `code` ∈ `not_your_turn`, `illegal_move`, `stale_version`, `game_over`, `not_claimable`, `malformed`. A rejection never mutates state. A stale/illegal rejection is followed by a fresh `snapshot`.

## Host → peer events
    30|
- `snapshot {version, phase, white, black, start_fen, moves: [uci], fen, outcome?, opponent_online}` — full authoritative state; sent after every accepted mutation and on demand.
- `host_closing {reason}` — host is quitting; the game is over.

`phase`: `active` | `finished`. `outcome`: `{result: "1-0"|"0-1"|"1/2-1/2", reason: checkmate|resignation|stalemate|insufficient_material|fivefold_repetition|seventy_five_moves|threefold_repetition|fifty_moves}`.

Connection status (`LIVE`, `RECONNECTING`, `OFFLINE`) is **derived locally** from heartbeats, never a game mutation.

## Example

```json
    40|{"type":"move","request_id":42,"expected_version":12,"uci":"e2e4"}
{"type":"accepted","request_id":42,"version":13}
{"type":"snapshot","version":13,"phase":"active","white":"Ana","black":"Ben","start_fen":"rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1","moves":["e2e4"],"fen":"rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1","outcome":null,"opponent_online":true}
```

## Protocol invariants

1. Exactly one accepted mutation per version; `version` increments monotonically.
2. A repeated `request_id` returns the cached result; it is never re-applied.
3. `expected_version` mismatch ⇒ `stale_version`; peer resyncs from the attached snapshot.
    50|4. Only the seat whose turn it is may move; either seat may resign; claims only by the side to move.
5. After `finished`, every mutation is rejected with `game_over`, even delayed frames.
6. `resume` requires the private seat token; the invite code is consumed after the first successful join.
7. The seat token is never sent to anyone except its owner, never logged, never rendered.
8. Untimed MVP: disconnection never forfeits. Host shows `Opponent offline`; game retained while the host process lives.

## Threat / failure model

- **Guessing invite codes / tokens:** 40-bit invite, 128-bit token, one handshake at a time, ≥1 s penalty per failure, 5 s handshake deadline.
- **Seat theft:** invite consumed on join; reconnect needs the token; a valid resume replaces any half-open old connection.
    60|- **Malformed / oversized / repeated frames:** strict schema, 64 KiB lines, request-id idempotency.
- **Drop during a move:** pending stays unresolved; the post-resume snapshot decides; the UI never guesses.
- **Simultaneous commands (host + peer):** the host serializes all `Session` mutations on one task; the version check rejects the loser.
- **Slow peer:** bounded outbound queue; overflow drops the connection, peer resumes and resyncs.
- **Host quits/crashes:** game is lost (in-memory). Peer shows `Host closed the game` / `Host unreachable`.
- **Eavesdropping / tampering on the network:** not mitigated in MVP (plaintext). Documented; Noise/QUIC upgrade path in ADR-002.
- **Malicious host:** can cheat by construction (it is the authority). Accepted for private games.
