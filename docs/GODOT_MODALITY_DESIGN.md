# Godot Modality — SceneGraph to .tscn/GDExtension Compilation

## Status: Design (Long-term)

This document describes the architectural design for a Godot rendering modality
within the ecoPrimals ecosystem, enabling esotericWebb's narrative engine to
drive a full 3D game client via Godot.

## Architecture Overview

```
esotericWebb (narrative engine)
  │
  ├─ push SceneGraph ──► petalTongue (visualization primal)
  │                           │
  │                           ├─ WebGL modality (browser)  ✓ implemented
  │                           ├─ SVG modality (static)     ✓ implemented
  │                           ├─ Terminal modality (TUI)   ✓ implemented
  │                           └─ Godot modality (NEW)
  │                                 │
  │                                 ├─ Option A: .tscn compilation
  │                                 └─ Option B: GDExtension live feed
  │
  └─ Webb never talks directly to Godot (primal boundary preserved)
```

## Design Principles

1. **Primal boundary**: Webb only talks to primals via JSON-RPC. It never
   imports Godot SDKs directly. All Godot interaction routes through petalTongue.

2. **Modality as compiler target**: Just as petalTongue compiles `SceneGraph`
   to WebGL draw calls, a Godot modality compiles it to Godot scene format.

3. **Incremental adoption**: The modality can start with 2D (matching current
   scene layout) and evolve to 3D as Webb's scene graph gains depth.

## Option A: Static .tscn Compilation

petalTongue compiles `SceneGraph` → Godot `.tscn` resource file.

### Mapping

| petalTongue Primitive | Godot Node Type        |
|----------------------|------------------------|
| `Point`              | `Sprite2D` (circle)    |
| `Rect`               | `ColorRect`            |
| `Text`               | `Label` / `RichTextLabel` |
| `Line`               | `Line2D`               |
| `Polygon`            | `Polygon2D`            |
| `Mesh`               | `MeshInstance3D`       |
| `Arc`                | `Path2D` with arc      |

### Animation Mapping

| petalTongue Animation | Godot Equivalent         |
|----------------------|--------------------------|
| `Opacity`            | `AnimationPlayer` (modulate.a) |
| `Translate`          | `AnimationPlayer` (position) |
| `Scale`              | `AnimationPlayer` (scale)    |
| `Rotation`           | `AnimationPlayer` (rotation) |

### Workflow

1. Webb pushes `visualization.render.scene` (existing flow)
2. petalTongue receives scene, selects Godot modality
3. Compiler transforms SceneGraph → `.tscn` text format
4. `.tscn` written to shared filesystem or served via HTTP
5. Godot project watches directory, auto-reloads scene

### Advantages
- Simple implementation (text generation)
- Works with standard Godot editor
- No runtime dependency on Godot being active

### Disadvantages
- File I/O latency
- No real-time scene updates
- Requires Godot to poll for changes

## Option B: GDExtension Live Feed (Preferred)

A `gdext` (godot-rust) extension receives scene updates in real-time via JSON-RPC.

### Architecture

```
petalTongue ──JSON-RPC──► GDExtension plugin (godot-rust)
                              │
                              ├─ Parses SceneGraph JSON
                              ├─ Creates/updates Godot nodes
                              ├─ Applies animations
                              └─ Sends input events back
```

### Implementation

```rust
// In the GDExtension (godot-rust crate)
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct PetalTongueReceiver {
    #[base]
    base: Base<Node>,
    client: Option<TcpStream>,
}

#[godot_api]
impl INode for PetalTongueReceiver {
    fn process(&mut self, _delta: f64) {
        if let Some(scene_update) = self.poll_scene() {
            self.apply_scene_graph(scene_update);
        }
    }
}
```

### Protocol

The GDExtension connects to petalTongue's TCP JSON-RPC port and subscribes
to scene updates via `visualization.render.stream`:

```json
{"jsonrpc": "2.0", "method": "visualization.render.stream", "params": {"subscribe": true}, "id": 1}
```

petalTongue pushes scene updates as NDJSON notifications:

```json
{"jsonrpc": "2.0", "method": "scene.update", "params": {"root": "root", "nodes": {...}, "animations": [...]}}
```

### Advantages
- Real-time scene updates (60fps capable)
- Full animation control
- Bidirectional: Godot sends input events back to Webb via petalTongue
- Native Godot rendering (shaders, particles, lighting)

### Disadvantages
- Requires godot-rust toolchain
- More complex deployment
- Godot must be running alongside primals

## Implementation Phases

### Phase 6a: SceneGraph → Godot Type Mapping (Design)
- Define the complete mapping table
- Handle edge cases (missing primitives, 2D→3D promotion)
- Design the animation timeline synchronization

### Phase 6b: GDExtension Skeleton (godot-rust)
- Create a minimal `gdext` crate in `springs/` or `gardens/`
- TCP client connecting to petalTongue
- Parse scene JSON into Godot node tree

### Phase 6c: Scene Compiler in petalTongue
- Add `Godot` variant to petalTongue's modality enum
- Implement `.tscn` text generation from `SceneGraph`
- Register `visualization.render.godot` method

### Phase 6d: Bidirectional Input
- GDExtension sends player actions back via JSON-RPC
- Maps Godot input events to Webb's action format
- Enables full game loop: Webb narrative ↔ Godot rendering

### Phase 6e: 3D Scene Evolution
- Webb scene graph gains depth (z-axis, camera, lighting)
- petalTongue compiles to `Node3D` hierarchy
- NPC models, environment meshes, particle effects

## Dependencies

- `godot-rust/gdext` (MIT) — Rust bindings for GDExtension API
- Godot 4.x engine
- petalTongue `petal-tongue-scene` crate (existing)
- esotericWebb scene push (existing, animations added in Phase 5)

## Timeline

This is a **long-term** goal. Prerequisites:
1. petalTongue's live mode working on ironGate (Phase 4 — blocked on xlib)
2. WebGL rendering proven end-to-end (Phase 3 — done)
3. Animation metadata flowing (Phase 5 — done)
4. Godot installed on ironGate or a dedicated render node

Estimated: Q4 2026 earliest for Phase 6b prototype.
