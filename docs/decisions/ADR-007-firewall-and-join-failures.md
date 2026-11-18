# ADR-007: Firewall policy and initial join failures

Status: Accepted
Date: 2026-11-18

## Context

An Omarchy host listened on `0.0.0.0:4747` and responded to ping, while TCP
connections from the guest timed out. Omarchy's default incoming firewall policy
can cause this. The peer previously retried initial connection failures forever
without reporting an error.

## Decision

Initial TCP and handshake failures return an actionable error through the existing
provider failure path. Reconnection with an issued seat token retains automatic
retry. A timeout suggests checking reachability and firewall policy; it does not
claim to diagnose the firewall conclusively.

Document listener checks, TCP checks, and scoped UFW allow/removal commands in the
README, and install that document with the Arch package. Installation and hosting
do not change firewall rules: choosing permitted source addresses and persistent
network access belongs to the host owner. Direct peer-to-peer transport is unchanged.

## Consequences

Guests retry an initial failed join themselves after fixing connectivity. Existing
games continue to retry automatically. No root privileges or firewall dependency
is added to Vesper. Packaged changes require a subsequent release; modifying the
local recipe does not update an already published version's source archive.
