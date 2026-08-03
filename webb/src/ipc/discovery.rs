// SPDX-License-Identifier: AGPL-3.0-or-later
//! Capability-based primal discovery.
//!
//! Discovers live primals by probing Unix sockets in XDG-compliant
//! directories **and** TCP addresses from environment variables and
//! `plasmidBin/` metadata.
//!
//! Primals are found by **capability**, never by hardcoded name or path.
//! Primal binaries are expected in `ecoPrimals/plasmidBin/` — this module
//! discovers their running sockets or TCP addresses, not their source.
//!
//! Discovery strategy:
//!
//! **UDS** (5-tier implemented, per wateringHole `UNIVERSAL_IPC_STANDARD_V3`):
//! 1. `$BIOMEOS_SOCKET_DIR/<domain>.sock`
//! 2. `$XDG_RUNTIME_DIR/biomeos/<domain>.sock`
//! 3. `/run/user/<uid>/biomeos/<domain>.sock`
//! 4. `/tmp/biomeos-<uid>/<domain>.sock`
//! 5. `/run/membrane/<primal>.sock` (system membrane, needs accessible perms)
//!
//! Tier 6 (Songbird `discovery.query`) is planned but not yet implemented
//! — see `EVOLUTION_GAPS.md` GAP-006.
//!
//! **TCP** (checked first, highest priority):
//! 1. `<PRIMAL>_ADDRESS` env var (e.g. `RHIZOCRYPT_ADDRESS=127.0.0.1:9401`)
//! 2. `<PRIMAL>_JSONRPC_PORT` env var → `127.0.0.1:<port>`
//! 3. `plasmidBin/<primal>/metadata.toml` `[transport]` section

use std::collections::HashMap;
use std::net::TcpStream;
use std::path::PathBuf;
use std::time::Duration;

pub use super::transport::TransportEndpoint;

/// A discovered primal endpoint (UDS and/or TCP).
#[derive(Debug, Clone)]
pub struct PrimalEndpoint {
    /// The domain this primal serves (e.g. "game", "ai", "dag").
    pub domain: String,
    /// Primal name (e.g. "rhizocrypt", "squirrel").
    pub name: String,
    /// Filesystem path to the Unix socket (if found).
    pub socket_path: Option<PathBuf>,
    /// TCP address (host:port) if known.
    pub tcp_addr: Option<String>,
    /// HTTP endpoint URL (for primals using HTTP transport, e.g. songBird `/jsonrpc`).
    pub http_url: Option<String>,
    /// Capabilities advertised by this primal.
    pub capabilities: Vec<String>,
    /// Whether the primal responded to a health check.
    pub healthy: bool,
}

impl PrimalEndpoint {
    /// Create a new endpoint with domain and name, no transports yet.
    fn empty(domain: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            domain: domain.into(),
            name: name.into(),
            socket_path: None,
            tcp_addr: None,
            http_url: None,
            capabilities: Vec::new(),
            healthy: false,
        }
    }

    /// Resolve the best `TransportEndpoint` for this primal.
    ///
    /// Priority: UDS (lowest latency) > TCP (wider platform support).
    /// Returns `None` if no transport is available.
    #[must_use]
    pub fn resolve_transport(&self) -> Option<TransportEndpoint> {
        if let Some(ref path) = self.socket_path {
            return Some(TransportEndpoint::uds(path.to_string_lossy()));
        }
        if let Some(ref addr) = self.tcp_addr {
            return TransportEndpoint::from_tcp_addr(addr);
        }
        None
    }

    /// Return all available transports (may be empty, 1, or 2).
    #[must_use]
    pub fn available_transports(&self) -> Vec<TransportEndpoint> {
        let mut transports = Vec::with_capacity(2);
        if let Some(ref path) = self.socket_path {
            transports.push(TransportEndpoint::uds(path.to_string_lossy()));
        }
        if let Some(ref addr) = self.tcp_addr {
            if let Some(ep) = TransportEndpoint::from_tcp_addr(addr) {
                transports.push(ep);
            }
        }
        transports
    }
}

/// Registry of discovered primals, keyed by domain.
#[derive(Debug, Default)]
pub struct PrimalRegistry {
    /// Endpoints indexed by domain.
    pub by_domain: HashMap<String, PrimalEndpoint>,
    /// Capability to domain mapping.
    pub capability_index: HashMap<String, String>,
}

use super::primal_names::DOMAIN_PRIMAL_MAP;

impl PrimalRegistry {
    /// Discover primals from TCP env vars, `plasmidBin/` metadata, and UDS socket directories.
    #[must_use]
    pub fn discover() -> Self {
        let mut registry = Self::default();

        // Phase 1: TCP discovery from env vars (including HTTP URLs)
        registry.discover_tcp_from_env();

        // Phase 2: TCP/metadata discovery from plasmidBin
        registry.discover_from_plasmid_bin();

        // Phase 3: UDS socket directory scan (fills in socket_path for existing
        // endpoints or creates new ones for primals only reachable via UDS)
        for dir in socket_directories() {
            if dir.is_dir() {
                registry.probe_directory(&dir);
            }
        }

        // Phase 4: Well-known HTTP endpoints (primals using HTTP transport)
        registry.discover_http_well_known();

        // Phase 5: Well-known TCP ports for NDJSON primals
        registry.discover_tcp_well_known();

        registry
    }

