// SPDX-License-Identifier: AGPL-3.0-or-later
#![expect(clippy::unwrap_used, reason = "test code")]

use super::*;
use crate::ipc::primal_names::{DOMAIN_PRIMAL_MAP, domain};

#[test]
fn standalone_bridge_has_no_connections() {
    let bridge = PrimalBridge::standalone();
    assert_eq!(bridge.connected_count(), 0);
    assert!(!bridge.has(domain::AI));
    assert!(!bridge.has(domain::VISUALIZATION));
    assert!(!bridge.has(domain::COMPUTE));
    assert!(!bridge.has(domain::STORAGE));
    assert_eq!(bridge.statuses().len(), DOMAIN_PRIMAL_MAP.len());
}

#[test]
fn standalone_ai_narrate_degrades() {
    let mut bridge = PrimalBridge::standalone();
    let resp = bridge.ai_narrate("test prompt").unwrap();
    assert!(resp.text.contains("unavailable"));
    assert_eq!(resp.model, "none");
}

#[test]
fn standalone_ai_summarize_degrades() {
    let mut bridge = PrimalBridge::standalone();
    let resp = bridge.ai_summarize("some context").unwrap();
    assert!(resp.contains("summary unavailable"));
}

#[test]
fn standalone_render_scene_returns_not_found() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.render_scene(&serde_json::json!({"type": "test"}));
    assert!(result.is_err());
    assert!(result.unwrap_err().is_connection_error());
}

#[test]
fn standalone_compute_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.compute_submit(&serde_json::json!({})).unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_store_returns_false() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.store("key", &serde_json::json!("value")).unwrap();
    assert!(!result);
}

#[test]
fn standalone_retrieve_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.retrieve("key").unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_provenance_returns_false() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .provenance_append(&serde_json::json!({"id": "v1"}))
        .unwrap();
    assert!(!result);
}

#[test]
fn standalone_dag_session_create_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.dag_session_create(&serde_json::json!({})).unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_dag_event_append_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .dag_event_append(&serde_json::json!({"session_id": "s1", "data": {}}))
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_dag_frontier_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .dag_frontier_get(&serde_json::json!({"session_id": "s1"}))
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_dag_merkle_root_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .dag_merkle_root(&serde_json::json!({"session_id": "s1"}))
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_npc_dialogue_degrades() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .npc_dialogue(&serde_json::json!({"npc_id": "test"}))
        .unwrap();
    assert!(result.text.contains("degraded"));
    assert!(result.voice_notes.is_empty());
}

#[test]
fn standalone_narrate_action_degrades() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .narrate_action(&serde_json::json!({"action": "test"}))
        .unwrap();
    assert_eq!(result.model, "none");
}

#[test]
fn standalone_voice_check_returns_empty() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .voice_check(&serde_json::json!({"voice_id": "logic"}))
        .unwrap();
    assert!(result.is_empty());
}

#[test]
fn standalone_dag_session_complete_is_noop() {
    let mut bridge = PrimalBridge::standalone();
    assert!(
        bridge
            .dag_session_complete(&serde_json::json!({"session_id": "s1"}))
            .is_ok()
    );
}

#[test]
fn standalone_dag_query_vertices_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    assert!(
        bridge
            .dag_query_vertices(&serde_json::json!({"session_id": "s1"}))
            .unwrap()
            .is_none()
    );
}

#[test]
fn standalone_mint_certificate_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    assert!(
        bridge
            .mint_certificate(&serde_json::json!({"npc": "test"}))
            .unwrap()
            .is_none()
    );
}

#[test]
fn standalone_poll_input_returns_empty() {
    let mut bridge = PrimalBridge::standalone();
    assert!(bridge.poll_input().unwrap().is_empty());
}

#[test]
fn discover_returns_status_for_every_domain() {
    let bridge = PrimalBridge::discover();
    assert_eq!(bridge.statuses().len(), DOMAIN_PRIMAL_MAP.len());
    for s in bridge.statuses() {
        assert!(!s.domain.is_empty());
        assert!(!s.name.is_empty());
        if s.healthy {
            assert!(
                s.transport.is_some(),
                "healthy primal {} must report transport",
                s.name,
            );
        }
    }
}

#[test]
fn standalone_nest_store_falls_back_to_false() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .nest_store(&serde_json::json!({"session_id": "s1", "data": {"action": "test"}}))
        .unwrap();
    assert!(!result);
}

#[test]
fn standalone_nest_commit_falls_back_to_noop() {
    let mut bridge = PrimalBridge::standalone();
    assert!(
        bridge
            .nest_commit(&serde_json::json!({"session_id": "s1"}))
            .is_ok()
    );
}

#[test]
fn standalone_announce_self_is_noop() {
    let mut bridge = PrimalBridge::standalone();
    bridge.announce_self("/tmp/test.sock", &["webb.health", "session.start"]);
    assert!(!bridge.has_neural_api());
}

#[test]
fn standalone_has_no_neural_api() {
    let bridge = PrimalBridge::standalone();
    assert!(!bridge.has_neural_api());
}

// ── Crypto domain (bearDog) tests ───────────────────────

#[test]
fn standalone_crypto_sign_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .crypto_sign(&serde_json::json!({"payload": "test"}))
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_crypto_verify_returns_false() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .crypto_verify(&serde_json::json!({"payload": "test", "sig": "abc"}))
        .unwrap();
    assert!(!result);
}

#[test]
fn standalone_crypto_hash_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .crypto_hash(&serde_json::json!({"data": "content"}))
        .unwrap();
    assert!(result.is_none());
}

// ── Mesh domain (songBird) tests ────────────────────────

#[test]
fn standalone_mesh_topology_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.mesh_topology().unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_mesh_health_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.mesh_health().unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_mesh_query_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.mesh_query("rhizocrypt").unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_mesh_bonds_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge.mesh_bonds().unwrap();
    assert!(result.is_none());
}

// ── Provenance / attribution domain (sweetGrass) tests ──

#[test]
fn standalone_attribution_create_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .attribution_create(&serde_json::json!({"session_id": "s1"}))
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_attribution_query_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let result = bridge
        .attribution_query(&serde_json::json!({"session_id": "s1"}))
        .unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_signal_plan_returns_none() {
    let mut bridge = PrimalBridge::standalone();
    let tools = vec![crate::ipc::squirrel::ToolDef {
        name: "session.act".to_owned(),
        description: "Perform an action".to_owned(),
        parameters: serde_json::json!({}),
    }];
    let result = bridge.signal_plan("test prompt", &tools).unwrap();
    assert!(result.is_none());
}

#[test]
fn standalone_dag_event_batch_returns_nones() {
    let mut bridge = PrimalBridge::standalone();
    let events = vec![
        serde_json::json!({"session_id": "s1", "data": {"action": "look"}}),
        serde_json::json!({"session_id": "s1", "data": {"action": "move"}}),
    ];
    let results = bridge.dag_event_batch(&events).unwrap();
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(Option::is_none));
}
