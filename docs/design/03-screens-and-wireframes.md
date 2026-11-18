# Screen specifications & ASCII wireframes

> These are layout **concepts**, not pixel-perfect renders. Ratatui agent must refine with live screenshots and test in real terminal widths. Aesthetic quality is an acceptance criterion.

## Screen A: Home / launcher

```text
╭──────────────────────────────────────────────────────────────────────╮
│ ♞ V E S P E R                                          v0.1         │
│──────────────────────────────────────────────────────────────────────│
│                                                                      │
│                        PLAY SOME CHESS                               │
│                    ─────────────────────                             │
│                                                                      │
│                    ▸  Host a game                                    │
│                       Join a game                                    │
│                       Local game · pass & play                       │
│                       How to play                                    │
│                                                                      │
│                PEER TO PEER · LIVE · IN YOUR TERMINAL               │
│                                                                      │
│──────────────────────────────────────────────────────────────────────│
│ [↑↓] Navigate    [↵] Select    [?] Help    [q] Quit                 │
╰──────────────────────────────────────────────────────────────────────╯
```

## Screen A2: Join form

Two fields — `Host address` (`192.168.1.20:4747`, port optional) and `Invite code` (`K7QF-M2XA`, case/dash-insensitive). Inline validation, `[Tab]` between fields, `[↵] Connect`, `[Esc] Back`. While connecting: `◌ Connecting to 192.168.1.20:4747…` with the form read-only; failures show a specific reason (`Host unreachable`, `Invite code not accepted`, `Game already full`).

## Screen B: Waiting room (host)

```text
╭──────────────────────────────────────────────────────────────────────╮
│ ♞ VESPER   /   HOSTING                                  ● LISTENING │
│──────────────────────────────────────────────────────────────────────│
│                                                                      │
│                      YOUR TABLE IS READY                             │
│                                                                      │
│                       ╭──────────────────╮                           │
│                       │    K7QF-M2XA     │                           │
│                       ╰──────────────────╯                           │
│              Ask your friend to run                                  │
│              vesper join 192.168.1.20:4747 K7QF-M2XA                 │
│                                                                      │
│              Same network, forwarded port, or VPN · encrypted       │
│                       ◌ Waiting for Black…                          │
│──────────────────────────────────────────────────────────────────────│
│ [Esc] Stop hosting    [?] Help                                       │
╰──────────────────────────────────────────────────────────────────────╯
```

## Screen C: In-game, wide terminal

```text
╭───────────────────────────────────────────────────────────────────────────────────────────────────╮
│ ♞ VESPER   /   HOSTING · 192.168.1.20:4747                            ● LIVE · 24 ms             │
├──────────────────┬─────────────────────────────────────────────────┬──────────────────────────────┤
│  YOUR GAME       │                                                 │  OPPONENT                    │
│                  │                  BLACK · ALICE                  │  Alice         ● Connected   │
│  ● YOUR MOVE     │                                                 │                              │
│  White           │        a  b  c  d  e  f  g  h                  │  MOVE HISTORY                │
│                  │     8  ♜  ♞  ♝  ♛  ♚  ♝  ♞  ♜  8               │  1. e4      e5               │
│  LAST MOVE       │     7  ♟  ♟  ♟  ♟  ♟  ♟  ♟  ♟  7               │  2. Nf3     Nc6              │
│  e7 → e5         │     6  ·  ·  ·  ·  ·  ·  ·  ·  6               │  3. Bb5     a6               │
│                  │     5  ·  ·  ·  ·  ♟  ·  ·  ·  5               │                              │
│  ACTIONS         │     4  ·  ·  ·  ·  ♙  ·  ·  ·  4               │  CAPTURED                    │
│  New opponent    │     3  ·  ·  ·  ·  ·  ♘  ·  ·  3               │  White: —                    │
│  Resign          │     2  ♙  ♙  ♙  ♙  ·  ♙  ♙  ♙  2               │  Black: —                    │
│                  │     1  ♖  ♘  ♗  ♕  ♔  ♗  ·  ♖  1               │                              │
│                  │        a  b  c  d  e  f  g  h                  │  STATUS                      │
│                  │                                                 │  Last accepted: 3…a6         │
│                  │                  WHITE · YOU                   │                              │
├──────────────────┴─────────────────────────────────────────────────┴──────────────────────────────┤
│ [↑↓←→] Cursor   [↵] Select   [/] Move by coordinates   [Tab] Focus   [?] Help   [r] Resign        │
╰───────────────────────────────────────────────────────────────────────────────────────────────────╯
```

Notes: This wireframe is illustrative; pieces shown are not guaranteed to correspond to the textual move history. **Real UI must render board and history from the same authoritative state.** Do not include irrelevant actions such as `New opponent` unless actually implemented; use `Resign` (and `Claim draw` only when claimable) in MVP.

## Screen D: Promotion modal

```text
             ╭──────────────────────────────────────╮
             │  PROMOTE YOUR PAWN                   │
             │  Choose a piece for e7 → e8          │
             │                                      │
             │    [Q] ♕ Queen     [R] ♖ Rook        │
             │    [B] ♗ Bishop    [N] ♘ Knight      │
             │                                      │
             │  [↵] Confirm     [Esc] Cancel         │
             ╰──────────────────────────────────────╯
```

## Screen E: Connection lost

- Board remains visible but read-only.
- `● LIVE` changes to `○ RECONNECTING` in a warning tone.
- Nonmodal status strip: `Connection lost · Reconnecting · Last synced move 12`.
- When reconnected, request snapshot, reconcile pending move, show `Back online`.
- On the host, a dropped peer shows `○ OPPONENT OFFLINE`; the host keeps playing its own turn if it is its move.
- If the host quits, the peer sees a final `Host closed the game` overlay (game cannot resume).

## Screen F: Game result

- Keep complete board behind a compact overlay.
- Clearly identify winner, reason (`Checkmate`, `Resignation`, `Stalemate`, `Draw by repetition`, etc.).
- `Review board`, `Back to home`, `Quit` controls.

## Board-state visual stack (highest priority first)

1. Cursor focus (explicit outline/reverse).
2. Selected source (accent).
3. Legal destinations (marker/square treatment).
4. King in check (warning and label).
5. Last-move origin/destination (subtle tint).
6. Default alternating squares.

Conflicts must be resolved consistently; do not hide the cursor behind last-move color.
