# Product vision, personas & scope

## Product statement

**Vesper Chess** is an aesthetic, fast, live chess experience entirely inside the terminal. Inspired by the calm density and clear focus cues of high-quality TUIs (including the JiraTUI reference), while respecting chess-specific needs: an unmistakable board, turn ownership, and low-friction move entry.

## Target users and jobs

- **Terminal-native developer:** Play a full game without opening a browser; use keyboard shortcuts and easily read the position.
- **Remote friend:** Share an address and short code, connect directly to a private untimed game, always know whose turn it is.
- **New chess player:** See legal destinations, errors that explain why a move failed, and an accessible shortcut guide.

## Core journeys

1. **Host:** `vesper host` (or Home → `Host a game`) → the app opens a direct listener and shows its address plus a short invite code → friend joins → White moves first.
2. **Join:** `vesper join <address> <code>` (or Home → `Join a game` → enter address and code) → connect directly to the host's terminal → take the open seat → see orientation and status → play.
3. **Play:** Focus board → select source square → destinations are indicated → choose target → promotion dialog if needed → submit pending move → **host peer** accepts/rejects → board and history update on both terminals.
4. **Recover:** Network drops → UI indicates reconnecting without pretending a move succeeded → peer reconnects to the host with its private seat token → authoritative snapshot restores state → resume.
5. **Finish:** Checkmate, stalemate, agreed/claimable draw as supported, or resignation → clear result overlay and final board, no more moves.

## MVP commitments

- Standard chess starting position, legal moves, captures, castling, en passant, promotion, check/checkmate, stalemate.
- Draw handling with correctly documented claim/automatic behavior based on FIDE rules and supported library behavior.
- Two humans; untimed; one active game per process for MVP; host address + invite code.
- **Peer-to-peer:** both players run the same binary; the hosting peer is authoritative; no central server, cloud service, accounts or database.
- Real-time updates on both terminals; safe reconnect with seat ownership retained while the host is running.
- Local pass-and-play mode (`vesper local`) on one terminal.
- Resignation and visible final result; SAN move history; board orientation for both seats.
- Arrow/Vim-style board navigation plus algebraic coordinate entry (`e2e4`, `e7e8q`); contextual help.
- Loading, empty, waiting, in-game, error, reconnecting and complete states designed—not left to defaults.

## Explicit non-goals

AI opponents, engine evaluations, public listings, matchmaking, ratings, clocks, friend lists, spectators, chat, long-term storage, full screen-reader equivalence across all terminals, mobile-terminal optimization, **central game servers, relay servers and automatic NAT traversal** (unless explicitly approved), transport encryption (post-MVP, see ADR-002).

## Constraints & product choices

- Anonymous access is acceptable for MVP **but the invite code must not be the credential used to impersonate a player after joining**. The host issues a private, unguessable seat token; reconnect requires it (ADR-003).
- Invite codes are random (40 bits), single-use (consumed when the seat is filled) and throttled.
- The game lives in the host's process. If the host quits, the game ends (in-memory only). Clearly communicate this limitation.
- Connectivity: LAN, VPN overlay, SSH tunnel or forwarded port. We never claim that arbitrary peers behind NAT can connect.
- Support typical modern terminals. Provide ASCII fallback if chess glyphs render badly. Minimum supported viewport to be validated (proposed: 80×24, preferred: ≥100×32).
- Avoid relying on color alone; combine semantic symbols, labels, and focus styling.

## Success / failure signals

- Two peers finish a real game without desync, invalid turns, or unrecoverable navigation issues.
- Players never confuse selected square, legal destinations, last move, side to move, or peer connection state.
- On a 100×32 terminal, critical gameplay information is visible without clipping.
- A new user finds the help overlay and makes their first legal move without consulting source code.
