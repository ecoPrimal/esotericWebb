// SPDX-License-Identifier: AGPL-3.0-or-later
//! `UniBin` subcommand implementations.

mod demo;
mod preview;

use esoteric_webb::content::ContentBundle;
use esoteric_webb::director::GameDirector;
use esoteric_webb::error::WebbError;
use esoteric_webb::state::WorldState;

type Result<T> = esoteric_webb::error::Result<T>;

/// Start the full BYOB niche with game director and IPC server.
///
/// With `--launch`, spawns primal binaries from `plasmidBin/` using the
/// deploy graph before discovering. Without it, connects to running primals.
///
/// When `listen_addr` is provided, also starts a TCP listener (`UniBin` v1.2).
pub fn cmd_serve(
    content_path: &str,
    launch: bool,
    graph_path: &str,
    listen_addr: Option<&str>,
    cell_path: Option<&str>,
    mode: &str,
) -> Result<()> {
    // Launcher owns child processes and kills them on Drop — must outlive the server.
    #[expect(
        clippy::collection_is_never_read,
        reason = "held for Drop semantics, not read access"
    )]
    let _launcher: Option<esoteric_webb::ipc::launcher::PrimalLauncher>;

    let mut bridge = if launch {
        println!("BYOB composition: launching primals from plasmidBin/ ...");
        let mut launcher = esoteric_webb::ipc::launcher::PrimalLauncher::new();

        let graph_exists = std::path::Path::new(graph_path).is_file();
        if graph_exists {
            println!("Deploy graph: {graph_path}");
            launcher.spawn_from_graph(graph_path)?;
        } else {
            println!("No deploy graph at {graph_path} — skipping graph-driven launch");
        }

        let mut bridge = esoteric_webb::ipc::bridge::PrimalBridge::discover();

        for sp in launcher.spawned() {
            if !bridge.has(&sp.domain) {
                if let Ok(client) =
                    esoteric_webb::ipc::client::PrimalClient::connect_tcp(&sp.address, &sp.name)
                {
                    bridge.inject(&sp.domain, client, &format!("tcp:{}", sp.address));
                }
            }
        }

        _launcher = Some(launcher);
        bridge
    } else {
        println!("BYOB composition: discovering primals from plasmidBin/ via Songbird...");
        _launcher = None;
        esoteric_webb::ipc::bridge::PrimalBridge::discover()
    };

    let connected = bridge.connected_count();
    for s in bridge.statuses() {
        let icon = if s.healthy { "+" } else { "-" };
        let transport = s.transport.as_deref().unwrap_or("—");
        println!(
            "  [{icon}] {name} ({domain}) {transport}",
            name = s.name,
            domain = s.domain,
        );
    }
    println!(
        "{connected} primal(s) connected — {mode}",
        mode = if connected == 0 {
            "standalone mode"
        } else {
            "composition mode"
        }
    );

    let attach_mode = mode == "attach";
    let cell_id = if let Some(cp) = cell_path {
        Some(validate_cell_graph(cp, &bridge)?)
    } else {
        if attach_mode {
            return Err(WebbError::Other(
                "--mode attach requires --cell <path> to specify the cell graph".to_owned(),
            ));
        }
        None
    };

    let sock = esoteric_webb::ipc::listener::socket_path();
    announce_to_biomeos(&mut bridge, &sock);

    if attach_mode {
        if let Some(ref id) = cell_id {
            attach_cell(cell_path.unwrap_or_default(), id, &sock, &mut bridge);
        }
    }

    let session = esoteric_webb::session::GameSession::with_bridge(content_path, Some(bridge))?;
    let b = session.bundle();
    println!(
        "Loaded: {} NPC(s), {} ability(ies), {} scene(s), {} narrative node(s)",
        b.npcs.len(),
        b.abilities.len(),
        b.scenes.len(),
        b.narrative.nodes.len(),
    );

    let shared = esoteric_webb::ipc::handlers::new_shared_session();
    {
        let mut guard = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        *guard = Some(session);
    }

    if let Some(addr) = listen_addr {
        let tcp_shared = std::sync::Arc::clone(&shared);
        let addr_owned = addr.to_owned();
        std::thread::spawn(move || {
            if let Err(e) = esoteric_webb::ipc::listener::serve_tcp(&addr_owned, &tcp_shared) {
                eprintln!("TCP listener error: {e}");
            }
        });
        println!("TCP IPC listening on {addr}");
    }

    println!("UDS IPC listening on {}", sock.display());
    println!("Session pre-loaded — connect and call session.state to begin");
    esoteric_webb::ipc::listener::serve(&sock, &shared)
}

