# Agent instructions — Vesper Chess

Read `README.md` and `prompts/IMPLEMENT_WITH_AGENT.md` before making changes. The source of truth is this specification; if requirements conflict, prefer product scope > acceptance criteria > architecture proposals, and document any trade-off in `docs/decisions/`.

- Implement **one vertical slice at a time**, with runnable increments and tests.
- Keep chess rules separate from terminal UI and transport. **Peer-to-peer is non-negotiable:** both players run this binary, the hosting process is the only authority, and `vesper host` / `vesper join` talk over a direct connection. Never introduce a central server, relay, matchmaker or hosted service (see `docs/decisions/ADR-001`). The fake provider exists only for `vesper demo` and visual fixtures; it must not back Host or Join.
- **UX quality is a release gate:** do not settle for an unstyled table and generic prompts. Follow `docs/design/`.
- Visual prototyping uses sample game data. Do not couple that prototype to a server; the multiplayer path is the embedded host.
- Keep external calls asynchronous; always restore terminal on exit/errors/panic where feasible.
- Protect game join/reconnect secrets, redact logs, reject unknown/malformed commands, bound payload sizes.
- Use established crates for legal move generation and game rules. Verify draw-rule support; implement missing adjudication explicitly.
- Run and report `cargo fmt --check`, `cargo clippy --all-targets --all-features -- -D warnings`, `cargo test --workspace` when the Rust workspace exists.
- Clearly state untested cases. Never claim command execution or visual review happened if it did not.
- Update `docs/quality/02-progress-log.md` after each milestone.
