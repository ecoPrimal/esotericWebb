# HANDOFF: V29 petalTongue Integration + Deep Debt Refactor

- **Date**: 2026-08-03
- **Garden**: esotericWebb
- **Gate**: ironGate
- **Wave**: 155q / 156b
- **Direction**: Outbound (upstream absorption candidates)
- **AGPL-3.0-or-later**

---

## Summary

V27–V29 evolved esotericWebb from a static composition into a live visual
system with real-time scene streaming. Key deliverables:

1. **Grammar of graphics integration** — `visualization.render.grammar` bridge
   sends NPC trust/knowledge `GrammarExpr` to petalTongue for declarative
   visualization.

2. **Rich scene primitives** — `build_game_scene_graph` emits typed `SceneNode`
   payloads (Rect, Text, Point) with transform/opacity/visibility semantics
   matching petalTongue's native format.

3. **Animation pipeline** — Scene transitions (fade-in background/title) and
   staggered NPC entrance effects (translate + opacity ramp) via animation
   metadata in the SceneGraph push.

4. **WebGL browser frontend** — SSE endpoint `/api/scene/stream` streams scene
   updates. `index.html` renders via WebGL with 2D canvas fallback. Zero
   external JavaScript dependencies.

5. **Live site deployment** — `webb.primals.eco` served from ironGate:8090 via
   golgi Caddy reverse proxy over WireGuard mesh. Verified 200 OK end-to-end.

6. **V29 deep debt refactor** — Extracted `session/scene_builder.rs` (224L)
   and `ipc/bridge/tests.rs` (309L). All files under 800L. Zero clippy
   warnings, 477 tests, `cargo fmt` clean.

---

## Patterns for Upstream Absorption

### Grammar of Graphics Protocol (petalTongue)

Webb sends `visualization.render.grammar` with:
```json
{
  "session_id": "esotericwebb-grammar",
  "grammar": { "mark": "bar", "encoding": { "x": {...}, "y": {...} } },
  "data": [...],
  "modality": "webgl"
}
```
petalTongue should compile `GrammarExpr` → SceneGraph internally and return
the rendered scene or confirmation. Currently petalTongue does not expose this
method — Webb logs and degrades.

### SSE Scene Streaming Pattern

The `/api/scene/stream` endpoint demonstrates a reusable pattern for any
primal that needs to push real-time updates to browser clients:
- `Content-Type: text/event-stream`
- Named events (`event: scene\n`)
- JSON payload per event
- Reconnect via `EventSource` with 3s retry

### Animation Metadata Schema

```json
{
  "target": {"Opacity": {"node_id": "background", "from": 0.0, "to": 1.0}},
  "duration_secs": 0.4,
  "easing": "EaseInOut",
  "delay_secs": 0.0
}
```
Targets: `Opacity`, `Translate`. This matches petalTongue's `Animation` struct.
Webb produces these; petalTongue (or any renderer) consumes them.

---

## Upstream Gaps Discovered

| Gap | Primal | Severity | Notes |
|-----|--------|----------|-------|
| `visualization.render.grammar` not exposed | petalTongue | low | Webb sends, degrades silently |
| petalTongue live GPU mode needs X11 lib rebuild | petalTongue | medium | `winit` can't find `libX11.so` symlinks in systemd user env |
| biomeOS neural-api still ZOMBIE | biomeOS | critical | Blocks orchestrated composition (GAP-017) |

---

## Scorecard

| Metric | V26 | V29 |
|--------|-----|-----|
| Tests | 455 | 477 |
| Files >800L | 0 | 0 |
| Clippy warnings | 0 | 0 |
| Unsafe blocks | 0 | 0 |
| C dependencies | 0 | 0 |
| Bridge methods | 33 | 33 |
| Capabilities | 27 | 27 |
| Frontend | static HTML | WebGL + SSE stream |
| Scene push | render_scene only | render_scene + grammar + animation |
| Live site | — | `webb.primals.eco` 200 OK |
