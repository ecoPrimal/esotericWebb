// SPDX-License-Identifier: AGPL-3.0-or-later
//! biomeOS lifecycle and registration methods.
//!
//! Outbound calls to the biomeOS orchestration layer:
//! - `primal.announce` — register self as a composition participant
//! - `graph.cell.attach` — register as a managed cell with health tracking
//! - `graph.cell.health` — push periodic health status updates
//! - `route.register` — register with the mesh router for cross-gate discovery

use super::PrimalBridge;

impl PrimalBridge {
    /// Self-announce to biomeOS (outbound `primal.announce`).
    ///
    /// Registers esotericWebb's socket, capabilities, methods, composition tiers,
    /// cost hints, and latency estimates with the orchestration layer.
    /// Aligned with Wave 45 announce schema (Songbird/BearDog key alignment).
    /// Falls back silently if biomeOS is unavailable.
    pub fn announce_self(&mut self, socket: &str, methods: &[&str]) {
        if !self.has_neural_api() {
            tracing::debug!("No neural-api connection — skipping primal.announce");
            return;
        }
        let params = serde_json::json!({
            "primal": crate::niche::NICHE_NAME,
            "socket": socket,
            "capabilities": ["narrative", "session", "mcp"],
            "methods": methods,
            "composition_tiers": ["nest", "meta"],
            "version": env!("CARGO_PKG_VERSION"),
            "cost_hints": {
                "session.act": "low",
                "session.start": "medium",
                "tools.call": "medium",
                "webb.scene.current": "low",
            },
            "latency_estimates": {
                "session.act": "< 10ms",
                "session.start": "< 50ms",
                "tools.call": "< 100ms",
                "webb.scene.current": "< 5ms",
            },
        });
        match self.neural_api_call("lifecycle", crate::ipc::METHOD_PRIMAL_ANNOUNCE, params) {
            Ok(resp) if resp.error.is_none() => {
                tracing::info!("primal.announce accepted by biomeOS");
            }
            Ok(_) | Err(_) => {
                tracing::debug!("primal.announce not accepted — operating without registration");
            }
        }

        self.register_mesh_route(socket, methods);
    }

    /// Attach to biomeOS as a managed cell (outbound `graph.cell.attach`).
    ///
    /// Sends the cell graph identity, requirements, and current health status
    /// to biomeOS so it can track this workload as a composition participant.
    /// Returns `true` if accepted, `false` if biomeOS is unavailable or the
    /// method is not yet supported (graceful degradation).
    pub fn cell_attach(
        &mut self,
        cell_id: &str,
        cell_name: &str,
        socket: &str,
        status: &crate::cell::CellStatus,
    ) -> bool {
        if !self.has_neural_api() {
            tracing::debug!("No neural-api — cannot attach cell");
            return false;
        }
        let params = serde_json::json!({
            "cell_id": cell_id,
            "cell_name": cell_name,
            "primal": crate::niche::NICHE_NAME,
            "socket": socket,
            "gate": crate::niche::gate_id(),
            "health": status.health.to_string(),
            "required_passed": status.required_passed,
            "required_failed": status.required_failed,
            "optional_present": status.optional_present,
            "degraded": status.optional_absent.iter().map(|d| {
                serde_json::json!({"domain": d.domain, "message": d.degradation})
            }).collect::<Vec<_>>(),
            "version": env!("CARGO_PKG_VERSION"),
        });
        match self.neural_api_call("composition", crate::ipc::METHOD_CELL_ATTACH, params) {
            Ok(resp) if resp.error.is_none() => {
                tracing::info!("graph.cell.attach accepted — cell '{cell_id}' managed");
                true
            }
            Ok(_) | Err(_) => {
                tracing::debug!("graph.cell.attach unavailable — cell operating unmanaged");
                false
            }
        }
    }

    /// Push a cell health update to biomeOS (outbound `graph.cell.health`).
    ///
    /// Called periodically or on status change to keep biomeOS informed of
    /// this cell's health state. Degrades silently when unavailable.
    pub fn cell_health_report(
        &mut self,
        cell_id: &str,
        status: &crate::cell::CellStatus,
    ) -> bool {
        if !self.has_neural_api() {
            return false;
        }
        let params = serde_json::json!({
            "cell_id": cell_id,
            "health": status.health.to_string(),
            "required_passed": status.required_passed,
            "required_failed": status.required_failed,
            "optional_present": status.optional_present,
            "degraded": status.optional_absent.iter().map(|d| {
                serde_json::json!({"domain": d.domain, "message": d.degradation})
            }).collect::<Vec<_>>(),
        });
        match self.neural_api_call("composition", crate::ipc::METHOD_CELL_HEALTH_REPORT, params) {
            Ok(resp) if resp.error.is_none() => true,
            Ok(_) | Err(_) => false,
        }
    }

    /// Register with mesh router for cross-gate capability discovery (Wave 73+).
    ///
    /// Enables other gates in the mesh to discover and invoke esotericWebb's
    /// interactive product capabilities. Includes stability tier metadata
    /// (Wave 75 push model) so the router can prioritize propagation of
    /// stable methods. Gracefully degrades if the mesh router is unavailable
    /// (standalone/single-gate operation continues).
    fn register_mesh_route(&mut self, socket: &str, methods: &[&str]) {
        let params = serde_json::json!({
            "primal": crate::niche::NICHE_NAME,
            "gate": crate::niche::gate_id(),
            "socket": socket,
            "capabilities": crate::niche::CAPABILITIES,
            "methods": methods,
            "version": env!("CARGO_PKG_VERSION"),
            "stability_tiers": {
                "stable": [
                    "health.liveness", "health.readiness", "health.check",
                    "health.version", "health.drain", "identity.get",
                    "capabilities.list", "primal.announce", "primal.info",
                    "webb.health", "webb.liveness", "webb.readiness",
                    "webb.scene.current", "webb.narrative.status", "webb.content.list",
                    "session.start", "session.state", "session.actions",
                    "session.act", "session.history", "session.narrate",
                    "session.graph", "session.metrics", "method.describe"
                ],
                "evolving": ["tools.list", "tools.call"],
            },
            "propagation": "push",
        });
        match self.neural_api_call("routing", crate::ipc::METHOD_ROUTE_REGISTER, params) {
            Ok(resp) if resp.error.is_none() => {
                tracing::info!("route.register accepted — mesh-visible (push propagation)");
            }
            Ok(_) | Err(_) => {
                tracing::debug!("route.register unavailable — single-gate mode");
            }
        }
    }
}
