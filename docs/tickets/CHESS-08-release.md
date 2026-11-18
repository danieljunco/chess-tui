# CHESS-08 — Testing, documentation & release

**Priority:** P1 · **Depends on:** CHESS-02 through CHESS-07 · **Type:** QA/release

## Outcome

A reliable, reproducible first release that someone can run and understand.

## Concepts

Testing pyramid, async integration tests, release binaries, terminal restoration, protocol negative tests, visual regression, threat model.

## Work

- Finalize domain, session, host/peer, protocol and TUI tests; address flakiness.
- Complete manual two-terminal full-game and reconnect scenarios.
- Run checks: fmt, clippy, tests, release build; fix critical failures.
- Write install / host / join / local / keymap docs with actual commands.
- Describe reachability (LAN, VPN, SSH tunnel, port forwarding — no NAT traversal), plaintext-transport limitation, host-quit ends game, anonymity and credential handling.
- Provide sample config and development workflow without embedding secrets.
- Complete final screenshot/terminal-size review; ensure `docs/quality/02-progress-log.md` matches reality.

## Acceptance

- [ ] Reproducible clean build and documented run path.
- [ ] Core test suites pass and are reported accurately.
- [ ] Two people complete a live game across terminals.
- [ ] Reconnection, malformed commands, unauthorized commands tested.
- [ ] No core UI clipped or inaccessible; visual quality gate passed.
- [ ] README clearly explains limits and troubleshooting.