    /// Find the endpoint that provides a given capability.
    #[must_use]
    pub fn find_by_capability(&self, capability: &str) -> Option<&PrimalEndpoint> {
        self.capability_index
            .get(capability)
            .and_then(|domain| self.by_domain.get(domain))
    }

    /// Check environment variables for TCP addresses.
    ///
    /// Patterns checked per primal:
    /// - `<PRIMAL>_ADDRESS` (e.g. `RHIZOCRYPT_ADDRESS=127.0.0.1:9401`)
    /// - `<PRIMAL>_JSONRPC_PORT` (e.g. `RHIZOCRYPT_JSONRPC_PORT=9401`)
    /// - `<PRIMAL>_HTTP_ADDRESS` (e.g. `SWEETGRASS_HTTP_ADDRESS=127.0.0.1:9403`)
    fn discover_tcp_from_env(&mut self) {
        for &(domain, name) in DOMAIN_PRIMAL_MAP {
            let upper = name.to_uppercase();

            // Check for HTTP URL first (e.g. SONGBIRD_HTTP_URL=http://127.0.0.1:7780/jsonrpc)
            if let Ok(url) = std::env::var(format!("{upper}{}", crate::env_keys::HTTP_URL_SUFFIX)) {
                let ep = self
                    .by_domain
                    .entry(domain.to_owned())
                    .or_insert_with(|| PrimalEndpoint::empty(domain, name));
                ep.http_url = Some(url);
                continue;
            }

            let addr = std::env::var(format!("{upper}{}", crate::env_keys::ADDR_SUFFIX))
                .ok()
                .or_else(|| {
                    std::env::var(format!("{upper}{}", crate::env_keys::PORT_SUFFIX))
                        .ok()
                        .map(super::host_port)
                })
                .or_else(|| {
                    std::env::var(format!("{upper}{}", crate::env_keys::HTTP_ADDR_SUFFIX)).ok()
                });

            if let Some(tcp_addr) = addr {
                let ep = self
                    .by_domain
                    .entry(domain.to_owned())
                    .or_insert_with(|| PrimalEndpoint::empty(domain, name));
                ep.tcp_addr = Some(tcp_addr);
            }
        }
    }

    /// Scan `plasmidBin/` metadata for transport hints.
    fn discover_from_plasmid_bin(&mut self) {
        let plasmidbins = plasmid_bin_directories();
        for base in &plasmidbins {
            if !base.is_dir() {
                continue;
            }
            let Ok(entries) = std::fs::read_dir(base) else {
                continue;
            };
            for entry in entries.flatten() {
                let meta_path = entry.path().join("metadata.toml");
                if meta_path.is_file() {
                    self.ingest_metadata(&meta_path);
                }
            }
        }
    }