/// Self-announce to biomeOS so other primals can discover Webb.
fn announce_to_biomeos(
    bridge: &mut esoteric_webb::ipc::bridge::PrimalBridge,
    sock: &std::path::Path,
) {
    bridge.announce_self(
        &sock.display().to_string(),
        esoteric_webb::niche::CAPABILITIES,
    );
}

/// Validate cell graph health gates against discovered primals.
fn validate_cell_graph(
    cell_path: &str,
    bridge: &esoteric_webb::ipc::bridge::PrimalBridge,
) -> Result<String> {
    let path = std::path::Path::new(cell_path);
    let graph = esoteric_webb::cell::load_cell_graph(path)?;
    let status = esoteric_webb::cell::validate_cell(&graph, bridge);
    println!();
    println!(
        "Cell [{id}] — {health}",
        id = graph.cell.id,
        health = status.health
    );
    for p in &status.required_passed {
        println!("  [+] required: {p}");
    }
    for f in &status.required_failed {
        println!("  [!] MISSING required: {f}");
    }
    for p in &status.optional_present {
        println!("  [+] optional: {p}");
    }
    for d in &status.optional_absent {
        println!("  [-] degraded {}: {}", d.domain, d.degradation);
    }
    println!();
    if status.health == esoteric_webb::cell::CellHealth::Failed {
        return Err(esoteric_webb::error::WebbError::Other(
            "Cell boot FAILED — required primals unavailable".to_owned(),
        ));
    }
    Ok(graph.cell.id)
}

/// Attach this process to biomeOS as a managed cell and start health heartbeat.
fn attach_cell(
    cell_path: &str,
    cell_id: &str,
    sock: &std::path::Path,
    bridge: &mut esoteric_webb::ipc::bridge::PrimalBridge,
) {
    let path = std::path::Path::new(cell_path);
    let graph = match esoteric_webb::cell::load_cell_graph(path) {
        Ok(g) => g,
        Err(e) => {
            eprintln!("cell attach: failed to reload graph: {e}");
            return;
        }
    };
    let status = esoteric_webb::cell::validate_cell(&graph, bridge);
    let sock_str = sock.display().to_string();

    let attached = bridge.cell_attach(cell_id, &graph.cell.name, &sock_str, &status);
    if attached {
        println!("Cell '{cell_id}' attached to biomeOS — managed mode");
    } else {
        println!("Cell '{cell_id}' operating unmanaged (biomeOS cell.attach unavailable)");
    }
}

/// Validate a content directory for correctness.
pub fn cmd_validate(content_path: &str) -> Result<()> {
    let bundle = ContentBundle::load(content_path)?;
    let issues = bundle.validate();
    if issues.is_empty() {
        println!(
            "Content valid: {} NPC(s), {} ability(ies), {} scene(s), {} narrative node(s)",
            bundle.npcs.len(),
            bundle.abilities.len(),
            bundle.scenes.len(),
            bundle.narrative.nodes.len(),
        );
        Ok(())
    } else {
        for issue in &issues {
            eprintln!("  - {issue}");
        }
        Err(WebbError::Validation {
            count: issues.len(),
            summary: issues
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join("; "),
        })
    }
}

