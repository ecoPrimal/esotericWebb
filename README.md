# Esoteric Webb

| | |
|---|---|
| **Version** | V31 |
| **Tests** | 484 |
| **Rust files** | 66 (~16.4k LOC production, ~19.4k total) |
| **Experiments** | 6 (exp001–exp006) |
| **MSRV** | 1.87 (edition 2024) |
| **License** | AGPL-3.0 + ORC + CC-BY-SA 4.0 |
| **Unsafe** | `#![forbid(unsafe_code)]` (workspace-level) |
| **C deps** | Zero — all pure Rust (noyalib replaced C libyaml) |
| **Prod unwraps** | 0 |
| **Files >800L** | 0 (max: director/mod.rs 750L) |
| **Bridge methods** | 35 (all domains, all degrading, composition-first) |
| **Capabilities exposed** | 28 (sourDough + lifecycle + narrative + session + dispatch + introspection + MCP) |
| **Primals consumed** | 9 domains (ai, viz, dag, lineage, compute, storage, provenance, crypto, mesh) |
| **Live primals (ironGate)** | 8/9 healthy (zero-config discovery) |
| **Transports** | NDJSON/TCP, UDS, UDS+BTSP, HTTP POST (`/jsonrpc`), REST health |
| **Frontend** | `webb.primals.eco` — WebGL scene canvas + SSE streaming, zero external deps |
| **Scene push** | `visualization.render.scene` + grammar-of-graphics + animation pipeline |
| **Discovery** | 5-tier: biomeOS XDG, `/run/membrane`, env override, TCP well-known, HTTP REST |
| **Cell graph** | `--cell graphs/esotericwebb_cell.toml` — validates LIVE/DEGRADED/FAILED |
| **Cell attach** | `--mode attach` — registers with biomeOS as managed cell |
| **Signed provenance** | bearDog `crypto.sign` in DAG vertices, provenance loop CLOSED |
| **Batch provenance** | `dag.event.batch` with 200/batch chunking (rhizoCrypt G31 aligned) |
| **Last validation** | 2026-08-04 (V31, ironGate NUCLEUS, exp006 22/22 PASS) |

