# ADR-002: Direct TCP with newline-delimited JSON frames
Status: Accepted
Date: 2026-11-18

## Context
Two terminal processes must exchange a handful of small, ordered messages (moves, snapshots, heartbeats). There is no browser client. The original spec chose WebSocket because it assumed a server reachable over HTTP infrastructure.

## Options
| Option | Pros | Cons |
|---|---|---|
| WebSocket (`tokio-tungstenite`) | Framing built in, proxies/HTTP friendly | HTTP upgrade handshake and extra deps buy nothing peer-to-peer; still needs NAT traversal |
| **Raw TCP + NDJSON** (`tokio` + `tokio-util::LinesCodec`) | Ordered, reliable, trivially debuggable (`nc`), bounded line length built in, minimal deps | Hand-written handshake; no TLS by default |
| QUIC (`quinn`) | Encryption, multiplexing, better NAT hole-punching story | Certificates/identity UX, heavier; premature without NAT traversal scope |
| libp2p / iroh | NAT traversal + relays | Relays are hosted infrastructure; large dependency surface; out of MVP scope |

## Decision
Raw TCP, one JSON object per line (`\n`-terminated UTF-8), `protocol_version` in the handshake. `LinesCodec::new_with_max_length(64 KiB)` bounds every frame; oversized lines close the connection. TCP already guarantees ordering, so message ordering is guaranteed per connection; cross-connection ordering (reconnects) is handled by state versions (ADR-004).

Default port **4747**, configurable with `--port`. The host binds `0.0.0.0` (LAN reachable) unless `--bind` is given, and displays its detected LAN address(es).

## Security posture
- **Transport encryption is a Noise_XX handshake** (`snow`, `Noise_XX_25519_ChaChaPoly_SHA256`) on the TCP connection before any JSON frame. The host keeps a static key for the process and shows its fingerprint on the waiting screen. A guest who passes that fingerprint refuses a different host. The seat token and the moves travel inside the encrypted session.
- This does not cross NAT. A friend outside the LAN still needs a forwarded TCP port or a VPN address. A relay is not part of this decision.
- Untrusted input: every frame size-bounded, strictly parsed (`deny_unknown_fields`), handshake must complete within 5 s, failed invite attempts are delayed ≥1 s and only one handshake is processed at a time.

## Consequences
- Zero infrastructure, small dependency set, easy integration testing over `127.0.0.1`.
- Internet play requires reachable addresses; NAT traversal is deliberately not claimed.
