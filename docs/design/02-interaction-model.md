# Interaction model & keyboard map

## Primary modes

`Home` → `Host (Waiting)` / `Join form` / `Local` → `Playing` → `Finished`.

Within `Playing`, explicit focus states: `Board`, `Move History`, `Actions`, `Dialog`, `Command Input`. **Typing commands must not trigger board shortcuts.**

## Keyboard map (recommended)

| Key | Context | Action |
|---|---|---|
| `↑ ↓ ← →` or `h j k l` | Board | Move square cursor |
| `Enter` / `Space` | Board | Select piece, then select destination |
| `Esc` | Board | Clear current selection |
| `Tab` / `Shift+Tab` | Playing | Cycle focus between board/history/actions |
| `/` | Non-input | Open coordinate move / command entry |
| `e2e4` + `Enter` | Command entry | Submit coordinate move (or `e7e8q`) |
| `?` | Non-input | Toggle help overlay |
| `r` | Playing | Open resignation **confirmation**, never resign immediately |
| `o` | Playing | Flip orientation (display-only) |
| `g` | Playing | Toggle Unicode / ASCII piece glyphs |
| `d` | Playing | Claim draw — shown only when threefold / 50-move is claimable; asks for confirmation |
| `q` | Home / Finished | Quit; while playing, confirm exit/disconnect semantics |
| `Esc` | Dialog | Cancel / close |

**Conflict rule:** input widgets consume normal characters; global actions should be Esc/explicit modifiers, not single-letter shortcuts while typing.

## Move selection flow

1. Cursor on board square.
2. Enter: if own piece and player's turn, mark selected source and display legal destinations.
3. Cursor moves to destination; Enter requests the move. Illegal destination gives non-destructive feedback.
4. If promotion possible, open **modal with Q/R/B/N choices** before sending.
5. UI shows pending state; the host peer's result determines the committed position.
6. Accepted → update board/history; rejected → show reason, reconcile snapshot as necessary.

`/` coordinate mode is a first-class alternative, not a hidden developer feature. Validate input and show helpful examples; never submit malformed coordinates.

## Focus contract

- Focus rail visible (accent bracket / line + bold label).
- Only one pane receives directional keys at a time.
- Changing focus does not clear move selection unless entering an action that explicitly cancels it.
- Screen resize preserves focus and selection when valid.
- Modal captures focus completely; Esc always exits safely.

## Important messages

| Situation | User-facing copy |
|---|---|
| waiting (host) | `Waiting for an opponent · vesper join 192.168.1.20:4747 K7QF-M2XA` |
| connecting (peer) | `Connecting to 192.168.1.20:4747…` |
| opponent dropped (host) | `Opponent offline · the game is kept while you stay` |
| host gone (peer) | `Host closed the game` |
| your turn | `Your move · White` |
| opponent's turn | `Waiting for Black's move` |
| submitting | `Sending move e2 → e4…` |
| rejected turn | `Not your turn` |
| rejected illegal | `Illegal move · choose a highlighted square` |
| check | `Check · protect your king` |
| reconnect | `Connection lost · reconnecting…` |
| completed | `Checkmate · White wins` (or actual result) |

## Usability edge cases

- Board coordinates rotate with perspective; UCI coordinate entry always uses canonical algebraic squares.
- On opponent move, never abruptly move user's cursor; highlight last move.
- Disconnected / waiting players cannot submit moves.
- Resignation requires affirmative confirmation; show effect in opponent's client immediately.
- When no Unicode support: use ASCII glyphs and outlined selection/cursor.
- Notifications are brief, but important errors remain accessible in a log/status panel.