    /// Parse a single `metadata.toml` and merge transport/capability info.
    fn ingest_metadata(&mut self, path: &std::path::Path) {
        let Ok(contents) = std::fs::read_to_string(path) else {
            return;
        };
        let Ok(table) = contents.parse::<toml::Table>() else {
            return;
        };

        let primal_section = table.get("primal").and_then(toml::Value::as_table);
        let caps_section = table.get("capabilities").and_then(toml::Value::as_table);

        let name = primal_section
            .and_then(|p| p.get("name"))
            .and_then(toml::Value::as_str)
            .unwrap_or("");
        let domain = caps_section
            .and_then(|c| c.get("domain"))
            .and_then(toml::Value::as_str)
            .unwrap_or("");

        if name.is_empty() || domain.is_empty() {
            return;
        }

        let methods: Vec<String> = caps_section
            .and_then(|c| c.get("methods"))
            .and_then(toml::Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();

        // Check for [transport] section (future-proofing for explicit TCP hints)
        let transport_section = table.get("transport").and_then(toml::Value::as_table);
        let tcp_addr = transport_section
            .and_then(|t| t.get("tcp_address"))
            .and_then(toml::Value::as_str)
            .map(str::to_owned)
            .or_else(|| {
                transport_section
                    .and_then(|t| t.get("port"))
                    .and_then(toml::Value::as_integer)
                    .map(super::host_port)
            });

        let ep = self
            .by_domain
            .entry(domain.to_owned())
            .or_insert_with(|| PrimalEndpoint::empty(domain, name));

        if ep.tcp_addr.is_none() {
            ep.tcp_addr = tcp_addr;
        }

        if ep.http_url.is_none() {
            ep.http_url = transport_section
                .and_then(|t| t.get("http_url"))
                .and_then(toml::Value::as_str)
                .map(str::to_owned);
        }

        if ep.capabilities.is_empty() {
            ep.capabilities.clone_from(&methods);
        }

        for method in &methods {
            self.capability_index
                .entry(method.clone())
                .or_insert_with(|| domain.to_owned());
        }
    }

    /// Probe a directory for `.sock` files and merge into existing endpoints.
    ///
    /// Resolves the socket file stem in two passes:
    /// 1. Match as a **domain** name (`dag.sock` -> domain "dag")
    /// 2. Match as a **primal slug** reverse-mapped through `DOMAIN_PRIMAL_MAP`
    ///    (`rhizocrypt.sock` -> primal "rhizocrypt" -> domain "dag")
    ///
    /// This handles the ecosystem reality where some primals register
    /// domain-named sockets and others register primal-named sockets.
    fn probe_directory(&mut self, dir: &std::path::Path) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("sock") {
                let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) else {
                    continue;
                };

                let (resolved_domain, resolved_name) = DOMAIN_PRIMAL_MAP
                    .iter()
                    .find(|&&(d, _)| d == file_stem)
                    .or_else(|| DOMAIN_PRIMAL_MAP.iter().find(|&&(_, n)| n == file_stem))
                    .map_or_else(
                        || (file_stem.to_owned(), file_stem.to_owned()),
                        |&(d, n)| (d.to_owned(), n.to_owned()),
                    );

                let ep = self
                    .by_domain
                    .entry(resolved_domain.clone())
                    .or_insert_with(|| PrimalEndpoint::empty(resolved_domain, resolved_name));
                if ep.socket_path.is_none() {
                    ep.socket_path = Some(path);
                }
            }
        }
    }

    /// Probe well-known HTTP endpoints for primals that use HTTP transport.
    ///
    /// Primals that expose HTTP surfaces (JSON-RPC or REST) are probed at
    /// well-known ports. This enables zero-config discovery of web-facing
    /// primals without requiring env vars.
    ///
    /// Discovery is capability-based: if a domain is already reachable via
    /// UDS or TCP, the HTTP probe is skipped.
    fn discover_http_well_known(&mut self) {
        use super::primal_names::domain;

        struct WellKnownHttp {
            domain: &'static str,
            name: &'static str,
            path: &'static str,
            ports: &'static [u16],
        }

        let well_known: &[WellKnownHttp] = &[
            WellKnownHttp {
                domain: domain::MESH,
                name: "songbird",
                path: "/jsonrpc",
                ports: &[7700, 7780, 8080],
            },
            WellKnownHttp {
                domain: domain::VISUALIZATION,
                name: "petaltongue",
                path: "/health",
                ports: &[3001, 3000, 8090],
            },
        ];

        for entry in well_known {
            if let Some(ep) = self.by_domain.get(entry.domain) {
                if ep.http_url.is_some() || ep.tcp_addr.is_some() {
                    continue;
                }
            }

            for &port in entry.ports {
                let addr = format!("{}:{port}", super::default_host());
                let Ok(socket_addr) = addr.parse() else {
                    continue;
                };
                if TcpStream::connect_timeout(&socket_addr, Duration::from_millis(200)).is_ok() {
                    let url = format!("http://{addr}{}", entry.path);
                    let ep = self
                        .by_domain
                        .entry(entry.domain.to_owned())
                        .or_insert_with(|| PrimalEndpoint::empty(entry.domain, entry.name));
                    ep.http_url = Some(url);
                    break;
                }
            }
        }
    }

    /// Probe well-known TCP ports for NDJSON primals (e.g. nestGate).
    ///
    /// For primals that speak raw NDJSON over TCP (not HTTP), this probes
    /// known ports and registers them as TCP addresses for direct connection.
    fn discover_tcp_well_known(&mut self) {
        use super::primal_names::domain;

        const WELL_KNOWN_TCP: &[(&str, &str, &[u16])] = &[
            (domain::STORAGE, "nestgate", &[8080, 8083, 8091]),
        ];

        for &(domain, name, ports) in WELL_KNOWN_TCP {
            if let Some(ep) = self.by_domain.get(domain) {
                if ep.http_url.is_some() || ep.tcp_addr.is_some() {
                    continue;
                }
            }

            for &port in ports {
                let addr = format!("{}:{port}", super::default_host());
                let Ok(socket_addr) = addr.parse() else {
                    continue;
                };
                if TcpStream::connect_timeout(&socket_addr, Duration::from_millis(200)).is_ok() {
                    let ep = self
                        .by_domain
                        .entry(domain.to_owned())
                        .or_insert_with(|| PrimalEndpoint::empty(domain, name));
                    ep.tcp_addr = Some(addr);
                    break;
                }
            }
        }
    }
}

/// Standard socket directory search order.
use super::discovery_paths::{plasmid_bin_directories, socket_directories};

#[cfg(test)]
#[path = "discovery_tests.rs"]
#[expect(clippy::unwrap_used, reason = "test code")]
mod tests;
