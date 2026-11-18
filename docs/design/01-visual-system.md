# Visual design system — Vesper Chess

## Creative direction

**Quiet, cinematic, tactile terminal luxury.** Inspired by the visual confidence of polished TUIs: deliberate spacing, compact navigation, restrained chroma, precise alignment, meaningful color. Dark-first, not neon cyberpunk. Chessboard should be the unmistakable hero.

**Reference behavior:** JiraTUI-like left rail, status pills, focus highlights, bottom command hints, and compact information hierarchy. Do not copy its brand/layout pixel-for-pixel or overload the chess screen with irrelevant columns.

## Composition principles

1. **Hero board:** board owns the center and visual focus. Panels support it, never compete.
2. **One accent, many neutrals:** reserve color for selections, turn status and outcomes.
3. **Negative space is functional:** whitespace around the board improves visualization of moves.
4. **Dense but breathable:** no excessive box nesting; use separators and alignment more than heavy borders.
5. **State over decoration:** visual treatment must communicate meaning.
6. **Terminal-aware:** narrow screens collapse support panels, never the board input.

## Proposed theme tokens (starting palette, tune after terminal screenshots)

| Token | Dark (suggested) | Purpose |
|---|---|---|
| `canvas` | `#14151C` | outer background |
| `panel` | `#1C1E28` | side and detail surfaces |
| `panel-raised` | `#262A37` | overlays/selected rows |
| `border` | `#373B4B` | faint separators |
| `text` | `#E8E9F1` | primary readable text |
| `muted` | `#9DA4B7` | secondary information |
| `accent` | `#AFA1FF` | focus / primary interactive |
| `positive` | `#91CDA4` | connected / valid |
| `warning` | `#E8C17C` | pending / check |
| `danger` | `#EC8E9D` | errors / checkmate |
| `square-light` | `#A5ABB7` | light board squares |
| `square-dark` | `#586171` | dark board squares |
| `square-selected` | `#B8A8FF` | selected-source highlight |
| `square-legal` | `#A9D5B1` | move destinations |

**Accessibility:** Evaluate actual foreground/background combinations in the terminal; colors and glyph appearance vary by emulator. Don't assume these suggested hex values pass all contrast cases. Use symbols, labels, borders and reverse attributes as redundant signals.

## Chess-piece rendering

- Unicode symbols: White `♔♕♖♗♘♙`, Black `♚♛♜♝♞♟`.
- Some fonts render filled and outlined glyphs confusingly: use **contrasting piece foreground** and optional bold weight; verify against both square colors.
- Offer ASCII fallback `K Q R B N P` / `k q r b n p` and no-color mode.
- Every square has fixed cell width (suggest **3 characters** minimum) with rank/file coordinate labels.
- Highlights: cursor border or reverse video; selected square with distinct accent and symbol; legal destination with dot `·` or marker; last move with subtle edge marker, priority underneath cursor/selection.

## Typographic hierarchy

- App identity: small glyph/wordmark `♞ VESPER`, never a giant ASCII banner.
- Screen title: bold + restrained accent; panel labels: uppercase/muted with tracking simulated by spacing.
- Main game status: prominent short sentence (`YOUR MOVE`, `OPPONENT THINKING`, `CHECK`).
- Move history: monospaced SAN columns; visible move numbers and current ply emphasis.
- Status bar: compact keyboard cues with bracketed keys, e.g. `[↵] Select  [/] Command  [?] Help`.

## Micro-interactions (functional, not gratuitous)

- Cursor update immediate (<1 frame); transition smooth through state changes, not distracting animation.
- Valid destination appears as subtle highlights, selected square stays anchored.
- Sending move shows `Sending…`; rejection gives concise reason and keeps selection when possible.
- On check: king square accented + `CHECK` badge; no flashing animation.
- On game completion: calm result overlay with `Review board` option; never obscure the outcome indefinitely.
- On disconnect: connection badge changes with copy + retry progress; board stays readable as read-only.

## Responsive rules

| Width | Layout |
|---|---|
| ≥ 120 columns | Sidebar + centered board + right context panel + bottom hints |
| 90–119 | Compact sidebar + board; move history collapses below/right |
| 80–89 | Board-centered, hide non-essential navigation, toggle details with Tab |
| < 80 or height < 24 | Explicit compact/unsupported message with actionable resize hint; no clipped, unusable board |

Exact thresholds depend on font/cell sizing and must be verified visually. Never assume a monospace font guarantees chess glyphs occupy one cell; provide fallback.

## Visual acceptance checklist

- Board stays visually centered and has consistent aspect/cell spacing at supported sizes.
- Current turn, cursor, selected source, legal moves, last move and check have **different** readable signals.
- Focused pane is distinguishable without color alone.
- Empty/loading/error/reconnect/final states are as deliberately designed as the normal game.
- No status bar overflow or cut-off labels at the minimum supported size.
- Dark theme is consistent across screens and dialogs; optional light variant is post-MVP.
- Capture and review terminal screenshots for each major screen before calling UX done.
