<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# HANDOFF: esotericWebb V31c — Bridge Reconnection + Membrane Permissions

- **Date**: 2026-08-05
- **Version**: V31c (commits 5902a6f → 63734f1)
- **Gate**: ironGate (downstream host)
- **Wave**: 156d
- **Tests**: 484 (484 PASS, 0 FAIL)

## Summary

V31c adds automatic bridge reconnection for stale primal connections and
documents the `/run/membrane/` socket permissions issue that blocks
petalTongue and toadStool access from user-space processes.

## Changes

### V31b: Capability Parity

- `niche::CAPABILITIES` 26→28 (`session.poll_input` + `signal.dispatch`)
- Cell graph aligned with biomeOS v4.57 `biomeos nucleus attach`
- Deploy fragment and capability registry synced to 28 methods
- Systemd unit now uses `--cell --mode attach --listen 0.0.0.0:8090`

### V31c: Bridge Reconnection

- `try_reconnect(domain)` method: re-discovers and reconnects a single
  domain when retries exhaust with a connection error.
- Handles the common case of a primal restarting during a long-running
  session (e.g., petalTongue restarting while esotericWebb has been up 19h).
- One attempt per failure cycle, no reconnect storms.

## Upstream Issues for Overwatch

### P1: Membrane socket permissions block user-space primals

**Evidence**:
```
srwxrwx--- 1 root root 0 Aug  4 14:19 /run/membrane/petaltongue.sock
srwxrwx--- 1 root root 0 Aug  4 14:19 /run/membrane/toadstool.sock
```

**Impact**: esotericWebb (and any user-space process) cannot connect to
primals in `/run/membrane/` when sockets are root:root 770. Discovery
finds them via HTTP health probe but cannot establish RPC connections.

**Affected primals**: petalTongue (visualization), toadStool (compute)

**Fix options**:
1. Socket group = `irongate` (or a `membrane` group) with 770 perms
2. Socket perms = 777 (simple but less secure)
3. Run esotericWebb as root (undesirable)
4. Use supplementary group membership (add `irongate` to socket group)

**Owner**: cellMembrane / gate operations

**Workaround**: esotericWebb degrades gracefully — "Text-mode narration
only. No scene rendering." and "CPU-only game science." All other 7
primals connect normally via XDG-based paths.

### P2: biomeOS neural-api `graph.cell.attach` handler not available

**Evidence**: `Cell 'esotericwebb' operating unmanaged (biomeOS cell.attach unavailable)`

**Status**: biomeOS v4.57 shipped `biomeos nucleus attach` CLI, but the
neural-api service on ironGate doesn't yet handle `graph.cell.attach`
inbound RPC. esotericWebb's client-side call degrades cleanly.

**Owner**: biomeOS team

### P3: `dag.event.batch` vs `dag.event.append_batch` method name

**Evidence**: Blurb mentions `dag.event.append_batch` (200/batch) live.
Our bridge uses `dag.event.batch`. One-line fix once canonical name
is confirmed.

**Owner**: rhizoCrypt team

## Live Site Status

| Endpoint | Status |
|----------|--------|
| `webb.primals.eco` | **200** — HTML frontend + JSON-RPC |
| `GET /` | WebGL canvas + SSE scene stream |
| `POST /` (JSON-RPC) | 28 methods, all operational |
| `GET /api/status` | Health JSON |
| `GET /api/scene/stream` | SSE scene data |

## Deep Debt Posture

| Vector | Status |
|--------|--------|
| Files >800L | 0 (max: director/mod.rs 750L, bridge/mod.rs 534L) |
| Unsafe code | `#![forbid(unsafe_code)]` |
| `#[allow]` annotations | 0 |
| Raw env var strings | 0 |
| Hardcoded primal names | 0 |
| C dependencies | 0 |
| Dead code | 0 |
| Mocks in production | 0 |
| Stale TODOs in code | 0 |
