# HANDOFF: esotericWebb V33 — Gossip Mesh Enmeshment

- **Date**: 2026-08-10
- **Wave**: 157g
- **Direction**: Outbound (esotericWebb → upstream overwatch)
- **Gate**: ironGate

## Summary

esotericWebb is now the 4th member injecting gossip into the swarmVine mesh
(joining rhizoCrypt, loamSpine, lithoSpore). Session lifecycle events are
broadcast so other primals and gates can observe CRPG activity across the mesh.

## What was done

1. **Gossip domain added** — `domain::GOSSIP = "gossip"` backed by swarmVine.
   Discovery via `$XDG_RUNTIME_DIR/biomeos/swarmvine.sock` (UDS).
2. **`gossip_inject` bridge method** — fire-and-forget, degrades silently if
   swarmVine unavailable. API: `gossip.inject(topic, key, value)`.
3. **Session lifecycle events** emitted at two points:
   - `esotericwebb.session.started` (when provenance DAG is created)
   - `esotericwebb.session.completed` (when game reaches an ending)
4. **Cell graph updated** — gossip is optional, degradation is clean text-only.
5. **Bridge expanded** to 10 domains, 36 methods, 9/10 connected on ironGate
   (toadstool absent as expected — no GPU dispatch needed locally).

## Gossip entry format

```json
{
  "topic": "data",
  "key": "esotericwebb.session.started",
  "value": "{\"gate\":\"ironGate\",\"primal\":\"esotericwebb\",\"session_id\":\"...\",\"turn\":0}"
}
```

## Upstream issues

| Issue | Owner | Detail |
|-------|-------|--------|
| Cross-gate TCP 7800 unreachable | gate ops | 4/6 peers UNREACHABLE from ironGate. gossip injects locally but doesn't cross gates. |
| songBird MeshRelay not shipped | songBird | Without MeshRelay, gossip stays local until TCP 7800 is opened. |
| swarmVine `gossip.get`/`gossip.query` | swarmVine | No retrieval API found — inject works, but reading back is undocumented. |

## Validation

- 466 tests pass (465 + 1 new gossip test)
- 0 clippy warnings
- `webb.primals.eco` 200 OK
- systemd logs confirm `[+] swarmvine (gossip) uds:/run/user/1000/biomeos/swarmvine.sock`
- Manual `gossip.inject` via socat returned `{"result":"Accepted"}` earlier

## Next (after pipeline enmeshment)

- WebGL pipeline (G19): browser surfaces with petalTongue
- Science pipeline E2E (G71): NFT endpoint, pseudoSpore bundles
- More gossip keys: `esotericwebb.action.milestone`, `esotericwebb.session.degraded`