/// Text-mode interactive game preview (no primals required).
pub fn cmd_preview(content_path: &str) -> Result<()> {
    let bundle = ContentBundle::load(content_path)?;
    print_load_warnings(&bundle);
    let issues = bundle.validate();
    if !issues.is_empty() {
        for issue in &issues {
            eprintln!("  - {issue}");
        }
        return Err(WebbError::Validation {
            count: issues.len(),
            summary: issues.join("; "),
        });
    }

    println!("=== Esoteric Webb — {} ===", bundle.meta.name);
    println!("by {}", bundle.meta.author);
    println!("{}", bundle.meta.description);
    println!();

    let mut director = GameDirector::new(&bundle)?;
    let mut state = WorldState::new();

    preview::run(&mut director, &mut state, &bundle);
    Ok(())
}

/// Show primal composition status.
///
/// Returns `Result` for command dispatch uniformity — currently infallible.
#[expect(clippy::unnecessary_wraps, reason = "uniform cmd_* Result signature")]
pub fn cmd_status() -> Result<()> {
    println!("Esoteric Webb — primal composition status");
    println!("Discovering primals...\n");

    let bridge = esoteric_webb::ipc::bridge::PrimalBridge::discover();
    let connected = bridge.connected_count();

    for s in bridge.statuses() {
        let disc = if s.discovered { "found" } else { "—" };
        let health = if s.healthy { "healthy" } else { "—" };
        println!(
            "  {name:<14} {domain:<16} {disc:<10} {health}",
            name = s.name,
            domain = s.domain,
        );
    }

    println!(
        "\n{connected}/{total} primal(s) connected",
        total = bridge.statuses().len()
    );

    if connected == 0 {
        println!("Mode: standalone (all degradation paths active)");
    } else {
        println!("Mode: composition");
    }

    Ok(())
}

/// Visualize the narrative DAG.
///
/// Three views:
/// - Bare: the full authored graph (no overlay)
/// - Played: overlay a completed session JSON onto the graph
/// - Live: start a session and show its initial state
///
/// Two formats:
/// - `dot`: graphviz DOT output
/// - `json`: structured 3D graph JSON (nodes with BFS depth, edges with
///   forward/back/lateral classification, session overlay state)
pub fn cmd_graph(
    content_path: &str,
    played_path: Option<&str>,
    live: bool,
    format: &str,
) -> Result<()> {
    let bundle = ContentBundle::load(content_path)?;
    print_load_warnings(&bundle);

    let overlay = if let Some(path) = played_path {
        let json_str = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&json_str)?;
        Some(esoteric_webb::narrative::DagOverlay::from_history_json(
            &json,
            &bundle.narrative,
        )?)
    } else if live {
        let session = esoteric_webb::session::GameSession::new(content_path)?;
        Some(session.dag_overlay())
    } else {
        None
    };

    match format {
        "json" => {
            let graph_json = bundle.narrative.to_graph_json(overlay.as_ref());
            println!(
                "{}",
                serde_json::to_string_pretty(&graph_json).unwrap_or_default()
            );
        }
        _ => {
            if let Some(ref ov) = overlay {
                println!("{}", bundle.narrative.to_dot_overlay(ov));
            } else {
                println!("{}", bundle.narrative.to_dot());
            }
        }
    }
    Ok(())
}

/// Replay a provenance-traced session.
///
/// Requires the provenance trio (rhizoCrypt, loamSpine, sweetGrass) to be
/// fully wired with end-to-end session DAG persistence. See
/// `EVOLUTION_GAPS.md` GAP-004 for tracking.
pub fn cmd_replay(session_path: &str, content_path: &str) -> Result<()> {
    let _bundle = ContentBundle::load(content_path)?;
    Err(WebbError::Other(format!(
        "provenance replay not yet implemented for session '{session_path}'. \
         The provenance trio (rhizoCrypt DAG, loamSpine certificates, sweetGrass attribution) \
         must be end-to-end wired before sessions can be replayed. \
         Track progress in EVOLUTION_GAPS.md GAP-004."
    )))
}

