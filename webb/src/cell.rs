// SPDX-License-Identifier: AGPL-3.0-or-later
//! biomeOS cell graph loader and health-gate validator.
//!
//! A cell graph defines what capabilities this deployment requires, how to
//! validate them (health gates), and what degradation looks like when optional
//! primals are absent. The validator checks discovered primal status against
//! the cell requirements and produces a [`CellStatus`].

use crate::ipc::bridge::PrimalBridge;
use serde::Deserialize;
use std::path::Path;

/// Overall cell health after gate validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellHealth {
    /// All required and optional primals are healthy.
    Live,
    /// All required primals healthy, some optional are absent.
    Degraded,
    /// One or more required (critical) primals are unavailable.
    Failed,
}

impl std::fmt::Display for CellHealth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Live => write!(f, "LIVE"),
            Self::Degraded => write!(f, "DEGRADED"),
            Self::Failed => write!(f, "FAILED"),
        }
    }
}

/// Result of validating the cell graph against live primals.
#[derive(Debug)]
pub struct CellStatus {
    /// Overall health determination.
    pub health: CellHealth,
    /// Which required primals passed their health gate.
    pub required_passed: Vec<String>,
    /// Which required primals failed (blocks LIVE).
    pub required_failed: Vec<String>,
    /// Which optional primals are present and healthy.
    pub optional_present: Vec<String>,
    /// Which optional primals are absent (causes DEGRADED).
    pub optional_absent: Vec<DegradedCapability>,
}

/// An optional primal that is absent, with its declared degradation message.
#[derive(Debug)]
pub struct DegradedCapability {
    /// Domain of the absent primal.
    pub domain: String,
    /// Human-readable degradation description from the cell graph.
    pub degradation: String,
}

/// Parsed cell graph TOML structure.
#[derive(Debug, Deserialize)]
pub struct CellGraph {
    /// Top-level cell definition.
    pub cell: CellDef,
}

/// Cell definition.
#[derive(Debug, Deserialize)]
pub struct CellDef {
    /// Unique cell identifier.
    pub id: String,
    /// Human-readable cell name.
    pub name: String,
    /// Cell mode (e.g. "attach").
    #[serde(default)]
    pub mode: String,
    /// Required capability domains.
    #[serde(default)]
    pub requires: Vec<CellRequirement>,
    /// Optional capability domains (graceful degradation).
    #[serde(default)]
    pub optional: Vec<CellOptional>,
}

/// A required primal capability.
#[derive(Debug, Deserialize)]
pub struct CellRequirement {
    /// Capability domain (e.g. "crypto", "mesh").
    pub domain: String,
    /// Primal slug.
    pub primal: String,
    /// Whether this is critical (blocks cell boot entirely).
    #[serde(default = "default_true")]
    pub critical: bool,
}

/// An optional primal capability with degradation description.
#[derive(Debug, Deserialize)]
pub struct CellOptional {
    /// Capability domain.
    pub domain: String,
    /// Primal slug.
    pub primal: String,
    /// What happens when this primal is absent.
    #[serde(default)]
    pub degradation: String,
}

const fn default_true() -> bool {
    true
}

/// Load and parse a cell graph TOML file.
///
/// # Errors
///
/// Returns an error if the file cannot be read or parsed.
pub fn load_cell_graph(path: &Path) -> Result<CellGraph, crate::error::WebbError> {
    let content = std::fs::read_to_string(path)?;
    toml::from_str(&content).map_err(|e| crate::error::WebbError::Config {
        path: path.display().to_string(),
        detail: e.to_string(),
    })
}

