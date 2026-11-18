# ADR-001: Embedded host-peer topology (no central server)
Status: Accepted
Date: 2026-11-18

## Context
The original specification described a separately deployed, authoritative WebSocket server (`chess-server`, CHESS-05), `wss://` remote deployment, a server-issued join code, and a "server restart" failure mode. The product requirement is now explicit: Vesper Chess is **peer-to-peer**. Both players run the same binary; no mandatory central game server, cloud infrastructure, accounts, database or hosted chess API may be required to play.

We still need exactly one source of truth so the two boards can never diverge.

## Options
1. **Symmetric P2P (both peers validate, consensus on each move).** No single authority, but conflicting mutations (simultaneous resign + move, divergent clocks) need a consensus protocol. Over-engineered for two players.
2. **Central server (original spec).** Simple authority, but violates the P2P requirement and needs deployment.
3. **Embedded host-peer.** The player who runs `vesper host` embeds a small TCP listener and the authoritative session inside their own process. The joining peer submits *intentions* and renders the host's snapshots.

## Decision
Option 3. The **host peer is authoritative** for seats, turn order, legality, versions and outcomes. The host's own UI goes through exactly the same `Session` command path as the remote peer's commands (no host shortcut), so authorization logic is tested once.

Both peers link the same chess domain. The joining peer uses it only to *derive* presentation data (legal destination hints, SAN, captured pieces) from the host's snapshot; it never commits a move locally.

## Consequences
- No deployment: `vesper host` / `vesper join <addr>` is the entire setup.
- The host's process lifetime *is* the game lifetime. If the host quits or crashes, the game ends (in-memory only, as in the original MVP). The joining peer may disconnect and resume while the host is alive.
- The host is trusted by definition: a modified host binary can cheat. Acceptable for private games between friends; documented in README.
- **Reachability is the user's responsibility in the MVP.** It works on a LAN, over a VPN/overlay (Tailscale, WireGuard, ZeroTier), via an SSH tunnel, or with a manually forwarded port. We do **not** claim that arbitrary internet peers behind NAT can connect. NAT traversal, relays and matchmaking are out of scope unless explicitly approved (see ADR-002).
- Original tickets CHESS-05/06 are rewritten as "embedded host session" and "peer synchronization".
