# HANDOFF: V30 Cell Graph Validation + Batch Provenance

- **Date**: 2026-08-04
- **Garden**: esotericWebb
- **Gate**: ironGate
- **Wave**: 155u / 156b
- **Direction**: Outbound
- **AGPL-3.0-or-later**

---

## Summary

V30 closes the biomeOS cell attachment gap and prepares for rhizoCrypt G31
batch provenance. V30b eliminates the last untyped content path and removes
all `#[allow]` annotations from production code.

---

## Deliverables

### 1. Cell graph validation (`--cell`)

```bash
esotericwebb serve --content content/ --cell graphs/esotericwebb_cell.toml
```

Loads `esotericwebb_cell.toml`, validates each required/optional domain
against live discovery, prints LIVE/DEGRADED/FAILED with per-domain detail.
Uses `has_direct()` to avoid Neural API routing inflation.

**Live result on ironGate:**
```
Cell [esotericwebb] — DEGRADED
  [+] required: beardog (crypto)
  [+] required: songbird (mesh)
  [+] required: squirrel (ai)
  [+] optional: 5/6 present
  [-] degraded compute: No GPU compute offload. CPU-only game science.
```

### 2. Batch provenance (`dag.event.batch`)

Bridge method ready for rhizoCrypt G31. Falls back to sequential
`dag.event.append` when `MethodNotFound`. Addresses the 12× throughput
collapse documented in Wave 155u provenance × acquisition divergence.

### 3. Typed LocationDef (V30b)

`worlds/` content loading evolved from `HashMap<String, serde_json::Value>`
to `HashMap<String, LocationDef>`. Dead `load_raw_yaml_dir` removed. All
content types now fully typed.

### 4. Canonical name constants

Discovery well-known port tables now use `primal_names::SONGBIRD`,
`PETALTONGUE`, `NESTGATE` instead of string literals.

---

## Upstream Gaps (unchanged from V29)

| Gap | Primal | Notes |
|-----|--------|-------|
| `visualization.render.grammar` | petalTongue | Webb sends, degrades silently |
| biomeOS neural-api ZOMBIE | biomeOS | Blocks orchestrated composition |
| toadStool misconfigured unit | toadStool | Compute domain absent on ironGate |

---

## Scorecard

| Metric | V29 | V30 |
|--------|-----|-----|
| Tests | 477 | 482 |
| `#[allow]` in production | 2 | 0 |
| Untyped content paths | 1 (worlds/) | 0 |
| Cell validation | — | DEGRADED (8/9) |
| Batch provenance | — | Ready (sequential fallback) |
| Clippy warnings | 0 | 0 |