/// Automated playthrough — AI-as-player demonstration.
pub fn cmd_autoplay(content_path: &str, max_turns: u32, json_output: bool) -> Result<()> {
    use esoteric_webb::autoplay::{self, AutoplayConfig};
    use esoteric_webb::session::GameSession;

    let mut session = GameSession::new(content_path)?;

    if !json_output {
        let snap = session.snapshot();
        println!("=== AUTOPLAY: {} ===", session.bundle().meta.name);
        println!("{}", snap.scene_description);
        println!();
    }

    let config = AutoplayConfig {
        max_turns,
        ..AutoplayConfig::default()
    };
    let result = autoplay::run(&mut session, &config)?;

    if json_output {
        let output = serde_json::json!({
            "world": session.bundle().meta.name,
            "ended": result.ended,
            "turns": result.turns,
            "final_node": session.snapshot().current_node,
            "knowledge": session.snapshot().knowledge,
            "inventory": session.snapshot().inventory,
            "flags": session.snapshot().flags,
            "nodes_visited": result.nodes_visited,
            "stale_halt": result.stale_halt,
            "history": serde_json::to_value(session.history()).unwrap_or_default(),
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).unwrap_or_default()
        );
    } else {
        let snap = session.snapshot();
        println!("=== AUTOPLAY COMPLETE ===");
        println!("Ended: {}", result.ended);
        println!("Turns: {}", snap.turn);
        println!("Node: {}", snap.current_node);
        println!("Knowledge: {}", snap.knowledge.join(", "));
        println!("Flags: {}", snap.flags.join(", "));
        println!("Nodes visited: {}", result.nodes_visited);
    }
    Ok(())
}

/// Scaffold a new content directory with template YAML.
pub fn cmd_new_world(output_path: &str) -> Result<()> {
    esoteric_webb::content::scaffold(output_path)
}

/// Run all experiment validation suites (meta-runner).
pub fn cmd_validate_all() -> Result<()> {
    const EXPERIMENTS: &[&str] = &[
        "esotericwebb-exp001",
        "esotericwebb-exp002",
        "esotericwebb-exp003",
        "esotericwebb-exp004",
        "esotericwebb-exp005",
        "esotericwebb-exp006",
    ];

    println!("=== Esoteric Webb — validate --all ===\n");

    let json_mode = std::env::var(esoteric_webb::env_keys::ESOTERICWEBB_JSON)
        .ok()
        .is_some_and(|v| v == "1" || v == "true");

    let mut passed = 0u32;
    let mut failed = 0u32;

    for &pkg in EXPERIMENTS {
        println!("--- {pkg} ---");
        let mut cmd = std::process::Command::new("cargo");
        cmd.args(["run", "--release", "-p", pkg]);
        if json_mode {
            cmd.env("ESOTERICWEBB_JSON", "1");
        }
        match cmd.status() {
            Ok(status) if status.success() => {
                passed += 1;
                println!("  -> PASS");
            }
            Ok(status) => {
                failed += 1;
                let code = status.code().unwrap_or(-1);
                println!("  -> FAIL (exit {code})");
            }
            Err(e) => {
                failed += 1;
                println!("  -> ERROR: {e}");
            }
        }
        println!();
    }

    let total = EXPERIMENTS.len();
    println!("=== SUMMARY ===");
    println!("  {passed}/{total} passed, {failed} failed");

    if failed > 0 {
        Err(WebbError::Validation {
            count: failed as usize,
            summary: format!("{failed} experiment(s) failed"),
        })
    } else {
        Ok(())
    }
}

fn print_load_warnings(bundle: &ContentBundle) {
    for w in &bundle.load_warnings {
        eprintln!("warning: {w}");
    }
}

fn current_scene_npcs(bundle: &ContentBundle, director: &GameDirector) -> Vec<String> {
    let node_id = director.current_node_id();
    bundle
        .narrative
        .get(node_id)
        .and_then(|node| bundle.scenes.get(&node.content_ref))
        .map(|scene| scene.npcs.clone())
        .unwrap_or_default()
}

/// Run a guided demo scenario — E2E verification of live composition.
pub fn cmd_demo(content_path: &str, scenario_path: &str, json: bool) -> Result<()> {
    demo::run(content_path, scenario_path, json)
}
