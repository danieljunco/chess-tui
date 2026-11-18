#!/usr/bin/env python3
"""Drive the real `vesper` binary in a pseudo-terminal.

Checks: launches, renders the home screen, starts a local game, plays a
move by keyboard, quits on `q` and restores the terminal (leaves the
alternate screen, shows the cursor).
Usage: scripts/pty_smoke.py [path/to/vesper]
"""

import fcntl
import os
import re
import pty
import select
import struct
import sys
import termios
import time

BIN = sys.argv[1] if len(sys.argv) > 1 else "target/debug/vesper"
COLS, ROWS = 100, 32


def read_for(fd, seconds):
    out = b""
    end = time.time() + seconds
    while time.time() < end:
        r, _, _ = select.select([fd], [], [], 0.05)
        if r:
            try:
                chunk = os.read(fd, 65536)
            except OSError:
                break
            if not chunk:
                break
            out += chunk
    return out


CSI = re.compile(r"\x1b\[([0-9;?]*)([A-Za-z])")


def grid(stream):
    """Replay output onto a COLS×ROWS grid. ratatui only redraws changed
    cells, positioning with CSI row;col H, so the stream alone is not the
    screen. Clears on entering the alternate screen."""
    text = stream.decode("utf-8", "replace")
    cells = [[" "] * COLS for _ in range(ROWS)]
    row = col = 0
    i = 0
    while i < len(text):
        m = CSI.match(text, i)
        if m:
            params, final = m.groups()
            if final == "H":
                parts = [int(p) if p.isdigit() else 1 for p in params.split(";")] + [1, 1]
                row, col = parts[0] - 1, parts[1] - 1
            elif final == "h" and params == "?1049":
                cells = [[" "] * COLS for _ in range(ROWS)]
            i = m.end()
            continue
        ch = text[i]
        i += 1
        if ch == "\x1b":
            continue
        if ch in "\r\n":
            continue
        if 0 <= row < ROWS and 0 <= col < COLS:
            cells[row][col] = ch
        col += 1
    return "\n".join("".join(r) for r in cells)


def has(screen, needle):
    return needle.decode() in grid(screen)


def main():
    pid, fd = pty.fork()
    if pid == 0:
        os.environ["TERM"] = "xterm-256color"
        os.execv(BIN, [BIN, "local"])
    fcntl.ioctl(fd, termios.TIOCSWINSZ, struct.pack("HHHH", ROWS, COLS, 0, 0))
    checks = {}

    screen = read_for(fd, 1.0)
    checks["enters alternate screen"] = b"\x1b[?1049h" in screen
    checks["renders header"] = has(screen, b"VESPER")
    checks["local game shown"] = has(screen, b"PASS & PLAY")

    # e2 is the default cursor for White: select, go up twice, confirm.
    for key in [b"\r", b"k", b"k", b"\r"]:
        os.write(fd, key)
        screen += read_for(fd, 0.25)
    checks["move committed (1. e4 in history)"] = has(screen, b"1. e4")
    checks["last move shown"] = has(screen, b"1. e4  e2 \xe2\x86\x92 e4")
    checks["turn passed to Black"] = has(screen, b"BLACK TO MOVE")

    os.write(fd, b"?")
    screen += read_for(fd, 0.3)
    checks["help overlay opens"] = has(screen, b"HOW TO PLAY")
    os.write(fd, b"\x1b")
    read_for(fd, 0.3)

    os.write(fd, b"q")
    screen += read_for(fd, 0.3)
    checks["quit asks for confirmation"] = has(screen, b"LEAVE THE TABLE?")
    os.write(fd, b"y")
    screen += read_for(fd, 0.3)
    os.write(fd, b"q")
    tail = read_for(fd, 1.0)

    _, status = os.waitpid(pid, 0)
    checks["exits cleanly"] = os.WIFEXITED(status) and os.WEXITSTATUS(status) == 0
    checks["leaves alternate screen"] = b"\x1b[?1049l" in tail
    checks["shows cursor again"] = b"\x1b[?25h" in tail

    ok = True
    for name, passed in checks.items():
        ok &= passed
        print(f"{'PASS' if passed else 'FAIL'}  {name}")
    sys.exit(0 if ok else 1)


if __name__ == "__main__":
    main()
