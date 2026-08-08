<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# AAR: esotericWebb V32 — ironGate G68 Redeploy (Wave 157a)

- **Date**: 2026-08-08
- **Version**: V32
- **Gate**: ironGate (downstream host, NUCLEUS)
- **Wave**: 157a
- **Outcome**: **13/13 ALIVE, Cell LIVE, 9/9 domains connected, webb.primals.eco 200**

---

## What Happened

ironGate was listed as "PENDING — Awaiting redeploy" in Wave 157a blurb
(sporeGate, blueGate, southGate already complete). This session executed
the full redeploy:

1. Pulled G68 depot from golgi via SSH rsync (17 musl binaries)
2. Discovered that membrane-managed NUCLEUS (system layer) was already
   running 13/13 primals — BUT with root:root 770 socket permissions
3. Fixed permissions (`chgrp biomeos /run/membrane/*.sock`)
4. Updated `esotericwebb.service` to acquire `biomeos` group via `sg` wrapper
5. Rebuilt and restarted esotericWebb — Cell LIVE, 9/9 domains

## What Works

| Component | Status | Details |
|-----------|--------|---------|
| **Membrane NUCLEUS** | 13/13 ALIVE | beardog, songbird, skunkbat, toadstool, barracuda, coralreef, nestgate, rhizocrypt, loamspine, sweetgrass, squirrel, petaltongue, biomeOS |
| **biomeOS Neural API** | v4.57.0 | 593 registered capabilities, alive on neural-api socket |
| **esotericWebb Cell** | LIVE | 3 required (beardog, songbird, squirrel) + 6 optional all healthy |
| **webb.primals.eco** | 200 OK | HTML frontend + JSON-RPC (28 methods), SSE scene stream |
| **Discovery** | 5-tier | XDG, `/run/membrane`, env override, TCP well-known, HTTP REST |
| **Transports discovered** | All | UDS, UDS+BTSP (sweetgrass, toadstool), TCP (nestgate), HTTP (songbird) |
| **SSH discipline** | ENFORCED | Zero GitHub remotes anywhere on ironGate |
| **G68 depot** | 17/17 musl | All Aug 8 builds, rsynced from golgi |
| **Signed provenance** | Active | bearDog `crypto.sign` wired into DAG vertices |
| **Batch provenance** | Ready | `dag.event.batch` with 200/batch chunking |
| **Reconnection** | Active | Auto-reconnects stale primal connections |

## What Was Blocked (and resolved)

### Membrane socket permissions (P1 from V31c)

**Previous state**: `/run/membrane/*.sock` were `root:root srwxrwx---` (770).
User-space processes (including esotericWebb) could not connect.

**Resolution**: `sudo chgrp biomeos /run/membrane/*.sock` — irongate is
already in the `biomeos` group (gid 1012).

**Remaining risk**: This fix does NOT persist across reboot. Needs permanent
solution (see "Upstream Issues" below).

### Systemd user service group inheritance

**Problem**: User's login session (systemd user manager, PID 1370) was started
before `irongate` was added to `biomeos` group. The `SupplementaryGroups=`
directive is system-only.

**Resolution**: `ExecStart=/usr/bin/sg biomeos -c "..."` wrapper acquires the
group at service start time. Works reliably, no restart required.

### Stale songbird PID lock

**Problem**: Membrane-managed songbird (PID lock at
`~/.local/share/songbird/songbird-irongate.pid`) prevented user-space
instance from starting.

**Resolution**: Not needed — membrane instance IS the authoritative songbird.
User-space duplicates are unnecessary when membrane layer is healthy.

## What Still Needs Evolution (Upstream)

### P1: Membrane socket permissions persistence

**Status**: Fixed manually, will revert on reboot.

**Recommended fix**: Add to membrane systemd unit:
```ini
ExecStartPost=/bin/chgrp biomeos /run/membrane/*.sock
```
Or configure `UMask=0007` with `Group=biomeos` on the membrane template unit.

**Owner**: cellMembrane team / gate operations

### P2: biomeOS `graph.cell.attach` handler

**Status**: Cell operates unmanaged. `graph.cell.attach` RPC returns nothing.
biomeOS v4.57 has the `biomeos nucleus attach` CLI but the neural-api
inbound RPC handler is not yet wired.

**Owner**: biomeOS team

### P3: `dag.event.batch` method name alignment

**Status**: Blurb mentions `dag.event.append_batch`. Our bridge uses
`dag.event.batch`. Fallback to sequential works, but names should align.

**Owner**: rhizoCrypt team

### P4: `sg` wrapper in service unit (minor)

**Status**: Works but is a workaround. Once the user re-logs (or systemd
user manager is restarted), the `biomeos` group will be inherited natively
and the `sg` wrapper can be removed.

**Alternative**: System reboot after group addition would have also worked.

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 484 (all pass) |
| Clippy warnings | 0 |
| Unsafe code | `#![forbid(unsafe_code)]` |
| Files >800L | 0 |
| C deps | 0 |
| `#[allow]` annotations | 0 |
| Dead code | 0 |
| Stale TODOs | 0 |
| Capabilities exposed | 28 |
| Bridge methods | 35 |
| RSS (service) | ~1 MB |

## Deployment Topology

```
Internet → webb.primals.eco (Caddy on golgi, HTTPS)
  → WireGuard 10.13.37.7:8090 (ironGate)
    → esotericWebb (sg biomeos, cell-attach mode)
      → /run/membrane/*.sock (13 primals, root:biomeos 770)
      → /run/user/1000/biomeos/*.sock (XDG discovery)
      → TCP 127.0.0.1:8080 (nestgate), :7700 (songbird)
```

## For Overwatch

**ironGate redeploy: DONE.** Mark as complete in gate status tracker.

Remaining from Wave 157a for other teams:
- **strandGate**: SSH depot access to golgi (same rsync pattern used here)
- **westGate**: Pending redeploy (same procedure applies)
- **cellMembrane**: Permanent socket permissions fix
- **biomeOS**: `graph.cell.attach` RPC handler
- **rhizoCrypt**: Method name alignment (`dag.event.batch` vs `append_batch`)