**A [sporeGarden](https://github.com/sporeGarden) project — the primals as a composed CRPG.**

**Live**: `webb.primals.eco` (ironGate:8090 via golgi reverse proxy, WireGuard mesh)  
**Deployed**: ironGate NUCLEUS (8/9 primals, systemd user unit, `esotericwebb.service`)

Esoteric Webb is not a spring. It is a standalone cross-evolution substrate
that composes deployed primals (genomeBins/ecoBins from `plasmidBin/`) into
a creator/player game system via BYOB composition. Its dual purpose:

1. **Build a real game** — a Disco Elysium-inspired CRPG with DAG-traced
   narrative, deep NPCs, emergent ability interactions, and meaningful
   bounded endings.
2. **Find every gap** — in rendering, AI, science, provenance, discovery,
   compute, and composition. Gaps feed back as evolution pressure on the
   primals that need to grow.

## Tool vs Science

Webb is a **working tool**, informed by science, anchored in primal
capabilities. Springs are not primals — springs PRODUCE primals. Webb
consumes primals like primalSpring does, via JSON-RPC IPC and capability-based
discovery from `ecoPrimals/plasmidBin/`. This makes Webb leaner than the
springs it draws science from.

```
Springs (science + experiments)  →  produce  →  primals (genomeBin/ecoBin)
                                                       ↓
                                               plasmidBin/ (deployment)
                                                       ↓
                                          Webb discovers + composes via IPC
```

## Architecture

Esoteric Webb sits atop the primal stack as a **narrative direction
engine**. It consumes primals via JSON-RPC IPC — zero Rust crate dependencies
on any spring. Primals are resolved from `plasmidBin/` or discovered via
Songbird at runtime.

| Domain | Primal | Role | Status (V30, ironGate) | Key IPC methods |
|--------|--------|------|-------------|-----------------|
| ai | Squirrel | AI narration, NPC dialogue, signal planning | **Live** (UDS) | `ai.query`, `ai.suggest`, `signal.plan`, `signal.dispatch` |
| visualization | petalTongue | Scene rendering, grammar-of-graphics | **Live** (REST+UDS) | `visualization.render.scene`, `visualization.render.grammar`, `interaction.poll` |
| dag | rhizoCrypt | Provenance DAG lifecycle | **Live** (UDS) | `dag.session.create`, `dag.event.append`, `dag.merkle.root` |
| lineage | loamSpine | NPC personality certs | **Live** (UDS) | `certificate.mint` |
| compute | toadStool | GPU compute dispatch | Offline (misconfigured unit) | `compute.dispatch.submit` |
| storage | nestGate | Key-value persistence | **Live** (TCP:8080) | `storage.store`, `storage.retrieve` |
| provenance | sweetGrass | Creative attribution | **Live** (UDS+BTSP) | `braid.create`, `braid.query` |
| crypto | bearDog | Signing, verification, hashing | **Live** (UDS) | `crypto.sign`, `crypto.verify`, `crypto.hash` |
| mesh | songBird | Topology, discovery, bonds | **Live** (HTTP:7700) | `discovery.topology`, `discovery.health`, `discovery.bonds` |

## The Core Thesis: Bounded Space, Infinite Exploration

Traditional CRPGs have branching narratives where NPCs can only say so many
things and player input variety is masked. Esoteric Webb inverts this:

- The **narrative topology** (DAG structure) is finite and authored
- The **traversal state** (knowledge, trust, conditions, inventory, arc phases)
  is combinatorially vast
- **Abilities** interact with state in ways authors cannot predict, creating
  emergent paths through authored structure
- **Bounded endings** carry more meaning because they reflect genuine paths
  through a rich state space

## Quick start

```bash
# Build
cargo build --workspace

# Validate content
cargo run --bin esotericwebb -- validate --content content/

# Preview (text mode, no primals required)
cargo run --bin esotericwebb -- preview --content content/

# Full BYOB niche (requires primal stack from plasmidBin/)
cargo run --bin esotericwebb -- serve --content content/
```

## For Creatives

Esoteric Webb is a **dual-surface** system: developers work in Rust,
creatives work in YAML. No Rust, no engine license, no publisher required.

- `esotericwebb validate` — lint your content for broken refs and dead ends
- `esotericwebb preview` — play through in text mode, iterate immediately
- `esotericwebb graph` — visualize your narrative DAG as DOT/SVG
- `esotericwebb new-world` — scaffold a blank world with template YAML

See `specs/CREATOR_PROFILES_AND_SYSTEM_DESIGN.md` for how the system maps
to the creative DNA of teams like ZA/UM (Disco Elysium) and Cliche Studio
(Esoteric Ebb), and `specs/CONTENT_AUTHORING_SPEC.md` for the YAML format.

## Project structure

```
webb/              Main Rust crate (narrative engine + IPC + director + bridge)
  src/bin/         CLI binary (serve, validate, preview, graph, new-world)
  src/ipc/         JSON-RPC client, bridge, discovery, launcher, resilience
  src/ipc/bridge/  Primal bridge (domains.rs, lifecycle.rs, resilience, tests.rs)
  src/narrative/   Graph, validator, predicate, effect, visualization
  src/director/    Game director (outcome evaluation, DDA integration)
  src/content/     YAML content loader, ability/NPC/scene models
  src/session/     Game session, enrichment pipeline, scene_builder, types
  src/science/     Local game science (flow, engagement, DDA, voice interjections)
  src/niche.rs     Self-knowledge (identity, 27 capabilities, socket resolution)
  src/state/       World state (knowledge, trust, inventory, flags, conditions)
  static/          Frontend HTML (WebGL canvas, SSE, zero external deps)
  capability_registry.toml   All 27 exposed JSON-RPC methods
content/           YAML game content (authored by creative teams)
experiments/       6 standalone validation crates (exp001–exp006)
graphs/            biomeOS deploy graphs (8 TOML compositions + cell graph)
niches/            BYOB niche definition (esoteric-webb.yaml)
deploy/            Systemd units + launcher scripts for ironGate
specs/             Design specifications (7 documents)
docs/              Runtime docs (degradation, Godot modality design)
wateringHole/      Handoffs to primal and spring teams (fossil record)
```

## gen4 — The First Consumer

Esoteric Webb is the first `gen4` entity in the ecoPrimals ecosystem (see
`ecoPrimals/whitePaper/gen4/`). Where gen3 proved the infrastructure computes
correct science across 7 springs, gen4 asks: can people who didn't build the
primals compose them into tools they care about? Webb answers yes — the
primals become invisible infrastructure inside a creative product.

## Quality gates

```bash
cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --lib --tests
cargo doc --workspace --no-deps
cargo llvm-cov --workspace --lib --fail-under-lines 90  # coverage
```

## License

ScyBorg triple license:
- **AGPL-3.0-or-later** — code (see `LICENSE`)
- **Open RPG Creative (ORC)** — game mechanics (see `LICENSE-ORC`)
- **CC BY-SA 4.0** — documentation and creative content (see `LICENSE-CC-BY-SA`)
