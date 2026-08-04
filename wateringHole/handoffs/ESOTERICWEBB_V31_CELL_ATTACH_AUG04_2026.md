<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# HANDOFF: esotericWebb V31 — Cell Attachment Protocol

- **Date**: 2026-08-04
- **Version**: V31 (commits c08a49a → c93a8f4)
- **Gate**: ironGate (downstream host)
- **Wave**: 155v/156d
- **Tests**: 484 (exp006 22/22 PASS on ironGate NUCLEUS)

## Summary

V31 closes the "remaining ops gap" identified in the ecosystem blurb:
**cell attachment (`--mode attach`)**. esotericWebb can now register with
biomeOS as a managed cell, report its health status, and receive lifecycle
events — completing the Phase 1 structural readiness for live cell boot.

## Changes (V30c → V31)

### V30c: Signed Provenance + Batch Chunking

- bearDog `crypto.sign` wired into `record_provenance_vertex()`. DAG vertices
  include a signature when the crypto domain is available. Closes the
  provenance loop on the consumer side (bearDog sig → sweetGrass braid).
- `DAG_BATCH_MAX = 200` constant aligned with rhizoCrypt upstream spec.
- `dag_event_batch` chunks large event slices respecting the batch limit.

### V30d: Deep Debt — Hardcoded Strings + Dead Code

- `HOSTNAME` env key constant added; wired in `niche::gate_id()`.
- Inline `"biomeos"` socket dir strings → `ECOSYSTEM_SOCKET_DIR` constant.
- `demo.rs` dead fields evolved: `description` now displayed, `voice_notes_possible`
  now verified during demo runs.
- Zero raw `std::env::var("...")` strings remain in production code.

### V31: Cell Attachment Protocol

- **`--mode attach` CLI flag**: when specified with `--cell`, registers the
  process with biomeOS as a managed cell via `graph.cell.attach`.
- **`graph.cell.attach` method**: sends cell ID, name, socket, gate, health
  status, and capability report to biomeOS composition layer.
- **`graph.cell.health` method**: periodic health status push (LIVE/DEGRADED/FAILED).
- **`bridge/lifecycle.rs` extraction**: announce_self, cell_attach,
  cell_health_report, register_mesh_route moved from domains.rs (780→697 lines).
- **Graceful degradation**: when biomeOS doesn't support `graph.cell.attach`,
  Webb logs and continues operating unmanaged.

## Upstream Gaps (for primal teams)

| Gap | Owner | Description | Priority |
|-----|-------|-------------|----------|
| **biomeOS `graph.cell.attach` handler** | biomeOS | Server-side handler to accept cell registrations and track health. esotericWebb sends the payload; biomeOS needs to receive it. | High |
| **biomeOS `graph.cell.health` handler** | biomeOS | Accept periodic health reports from attached cells. | Medium |
| **toadStool offline** | toadStool | Compute domain still shows misconfigured unit on ironGate (9th primal absent). Cell reports DEGRADED due to this. | Low |
| **rhizoCrypt `dag.event.batch` → `dag.event.append_batch`** | rhizoCrypt | Upstream method name is `dag.event.append_batch` (200/batch). Our bridge tries `dag.event.batch` first — need name alignment confirmation. | Low |

## Deep Debt Posture (confirmed clean)

| Vector | Status |
|--------|--------|
| Files >800L | 0 (max: director/mod.rs 750L) |
| Unsafe code | `#![forbid(unsafe_code)]` |
| Mocks in production | 0 |
| `#[allow]` annotations | 0 (all `#[expect]` with reasons) |
| Raw env var strings | 0 |
| `todo!()`/`panic!()` | 0 |
| Hardcoded primal names | 0 (canonical `primal_names` constants) |
| C dependencies | 0 (libc via signal-hook only) |
| Dead code | 0 |

## Cell Boot Readiness

```
esotericwebb serve \
  --content content/ \
  --cell graphs/esotericwebb_cell.toml \
  --mode attach \
  --listen 0.0.0.0:8090
```

This will:
1. Discover all primals via 5-tier discovery
2. Validate cell graph health gates
3. Announce to biomeOS (primal.announce)
4. Attempt cell attachment (graph.cell.attach)
5. Start UDS + TCP listeners
6. Serve `webb.primals.eco` via Caddy reverse proxy

Blocked only on: biomeOS shipping `graph.cell.attach` server-side handler.

## For Upstream Overwatch

esotericWebb is structurally complete for Phase 1 live cell boot. The
ecosystem posture (155v/156d) confirms V31 validated on ironGate. No P0/P1/P2
issues. The cell attachment protocol is ready for biomeOS to consume whenever
the server-side handler ships.
