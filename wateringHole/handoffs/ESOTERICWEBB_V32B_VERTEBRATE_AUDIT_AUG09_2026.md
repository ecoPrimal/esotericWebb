<!-- SPDX-License-Identifier: CC-BY-SA-4.0 -->
# AAR: esotericWebb V32b — Vertebrate Self-Audit (Wave 157a)

- **Date**: 2026-08-09
- **Version**: V32b
- **Gate**: ironGate (downstream host, NUCLEUS)
- **Wave**: 157a (Vertebrate Evolution)
- **Outcome**: **P0 impacts mitigated, self-audit PASS, Cell LIVE 9/9**

---

## Context

Wave 157a exposed 3 P0 issues from westGate's 7-session retrospective:
- P0-A: bearDog depot binary returns health for all methods (sign surface missing)
- P0-B: nestGate `content.ingest` doesn't exist (API surface diverged)
- P0-C: biomeOS `capability.call` leaks FDs (14→58K after 4 calls)

This session assessed esotericWebb's exposure to each P0 and executed the
"vertebrate self-audit" directive: verify actual RPC surface matches registry.

## P0-A Impact: crypto.sign Parameter Mismatch (FIXED)

**Finding**: Our bridge called `crypto.sign` with `{"payload": "..."}` (raw
string). bearDog's actual API expects `{"message": "<base64>"}`.

**Evidence on ironGate**: bearDog v0.9.0 on our membrane IS functional (not a
stub — it has real Ed25519 signing). But our param format was wrong, so
signatures were never produced.

**Fix applied**:
- `crypto_sign(&str)` now base64-encodes the message internally
- `crypto_verify(message, signature, public_key)` typed properly
- Added `base64` v0.23.1 crate dependency
- Tests updated to match new signatures

**Status**: Provenance vertices will now include real Ed25519 signatures
when bearDog is connected (9/9 on ironGate).

## P0-B Impact: Not Affected

**Finding**: esotericWebb does NOT use `content.ingest` — our storage path
calls `storage.store` and `storage.retrieve` (both confirmed working on
nestGate v0.5.0 behind BTSP auth). The P0-B `content.ingest` issue is
specific to westGate's pipeline (bulk directory ingestion).

## P0-C Impact: Signal Dispatch Bypassed (MITIGATED)

**Finding**: `nest_store()` and `nest_commit()` used `neural_api_call()` which
routes through biomeOS `capability.call` — the FD-leaking codepath.

**Mitigation**: Bypassed signal dispatch entirely. Both methods now go
directly to `dag.event.append` and `dag.session.complete` (rhizoCrypt) via
established UDS connections. Zero Neural API forwarding calls in data path.

**When to re-enable**: Once biomeOS ships the FD leak fix, the signal
dispatch path can be restored (it provides atomicity across 4 primals
in a single orchestrated call).

## Self-Audit: PASS

| Check | Result |
|-------|--------|
| `niche::CAPABILITIES` count | 28 (matches registry) |
| `capability_registry.toml` methods | 28 (all have handlers) |
| `introspection.rs` descriptors | 28 (all annotated) |
| Unknown method response | `-32601 Method not found` (NOT health stub) |
| Dead/stub methods | 0 |
| Parameter name mismatches | 1 FIXED (crypto.sign payload→message) |
| Methods behind auth | Correct (session mutations require session) |

## Additional Fixes

- `LimitNOFILE=65536` added to `esotericwebb.service`
- UFW rule for port 8090 reloaded (was present but not in iptables)
- Membrane socket permissions re-fixed after reboot
- 3 dead membrane primals restarted (petaltongue, squirrel, sweetgrass)

## Stats

- 484 tests, 0 clippy warnings, 0 unsafe
- Cell: LIVE (9/9 domains connected)
- webb.primals.eco: 200 OK
- Zero `capability.call` usage in data path (P0-C safe)

## Upstream Issues (for overwatch)

### P1: Membrane socket permissions don't persist reboots

The `chgrp biomeos /run/membrane/*.sock` fix resets on every reboot.
Membrane systemd template unit needs:
```ini
ExecStartPost=/bin/sh -c 'chgrp biomeos /run/membrane/%i.sock; chmod 770 /run/membrane/%i.sock'
```
**Owner**: cellMembrane team

### P2: petaltongue/squirrel/sweetgrass die and don't auto-restart

These 3 membrane services stopped and stayed dead. Their unit needs
`Restart=on-failure` (currently only `Type=simple` with no restart).
**Owner**: gate operations / membrane team

### P3: bearDog crypto.sign param documentation

bearDog's actual API: `crypto.sign({"message": "<base64>"})` →
`{"signature": "...", "public_key": "...", "algorithm": "Ed25519"}`.
This should be documented in bearDog's `capability_registry.toml`.
**Owner**: bearDog team
