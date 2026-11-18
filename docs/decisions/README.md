# Architecture decision records

Create ADRs here during discovery and implementation. Use this minimal format:

```md
# ADR-XXX: Decision title
Status: Proposed | Accepted | Superseded
Date: YYYY-MM-DD

## Context
What problem requires a decision?

## Options
Describe meaningful alternatives.

## Decision
What is chosen and why?

## Consequences
Trade-offs, risks and follow-up work.
```

## Index

| ADR | Decision |
|---|---|
| [ADR-001](ADR-001-p2p-host-peer-topology.md) | Embedded host-peer topology; no central server |
| [ADR-002](ADR-002-transport-tcp-ndjson.md) | Direct TCP, Noise_XX, then JSON frames; reachability limits |
| [ADR-003](ADR-003-invite-code-and-seat-token.md) | Single-use invite code, private seat token, seat ownership |
| [ADR-004](ADR-004-sync-versions-idempotency.md) | Full snapshots, versions, request-id idempotency, heartbeats |
| [ADR-005](ADR-005-rules-crate-and-draws.md) | shakmaty rules; explicit automatic vs claimable draws |
| [ADR-006](ADR-006-layout-terminal-and-crates.md) | Single package with module boundaries; terminal baseline and visual review process |
