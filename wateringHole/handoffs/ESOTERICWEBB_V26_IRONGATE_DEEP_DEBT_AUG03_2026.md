<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# HANDOFF: esotericWebb V23–V26 Deep Debt on ironGate

- **Date**: 2026-08-03
- **Gate**: ironGate (PRIMARY DOWNSTREAM HOST)
- **Wave**: 155p / 156b
- **Direction**: Outbound (Webb → upstream primals + infra)

---

## Summary

Four-version deep debt pass (V23–V26) executing on ironGate as first live
cell boot target. esotericWebb is now 100% pure safe Rust, zero-config
auto-discovers 8/9 primals, and operates as a persistent systemd service.

## What was done

### V23: Pure Rust deps, typed rulesets, zero-config discovery
- Replaced `serde_yaml` (C libyaml wrapper) with `noyalib` (pure Rust YAML 1.2)
- Typed `RulesetCert`, `RuleEntry`, `RuleModifier` (replaced `serde_json::Value`)
- `NpcDef.trust_rewards` evolved from `BTreeMap<i32, _>` to `Vec<NpcTrustReward>`
- REST `/health` probing for petalTongue
- TCP well-known port scan for nestGate
- Chunked transfer encoding support in probe
- Zero-config: 8/9 primals discovered without environment variables

### V24: Live cell boot — BTSP transport, membrane discovery, cell graph
- `PrimalClient::connect_btsp` for sweetGrass riboCipher signal
- `/run/membrane/` UDS socket discovery (5-tier hierarchy)
- `graphs/esotericwebb_cell.toml` — biomeOS composition topology
- Discovery priority logic (user-level sockets > membrane sockets)
- Confirmed 8/9 healthy with zero env vars

### V25: G18 signal.plan, discovery refactor, live cell systemd unit
- Wired `signal.plan` types for squirrel G18 dispatch
- Extracted discovery tests (816→424L production + 393L tests)
- `deploy/esotericwebb.service` — systemd user unit for live cell
- `without_discovery()` pattern confirmed in test infra

### V26: Client test extraction, hardcode elimination
- Extracted client tests (799→401L production + 399L tests)
- CLI `--port` uses `ipc::host_port()` (env-overridable, no hardcoded address)
- Final audit: zero unsafe, zero C deps, zero prod mocks, zero >800L files

## Posture

| Metric | Value |
|--------|-------|
| Tests | 455 |
| Clippy warnings | 0 |
| Files >800L | 0 (max: 751L) |
| Unsafe blocks | 0 (forbid enforced) |
| C dependencies | 0 |
| Production mocks | 0 |
| Live primals | 8/9 |
| Demo result | PASS (8 steps, 4 scene pushes) |

## Upstream gaps (for primal teams)

### toadStool — misconfigured systemd unit (BLOCKING 9/9)
The membrane unit passes `--foreground` which toadStool rejects.
`up` subcommand requires a `<MANIFEST>` argument. Fix: correct the
unit `ExecStart` to match toadStool's actual CLI.

### petalTongue — membrane socket permissions
`/run/membrane/petaltongue.sock` is `root:root srwxr-xr-x`.
User `irongate` cannot connect. Workaround: REST `/health` on :3001
works. Fix: membrane socket should be group-writable for `biomeos` group.

### biomeOS neural-api — GAP-017 persists
Neural API is running but composition dispatch (`nest.store`) untested
end-to-end. Webb routes via direct domain calls (functional, just not
orchestrated).

## Patterns for upstream absorption

1. **BTSP handshake**: `0xEC 0x01` prefix before JSON-RPC on UDS. Required
   by sweetGrass. Other primals should document if they require it.

2. **REST `/health` for non-JSON-RPC primals**: petalTongue exposes REST
   not JSON-RPC. Consumers need an HTTP health probe path. Standard: return
   `{"status":"healthy"}` on `GET /health`.

3. **Test extraction pattern**: Production files stay lean (<800L); tests
   live in sibling `*_tests.rs` files via `#[path = "..._tests.rs"] mod tests;`

4. **Capability-based host resolution**: Never hardcode `127.0.0.1`.
   Use env var (`{PRIMAL}_DEFAULT_HOST`) for container/WireGuard portability.

5. **Discovery priority**: User XDG sockets > membrane sockets > TCP > HTTP.
   Don't overwrite higher-priority endpoints with lower-priority ones.

---

## For overwatch

This handoff is ready for upstream audit. esotericWebb on ironGate is
production-grade as a downstream consumer. The remaining 1/9 gap
(toadStool) is an upstream unit file fix, not a Webb issue.
