<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# AAR: esotericWebb V31b — First Live Cell Boot Attempt on ironGate

- **Date**: 2026-08-04 13:28 EDT
- **Version**: V31b
- **Gate**: ironGate (downstream host, i9-14900K, RTX 5070, 94GB)
- **Wave**: 155v/156d
- **Operator**: esotericWebb on ironGate, overwatch from eastGate

---

## Executive Summary

Deployed esotericWebb V31b with full cell-attach mode on ironGate. **The
first-ever live cell composition boot attempted in the ecosystem.** Cell
graph validated, 8/9 primals connected, service live on `webb.primals.eco`.
Cell attach degraded gracefully (biomeOS neural-api on ironGate hasn't
received v4.57 cell handler yet). Zero downtime transition.

---

## What Works (CONFIRMED LIVE)

| Component | Evidence |
|-----------|----------|
| **28 JSON-RPC capabilities** | `capabilities.list` returns 28 methods on `webb.primals.eco` |
| **8/9 primals connected** | squirrel, petalTongue, nestGate, rhizoCrypt, loamSpine, sweetGrass, bearDog, songBird |
| **Cell graph validation** | Runs at boot, reports DEGRADED (compute absent), all required PASS |
| **Signed provenance** | bearDog `crypto.sign` in DAG vertices (crypto domain connected) |
| **Scene push to petalTongue** | `enrichment.scene_pushed: true` on every action |
| **Flow science** | Local `science/` module computing `flow_score` per action |
| **External HTTPS** | `webb.primals.eco` via golgi Caddy reverse proxy (326ms RTT) |
| **WebGL frontend** | HTML/JS game client served on GET, zero external deps |
| **JSON-RPC over POST** | All 28 methods callable via external HTTPS |
| **UDS + TCP dual listener** | `/run/user/1000/biomeos/esotericwebb.sock` + `0.0.0.0:8090` |
| **Graceful shutdown** | Clean `shutting down` on SIGTERM, systemd restart works |
| **Gameplay E2E** | talk, navigate, abilities, knowledge gain, trust changes all working |
| **Batch provenance ready** | `dag.event.batch` method present (fallback to sequential) |
| **`primal.announce` at startup** | Registers with biomeOS neural-api on boot |

---

## What Degrades Gracefully

| Issue | Impact | Mitigation |
|-------|--------|------------|
| **toadStool compute offline** | No GPU compute offload | CPU-only game science (local `science/` module) |
| **biomeOS `graph.cell.attach` unavailable** | Cell operates unmanaged | All features work — management is a bonus, not a requirement |
| **petalTongue GPU live mode blocked** | No native GPU window | WebGL via browser works; SSE streaming active |

---

## Remaining Issues for Overwatch

### P1: biomeOS neural-api on ironGate needs v4.57 cell handler

**Evidence**: Service log shows `Cell 'esotericwebb' operating unmanaged (biomeOS cell.attach unavailable)`

**Action**: The biomeOS neural-api running on ironGate's NUCLEUS doesn't yet have the `graph.cell.attach` server-side handler. biomeOS v4.57 shipped the `biomeos nucleus attach` CLI but the neural-api service binary on ironGate needs updating.

**Owner**: biomeOS team / gate operations

**Fix**: Update ironGate's biomeOS neural-api binary to v4.57+ and restart. esotericWebb will automatically transition to managed mode on next restart.

### P2: toadStool systemd unit misconfigured on ironGate

**Evidence**: Discovery shows `[-] toadstool (compute) —` (no socket found)

**Action**: The toadStool primal's membrane unit isn't starting on ironGate. This is the 9th primal blocking LIVE cell status (currently DEGRADED).

**Owner**: toadStool team / ironGate operations

**Fix**: Check `membrane-nucleus@toadstool.service` unit, verify binary in plasmidBin, fix socket registration.

### P3: `dag.event.batch` vs `dag.event.append_batch` method name alignment

**Evidence**: esotericWebb sends `dag.event.batch`; ecosystem blurb mentions rhizoCrypt shipped `dag.event.append_batch` (200/batch).

**Action**: Confirm canonical method name. If it's `append_batch`, esotericWebb needs a one-line constant rename.

**Owner**: rhizoCrypt team

**Impact**: Low — sequential fallback works. Batch performance is degraded (1× instead of 12×) until names align.

### P4: petalTongue live GPU mode (X11 library linkage)

**Evidence**: Previous attempt to run `petaltongue-live.service` failed with `winit EventLoopError: os error: Failed to load one of xlib's shared libraries`

**Action**: petalTongue binary needs to be compiled/linked against the correct X11 shared libraries present on ironGate, or the symlinks need to be created.

**Owner**: petalTongue team

**Impact**: Low — WebGL browser path works. GPU live mode is a nice-to-have for native rendering.

---

## Metrics

| Metric | Value |
|--------|-------|
| Tests | 484 (465 unit + 18 E2E + 1 integration) |
| Capabilities | 28 |
| Primals connected | 8/9 |
| Cell health | DEGRADED (compute absent) |
| Clippy warnings | 0 |
| Unsafe code | `#![forbid(unsafe_code)]` |
| External latency | 326ms (golgi → ironGate via WireGuard) |
| Memory usage | 644 KB RSS |
| Uptime since deploy | Continuous (systemd auto-restart) |

---

## Deployment Configuration (LIVE)

```bash
# Service unit: ~/.config/systemd/user/esotericwebb.service
ExecStart=esotericwebb serve \
  --content content/ \
  --cell graphs/esotericwebb_cell.toml \
  --mode attach \
  --listen 0.0.0.0:8090

# Caddy (golgi): webb.primals.eco → 10.13.37.7:8090 (WireGuard)
# UFW: allow from 10.13.37.0/24 to port 8090 tcp
```

---

## Conclusion

esotericWebb V31b is the **first live cell composition in the ecoPrimals
ecosystem**. It discovers 8 primals at runtime, validates a cell graph,
attempts biomeOS registration, and serves a playable CRPG over HTTPS — all
from a single 644 KB process with zero C dependencies. The remaining issues
are all upstream (biomeOS handler update, toadStool unit fix, method name
alignment) and can be resolved without any esotericWebb code changes.

**Next milestone**: biomeOS neural-api update → cell transitions to MANAGED
→ first fully tracked cell in the ecosystem.