/// Validate a cell graph against discovered primal status.
///
/// Uses `has_direct` for accurate health reporting — Neural API routing
/// is a fallback mechanism, not proof of primal presence.
#[must_use]
pub fn validate_cell(graph: &CellGraph, bridge: &PrimalBridge) -> CellStatus {
    let mut required_passed = Vec::new();
    let mut required_failed = Vec::new();

    for req in &graph.cell.requires {
        if bridge.has_direct(&req.domain) {
            required_passed.push(format!("{} ({})", req.primal, req.domain));
        } else if req.critical {
            required_failed.push(format!("{} ({})", req.primal, req.domain));
        } else {
            required_passed.push(format!(
                "{} ({}, non-critical skip)",
                req.primal, req.domain
            ));
        }
    }

    let mut optional_present = Vec::new();
    let mut optional_absent = Vec::new();

    for opt in &graph.cell.optional {
        if bridge.has_direct(&opt.domain) {
            optional_present.push(format!("{} ({})", opt.primal, opt.domain));
        } else {
            optional_absent.push(DegradedCapability {
                domain: opt.domain.clone(),
                degradation: opt.degradation.clone(),
            });
        }
    }

    let health = if !required_failed.is_empty() {
        CellHealth::Failed
    } else if optional_absent.is_empty() {
        CellHealth::Live
    } else {
        CellHealth::Degraded
    };

    CellStatus {
        health,
        required_passed,
        required_failed,
        optional_present,
        optional_absent,
    }
}

#[cfg(test)]
#[expect(clippy::expect_used, reason = "tests use expect for brevity")]
mod tests {
    use super::*;

    fn sample_graph() -> CellGraph {
        CellGraph {
            cell: CellDef {
                id: "test-cell".to_owned(),
                name: "Test Cell".to_owned(),
                mode: "attach".to_owned(),
                requires: vec![
                    CellRequirement {
                        domain: "crypto".to_owned(),
                        primal: "beardog".to_owned(),
                        critical: true,
                    },
                    CellRequirement {
                        domain: "mesh".to_owned(),
                        primal: "songbird".to_owned(),
                        critical: true,
                    },
                ],
                optional: vec![
                    CellOptional {
                        domain: "visualization".to_owned(),
                        primal: "petaltongue".to_owned(),
                        degradation: "No rendering.".to_owned(),
                    },
                    CellOptional {
                        domain: "storage".to_owned(),
                        primal: "nestgate".to_owned(),
                        degradation: "No persistence.".to_owned(),
                    },
                ],
            },
        }
    }

    #[test]
    fn standalone_bridge_fails_all_required() {
        let bridge = PrimalBridge::standalone();
        let graph = sample_graph();
        let status = validate_cell(&graph, &bridge);
        assert_eq!(status.health, CellHealth::Failed);
        assert_eq!(status.required_failed.len(), 2);
    }

    #[test]
    fn load_real_cell_graph() {
        let path = Path::new("graphs/esotericwebb_cell.toml");
        if path.exists() {
            let graph = load_cell_graph(path).expect("cell graph should parse");
            assert_eq!(graph.cell.id, "esotericwebb");
            assert_eq!(graph.cell.mode, "attach");
            assert!(!graph.cell.requires.is_empty());
            assert!(!graph.cell.optional.is_empty());
        }
    }

    #[test]
    fn cell_health_display() {
        assert_eq!(CellHealth::Live.to_string(), "LIVE");
        assert_eq!(CellHealth::Degraded.to_string(), "DEGRADED");
        assert_eq!(CellHealth::Failed.to_string(), "FAILED");
    }

    #[test]
    fn toml_roundtrip() {
        let toml_str = r#"
[cell]
id = "test"
name = "Test"
mode = "attach"

[[cell.requires]]
domain = "crypto"
primal = "beardog"
critical = true

[[cell.optional]]
domain = "viz"
primal = "petaltongue"
degradation = "No viz."
"#;
        let graph: CellGraph = toml::from_str(toml_str).expect("parse");
        assert_eq!(graph.cell.id, "test");
        assert_eq!(graph.cell.requires.len(), 1);
        assert_eq!(graph.cell.optional.len(), 1);
        assert_eq!(graph.cell.optional[0].degradation, "No viz.");
    }
}
