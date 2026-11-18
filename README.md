# ♞ Vesper Chess

Two people play chess from their own terminals. One person hosts the game. The other connects straight to that terminal. There is no server to install and no account to create.

**You can play by building from source.** There is no Homebrew formula and no Arch package yet. You need [Rust 1.88 or newer](https://rustup.rs) and a terminal at least 80 columns by 24 rows. 100×32 or larger looks better.

```bash
git clone https://github.com/danieljunco/chess-tui.git
cd chess-tui
cargo install --path .
```

`cargo install` puts `vesper` on your `PATH` (usually `~/.cargo/bin`).

On a Mac, Homebrew does that job instead:

```bash
brew tap danieljunco/tap
brew install vesper
```

On Arch, including Omarchy, build the package from the recipe in this repo. It installs `/usr/bin/vesper` through pacman:

```bash
git clone https://github.com/danieljunco/chess-tui.git
cd chess-tui/packaging/arch
makepkg -si
```

Omarchy can install that same package once it is on the AUR (`omarchy pkg aur add vesper-chess`). The AUR entry is not published yet. The recipe above works today.

Then, in two terminals:

```bash
vesper host --name Ana
vesper join 127.0.0.1:4747 K7QF-M2XA --name Ben
```

Use the invite code shown on the host screen. `K7QF-M2XA` is only an example. The first build takes a few minutes.

To run from this folder without installing:

Terminal A, the host:

```bash
cargo run --release -- host --name Ana
```

The first build takes a few minutes. The screen then shows an invite code (`K7QF-M2XA` is only an example; use the one on your screen) and a join line.

Terminal B, the guest:

```bash
cargo run --release -- join 127.0.0.1:4747 K7QF-M2XA --name Ben
```

Replace the code with the one from Terminal A. The host plays White and moves first.

If port 4747 is already taken:

```bash
cargo run --release -- host --port 4748 --name Ana
cargo run --release -- join 127.0.0.1:4748 K7QF-M2XA --name Ben
```

## How to move

| Key | What it does |
|---|---|
| Arrow keys, or `h` `j` `k` `l` | Move the cursor |
| Enter | Pick up a piece, then put it on a marked square |
| `/`, then `e2e4`, then Enter | Type a move. Promotions look like `e7e8q` |
| `?` | Show every key |
| `q`, then `y` | Leave the table |

The other terminal updates when the move is accepted. An illegal move stays off the board and the screen says why.

## Play somewhere else on the network

Same two commands. The host screen prints the address to share, for example `192.168.1.20:4747`, plus the invite code. The guest must be able to open a TCP connection to that address: same Wi-Fi, a VPN (Tailscale, WireGuard, ZeroTier), an SSH tunnel, or a port you forwarded yourself.

```bash
cargo run --release -- join 192.168.1.20:4747 K7QF-M2XA --name Ben
```

Vesper does not punch through a home router on its own. The chess messages are encrypted. The host screen shows a key; pass it on the join command when the other player is not on your own machine.

Leaving the host ends the game. If the guest's connection drops, the game stays open while the host is still running, and the guest's app tries to reconnect by itself.

## Troubleshooting connections

For two laptops, join the **hosting laptop's LAN address**, such as
`192.168.1.104:4747`. `127.0.0.1` and `localhost` only reach the laptop running
the join command. Keep the host's waiting screen open and use its current invite code.

If joining times out, check TCP reachability from the guest (macOS):

```bash
nc -vz -G 5 192.168.1.104 4747
```

On the Linux host, confirm Vesper is listening:

```bash
ss -ltnp 'sport = :4747'
```

Expect `0.0.0.0:4747` for the default IPv4 listener. If it shows
`127.0.0.1:4747`, restart with `vesper host --bind 0.0.0.0 --port 4747`.
A refused connection usually means the address or port is wrong, the host has
stopped, or a firewall is rejecting connections. A timeout can mean a firewall
is dropping connections or the laptops cannot reach each other. Ping succeeding
does not prove TCP port 4747 is allowed.

### Omarchy / UFW firewall

[Omarchy blocks incoming connections by default](https://omarchy.org/manual/networking/).
On the **hosting laptop**, check the firewall and allow TCP 4747 from your guest's
network. For a home LAN using `192.168.1.0/24`:

```bash
sudo ufw status verbose
sudo ufw allow proto tcp from 192.168.1.0/24 to any port 4747
```

Replace the subnet with your actual LAN subnet, or use the guest's exact IP to
allow only that laptop. Use your chosen port if you changed `--port`. The rule
persists across reboots; Vesper's package leaves firewall policy under your control.
Retry joining with the host's invite code. To remove this rule later:

```bash
sudo ufw delete allow proto tcp from 192.168.1.0/24 to any port 4747
```

If the port still times out, check that both laptops are on the same LAN and
that the Wi-Fi network does not isolate guests or wireless clients. If TCP
connects but Vesper rejects the invite or handshake, use a fresh invite and
matching Vesper versions on both laptops.

## Other ways to start

```bash
cargo run --release -- local     # two people, one keyboard, no network
cargo run --release               # home menu. Host and Join are a real connection
cargo run --release -- demo      # a simulated opponent, for looking at the interface
cargo run --release -- --ascii   # letter pieces when chess symbols render badly
NO_COLOR=1 cargo run --release   # no colour. Brackets and markers carry the meaning
```

In demo mode, `x` previews a dropped connection. That key does nothing in a real game.

## Install the `vesper` command

`cargo install --path .` builds a release binary and copies it to `~/.cargo/bin`. Homebrew (`brew install vesper` after `brew tap danieljunco/tap`) and the Arch recipe in `packaging/arch` do the same install for their own package managers. There is no crates.io package yet. A local release build also leaves the binary at `target/release/vesper`.

## For contributors

The rest of this file is the product spec. Players can stop at the sections above.

## North star

**The elegance of a premium desktop chess app, the speed of a terminal, and the ergonomics of a great developer tool.** Imagine JiraTUI's dense-but-readable layout and refined navigation applied to a live chess board. Every state—hover, selection, check, network delay, victory—should feel intentional.

## Start here (for humans)

1. Read [product vision](docs/concept/01-product-vision.md) and [interaction model](docs/design/02-interaction-model.md).
2. Review the [visual system](docs/design/01-visual-system.md) and [screens](docs/design/03-screens-and-wireframes.md).
3. Review [architecture](docs/architecture/01-system-architecture.md), [domain](docs/concept/02-chess-domain.md), and [protocol](docs/architecture/02-realtime-protocol.md).
4. Follow the [implementation roadmap](docs/tickets/README.md) and [quality gates](docs/quality/01-acceptance-and-testing.md).

## Start here (for coding agents)

Paste **[prompts/IMPLEMENT_WITH_AGENT.md](prompts/IMPLEMENT_WITH_AGENT.md)** into your agent at the repository root. The prompt specifies discovery, implementation order, gates, testing, and non-negotiable aesthetic standards. **It allows implementation**, unlike a planning-only prompt.

Suggested agent entry point: `AGENTS.md` at repository root. If using Cursor, Codex, or Claude Code, include that file as project context.

## MVP in one sentence

Two people start the same app in separate terminals; one runs `vesper host`, the other connects **directly** with `vesper join <address> <code>` — no central server, accounts or cloud — and they play a complete *untimed* standard chess game using keyboard or coordinate entry, see synchronized moves and game results, and recover from a temporary disconnect.

## Ticket index

| Ticket | Outcome | Depends on |
|---|---|---|
| [CHESS-01](docs/tickets/CHESS-01-discovery.md) | Decisions, feasibility, executable plan | — |
| [CHESS-02](docs/tickets/CHESS-02-domain.md) | Correct chess domain and rules | 01 |
| [CHESS-03](docs/tickets/CHESS-03-shell.md) | Beautiful, fully navigable mock TUI | 01 |
| [CHESS-04](docs/tickets/CHESS-04-local-play.md) | Complete pass-and-play local game | 02, 03 |
| [CHESS-05](docs/tickets/CHESS-05-host-session.md) | Embedded authoritative host session | 01, 02 |
| [CHESS-06](docs/tickets/CHESS-06-peer-sync.md) | Joining peer, live two-terminal gameplay | 04, 05 |
| [CHESS-07](docs/tickets/CHESS-07-polish.md) | Premium UX and accessibility polish | 04, 06 |
| [CHESS-08](docs/tickets/CHESS-08-release.md) | Quality gates, docs, release | 02–07 |

## The experience

- Command bar and status rail inspired by advanced TUIs, not a web UI squeezed into a terminal.
- Crisp chessboard, strong square contrast, restrained accent colors, excellent typographic rhythm.
- Contextual keyboard hints and highly legible focus behavior.
- **Mock-first visual milestone** before networking; the aesthetics are part of the product, not an optional final coat.
- Graceful behavior on smaller terminals, no overlapping panels, no inaccessible moves.

## Fixed decisions vs discovery

**Fixed for MVP:** Rust, Ratatui, Crossterm, Tokio, two humans, **peer-to-peer with an authoritative hosting peer** (no central server), direct TCP transport, untimed games, address + invite-code join, no accounts, no AI.

**Decided in CHESS-01** (see [`docs/decisions/`](docs/decisions/)): host-peer topology (ADR-001), TCP plus a Noise_XX handshake and then JSON frames (ADR-002), invite code vs seat token (ADR-003), snapshot sync/versions/idempotency (ADR-004), shakmaty + explicit draw rules (ADR-005), single-package layout and terminal baseline (ADR-006).

**Connectivity:** direct reachable addresses only — same LAN, VPN overlay (Tailscale/WireGuard/ZeroTier), SSH tunnel or a forwarded port. NAT traversal and relays are out of scope; traffic is unencrypted in the MVP.

## Non-goals

Rated matchmaking, AI engines, chess clocks, spectators, chat, tournaments, accounts, permanent game storage, public matchmaking, central/relay servers, NAT traversal. See [scope](docs/concept/01-product-vision.md).
