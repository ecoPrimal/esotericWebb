// SPDX-License-Identifier: AGPL-3.0-or-later
//! Domain-specific IPC delegations.
//!
//! Each primal domain gets a section of 1-3 line methods that map to
//! the generic call helpers in [`super`]. Keeping these delegations
//! in their own file prevents the bridge module from growing beyond
//! the 1000-line ecosystem limit as new domains are wired.
//!
//! ## Architecture (V6+ — direct primal composition)
//!
//! All calls go directly to the underlying primals:
//!
//! - **AI domain** (Squirrel): narration, NPC dialogue, voice checks,
//!   summarization — using biomeOS semantic methods (`ai.query`,
//!   `ai.suggest`, `ai.analyze`).
//! - **Visualization** (petalTongue): scene rendering, input polling.
//! - **Compute** (ToadStool): GPU dispatch.
//! - **Storage** (NestGate): key-value store.
//! - **DAG** (rhizoCrypt): provenance, session lifecycle.
//! - **Lineage** (LoamSpine): certificates.
//!
//! Game science (flow, engagement, DDA) is now local via `science/`.

use crate::ipc::envelope::IpcError;
use crate::ipc::petaltongue::{
    InputEvent, METHOD_INTERACTION_POLL, METHOD_RENDER_GRAMMAR, METHOD_RENDER_SCENE,
    METHOD_UI_RENDER,
};
use crate::ipc::primal_names::domain;
use crate::ipc::squirrel::{
    ChatResponse, DialogueResponse, METHOD_AI_ANALYZE, METHOD_AI_QUERY, METHOD_AI_SUGGEST,
    METHOD_SIGNAL_PLAN, SignalPlanResponse, ToolDef, VoiceNote,
};
use crate::ipc::{
    METHOD_CERT_MINT, METHOD_COMPUTE_SUBMIT, METHOD_CRYPTO_HASH, METHOD_CRYPTO_SIGN,
    METHOD_CRYPTO_VERIFY, METHOD_DAG_EVENT_APPEND, METHOD_DAG_EVENT_BATCH, METHOD_DAG_FRONTIER_GET,
    METHOD_DAG_MERKLE_ROOT, METHOD_DAG_QUERY_VERTICES, METHOD_DAG_SESSION_COMPLETE,
    METHOD_DAG_SESSION_CREATE, METHOD_GOSSIP_INJECT, METHOD_MESH_BONDS, METHOD_MESH_HEALTH,
    METHOD_MESH_QUERY, METHOD_MESH_TOPOLOGY, METHOD_STORAGE_RETRIEVE, METHOD_STORAGE_STORE,
};

use super::{PrimalBridge, degraded_summary_limit};

impl PrimalBridge {
    // ── AI domain (Squirrel) ────────────────────────────────

    /// Generate narration via the AI primal (`ai.query`).
    ///
    /// Degrades to a placeholder if Squirrel is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn ai_narrate(&mut self, prompt: &str) -> Result<ChatResponse, IpcError> {
        let params = serde_json::json!({
            "messages": [{"role": "user", "content": prompt}],
        });
        let default = ChatResponse {
            text: format!("[AI primal unavailable — narration placeholder for: {prompt}]"),
            model: "none".to_owned(),
            tokens: 0,
        };
        self.call_or_default(domain::AI, METHOD_AI_QUERY, params, default)
    }

    /// Summarize context via the AI primal (`ai.suggest`).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn ai_summarize(&mut self, context: &str) -> Result<String, IpcError> {
        if self.has(domain::AI) {
            let params = serde_json::json!({ "text": context });
            if let Ok(resp) = self.resilient_call(domain::AI, METHOD_AI_SUGGEST, params) {
                if let Some(serde_json::Value::String(text)) = resp.result {
                    return Ok(text);
                }
            }
        }
        let truncated: String = context.chars().take(degraded_summary_limit()).collect();
        Ok(format!("[degraded: summary unavailable] {truncated}..."))
    }

    /// NPC dialogue via AI primal (`ai.query` with NPC personality context).
    ///
    /// Webb formats the NPC context into a prompt and calls Squirrel
    /// directly via biomeOS semantic methods.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn npc_dialogue(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<DialogueResponse, IpcError> {
        if !self.has(domain::AI) {
            return Ok(DialogueResponse {
                text: "[AI primal unavailable — NPC dialogue degraded]".to_owned(),
                voice_notes: Vec::new(),
                passive_checks_fired: false,
                degraded: true,
            });
        }

        let npc_id = params
            .get("npc_id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let context = params
            .get("context")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        let prompt =
            format!("You are NPC '{npc_id}' in a CRPG. Respond in character.\nContext: {context}");

        let query_params = serde_json::json!({
            "messages": [{"role": "user", "content": prompt}],
        });
        match self.resilient_call(domain::AI, METHOD_AI_QUERY, query_params) {
            Ok(resp) => {
                if let Some(result) = resp.result {
                    if let Ok(chat) = serde_json::from_value::<ChatResponse>(result) {
                        return Ok(DialogueResponse {
                            text: chat.text,
                            voice_notes: Vec::new(),
                            passive_checks_fired: false,
                            degraded: false,
                        });
                    }
                }
                Ok(DialogueResponse {
                    text: "[AI response could not be parsed]".to_owned(),
                    voice_notes: Vec::new(),
                    passive_checks_fired: false,
                    degraded: true,
                })
            }
            Err(_) => Ok(DialogueResponse {
                text: "[AI primal unavailable — NPC dialogue degraded]".to_owned(),
                voice_notes: Vec::new(),
                passive_checks_fired: false,
                degraded: true,
            }),
        }
    }

    /// Narrate an action via AI primal (`ai.suggest`).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn narrate_action(&mut self, params: &serde_json::Value) -> Result<ChatResponse, IpcError> {
        let action = params
            .get("action")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let outcome = params
            .get("outcome")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");

        let suggest_params = serde_json::json!({
            "text": format!("Narrate: {action}. Outcome: {outcome}"),
        });
        self.call_or_default(
            domain::AI,
            METHOD_AI_SUGGEST,
            suggest_params,
            ChatResponse {
                text: "[AI primal unavailable — narration degraded]".to_owned(),
                model: "none".to_owned(),
                tokens: 0,
            },
        )
    }

    /// Internal voice check via AI primal (`ai.analyze`).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn voice_check(&mut self, params: &serde_json::Value) -> Result<Vec<VoiceNote>, IpcError> {
        self.call_or_default(domain::AI, METHOD_AI_ANALYZE, params.clone(), Vec::new())
    }

    /// Plan a multi-step action via squirrel's `signal.plan` (G18 dispatch).
    ///
    /// Sends a prompt and available tools to squirrel, which returns a planned
    /// sequence of tool invocations. Returns `None` if the AI domain is absent
    /// or does not support `signal.plan`.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] on transport failure.
    pub fn signal_plan(
        &mut self,
        prompt: &str,
        tools: &[ToolDef],
    ) -> Result<Option<SignalPlanResponse>, IpcError> {
        if !self.has(domain::AI) {
            return Ok(None);
        }
        let params = serde_json::json!({
            "prompt": prompt,
            "tools": tools,
        });
        match self.resilient_call(domain::AI, METHOD_SIGNAL_PLAN, params) {
            Ok(resp) => {
                if let Some(result) = resp.result {
                    if let Ok(plan) = serde_json::from_value::<SignalPlanResponse>(result) {
                        return Ok(Some(plan));
                    }
                }
                Ok(None)
            }
            Err(IpcError::MethodNotFound { .. }) => Ok(None),
            Err(e) => Err(e),
        }
    }

    // ── Visualization domain (petalTongue) ─────────────────

    /// Push a scene payload for rendering via the full `SceneGraph` protocol.
    ///
    /// Returns an error if the visualization domain rejects the scene format,
    /// allowing callers to fall back to simpler rendering methods.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the domain is absent or the call fails.
    pub fn render_scene(&mut self, scene: &serde_json::Value) -> Result<(), IpcError> {
        if !self.has(domain::VISUALIZATION) {
            return Err(IpcError::PrimalNotFound {
                domain: domain::VISUALIZATION.to_owned(),
            });
        }
        let _ = self.resilient_call(domain::VISUALIZATION, METHOD_RENDER_SCENE, scene.clone())?;
        Ok(())
    }

    /// Push a grammar-of-graphics expression for compilation and rendering.
    ///
    /// petalTongue compiles the `GrammarExpr` + inline data into a scene graph
    /// and renders it in the requested modality (svg, webgl, terminal).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the visualization domain is absent or rejects the grammar.
    pub fn render_grammar(
        &mut self,
        grammar: &serde_json::Value,
        data: &[serde_json::Value],
        modality: &str,
    ) -> Result<serde_json::Value, IpcError> {
        if !self.has(domain::VISUALIZATION) {
            return Err(IpcError::PrimalNotFound {
                domain: domain::VISUALIZATION.to_owned(),
            });
        }
        let params = serde_json::json!({
            "session_id": "esotericwebb-grammar",
            "grammar": grammar,
            "data": data,
            "modality": modality,
        });
        let resp = self.resilient_call(domain::VISUALIZATION, METHOD_RENDER_GRAMMAR, params)?;
        Ok(resp.result.unwrap_or(serde_json::Value::Null))
    }

    /// Render content via petalTongue's `ui.render` — simpler than the full
    /// `SceneGraph` protocol and confirmed working (v1.6.6+).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn render_ui(&mut self, payload: serde_json::Value) -> Result<bool, IpcError> {
        let result: serde_json::Value = self.call_or_default(
            domain::VISUALIZATION,
            METHOD_UI_RENDER,
            payload,
            serde_json::json!({"rendered": false}),
        )?;
        Ok(result
            .get("rendered")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false))
    }

    /// Poll for player input events from petalTongue.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn poll_input(&mut self) -> Result<Vec<InputEvent>, IpcError> {
        self.call_or_default(
            domain::VISUALIZATION,
            METHOD_INTERACTION_POLL,
            serde_json::Value::Null,
            Vec::new(),
        )
    }

    // ── Compute domain (ToadStool) ────────────────────────

    /// Submit a compute task.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn compute_submit(&mut self, task: &serde_json::Value) -> Result<Option<String>, IpcError> {
        self.call_extract_id(
            domain::COMPUTE,
            METHOD_COMPUTE_SUBMIT,
            task.clone(),
            &["job_id"],
        )
    }

    // ── Storage domain (NestGate) ─────────────────────────

    /// Store a value.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn store(&mut self, key: &str, value: &serde_json::Value) -> Result<bool, IpcError> {
        if !self.has(domain::STORAGE) {
            return Ok(false);
        }
        let params = serde_json::json!({ "key": key, "value": value });
        let resp = self.resilient_call(domain::STORAGE, METHOD_STORAGE_STORE, params)?;
        Ok(resp.error.is_none())
    }

    /// Retrieve a value.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn retrieve(&mut self, key: &str) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(
            domain::STORAGE,
            METHOD_STORAGE_RETRIEVE,
            serde_json::json!({ "key": key }),
        )
    }

    // ── Provenance / DAG domain (rhizoCrypt) ────────────────

    /// Append a provenance vertex to the session DAG.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn provenance_append(&mut self, vertex: &serde_json::Value) -> Result<bool, IpcError> {
        if !self.has(domain::DAG) {
            return Ok(false);
        }
        let resp = self.resilient_call(domain::DAG, METHOD_DAG_EVENT_APPEND, vertex.clone())?;
        Ok(resp.error.is_none())
    }

    /// Create a new session on the DAG primal.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn dag_session_create(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<String>, IpcError> {
        self.call_extract_id(
            domain::DAG,
            METHOD_DAG_SESSION_CREATE,
            params.clone(),
            &["session_id", "id"],
        )
    }

    /// Append an event to a session DAG.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn dag_event_append(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<String>, IpcError> {
        if !self.has(domain::DAG) {
            return Ok(None);
        }
        let resp = self.resilient_call(domain::DAG, METHOD_DAG_EVENT_APPEND, params.clone())?;
        if let Some(result) = resp.result {
            for field in &["vertex_id", "id"] {
                if let Some(id) = result.get(*field).and_then(serde_json::Value::as_str) {
                    return Ok(Some(id.to_owned()));
                }
            }
            return Ok(Some("ok".to_owned()));
        }
        Ok(None)
    }

    /// Batch-append multiple events to a session DAG in one RPC call.
    ///
    /// Uses `dag.event.batch` (rhizoCrypt G31) for high-throughput provenance.
    /// Falls back to sequential `dag.event.append` if batch is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn dag_event_batch(
        &mut self,
        events: &[serde_json::Value],
    ) -> Result<Vec<Option<String>>, IpcError> {
        if !self.has(domain::DAG) {
            return Ok(vec![None; events.len()]);
        }

        let mut all_results = Vec::with_capacity(events.len());

        for chunk in events.chunks(crate::ipc::DAG_BATCH_MAX) {
            let params = serde_json::json!({ "events": chunk });
            match self.resilient_call(domain::DAG, METHOD_DAG_EVENT_BATCH, params) {
                Ok(resp) => {
                    if let Some(result) = resp.result {
                        if let Some(ids) = result.get("vertex_ids").and_then(|v| v.as_array()) {
                            all_results
                                .extend(ids.iter().map(|v| v.as_str().map(str::to_owned)));
                            continue;
                        }
                    }
                    all_results.extend(std::iter::repeat_n(None, chunk.len()));
                }
                Err(e) if e.is_method_not_found() => {
                    tracing::debug!(
                        "dag.event.batch not available — falling back to sequential"
                    );
                    for event in chunk {
                        all_results.push(self.dag_event_append(event)?);
                    }
                }
                Err(e) => return Err(e),
            }
        }
        Ok(all_results)
    }

    /// Get the current frontier of a session DAG.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn dag_frontier_get(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(domain::DAG, METHOD_DAG_FRONTIER_GET, params.clone())
    }

    /// Get the Merkle root of a session DAG.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn dag_merkle_root(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<String>, IpcError> {
        self.call_extract_id(
            domain::DAG,
            METHOD_DAG_MERKLE_ROOT,
            params.clone(),
            &["root", "merkle_root"],
        )
    }

    /// Complete a session DAG.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn dag_session_complete(&mut self, params: &serde_json::Value) -> Result<(), IpcError> {
        self.call_fire(domain::DAG, METHOD_DAG_SESSION_COMPLETE, params.clone())
    }

    /// Query vertices from a session DAG.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn dag_query_vertices(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(domain::DAG, METHOD_DAG_QUERY_VERTICES, params.clone())
    }

    // ── Lineage domain (LoamSpine) ──────────────────────────

    /// Mint a certificate via `LoamSpine`.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn mint_certificate(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(domain::LINEAGE, METHOD_CERT_MINT, params.clone())
    }

    // ── Provenance / attribution domain (sweetGrass) ──────────

    /// Create an attribution braid (sweetGrass).
    ///
    /// Braids weave action → provenance → lineage → attribution into a
    /// single traceable record. Returns the braid ID if successful.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn attribution_create(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<String>, IpcError> {
        self.call_extract_id(
            domain::PROVENANCE,
            "braid.create",
            params.clone(),
            &["braid_id", "id"],
        )
    }

    /// Query attribution records for a session or content item.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn attribution_query(
        &mut self,
        params: &serde_json::Value,
    ) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(domain::PROVENANCE, "braid.query", params.clone())
    }

    // ── Crypto domain (bearDog) ─────────────────────────────

    /// Sign a message with the session key (bearDog Ed25519).
    ///
    /// `message` is the raw string to sign — this method base64-encodes it
    /// before sending to bearDog (which requires base64 `message` param).
    /// Returns the base64 signature string. Degrades to `None` when bearDog
    /// is unavailable — the game continues without cryptographic sealing.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn crypto_sign(&mut self, message: &str) -> Result<Option<String>, IpcError> {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(message.as_bytes());
        let params = serde_json::json!({ "message": encoded });
        self.call_extract_id(domain::CRYPTO, METHOD_CRYPTO_SIGN, params, &["signature", "sig"])
    }

    /// Verify a signed message (bearDog Ed25519).
    ///
    /// Accepts raw `message` string (base64-encodes internally), plus
    /// `signature` and `public_key` (both base64, as returned by `crypto_sign`).
    /// Returns `true` if valid, `false` if bearDog is unavailable
    /// (degradation: unsigned is assumed valid locally).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn crypto_verify(
        &mut self,
        message: &str,
        signature: &str,
        public_key: &str,
    ) -> Result<bool, IpcError> {
        use base64::Engine as _;
        let encoded = base64::engine::general_purpose::STANDARD.encode(message.as_bytes());
        let params = serde_json::json!({
            "message": encoded,
            "signature": signature,
            "public_key": public_key,
        });
        self.call_or_default(domain::CRYPTO, METHOD_CRYPTO_VERIFY, params, false)
    }

    /// Hash arbitrary data for content-addressable identity (bearDog).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn crypto_hash(&mut self, params: &serde_json::Value) -> Result<Option<String>, IpcError> {
        self.call_extract_id(
            domain::CRYPTO,
            METHOD_CRYPTO_HASH,
            params.clone(),
            &["hash", "digest"],
        )
    }

    // ── Mesh / topology domain (songBird) ───────────────────

    /// Query the live ecosystem topology from songBird.
    ///
    /// Returns the full mesh graph: gates, primals, bonds, health
    /// status. This is the primary data source for the first milestone
    /// (static site rendering of ecosystem topology).
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn mesh_topology(&mut self) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(domain::MESH, METHOD_MESH_TOPOLOGY, serde_json::Value::Null)
    }

    /// Query mesh-wide health from songBird.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn mesh_health(&mut self) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(domain::MESH, METHOD_MESH_HEALTH, serde_json::Value::Null)
    }

    /// Query a specific primal's mesh status by name.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn mesh_query(&mut self, primal: &str) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(
            domain::MESH,
            METHOD_MESH_QUERY,
            serde_json::json!({ "primal": primal }),
        )
    }

    /// List active bonds between primals in the mesh.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if the call fails unexpectedly.
    pub fn mesh_bonds(&mut self) -> Result<Option<serde_json::Value>, IpcError> {
        self.call_passthrough(domain::MESH, METHOD_MESH_BONDS, serde_json::Value::Null)
    }

    // ── Signal dispatch (Wave 17 orchestration collapse) ─────

    /// Atomic provenance step via `nest.store` signal.
    ///
    /// When biomeOS routes the signal, it decomposes into:
    /// `NestGate.content.put → rhizoCrypt.dag.event.append → loamSpine.spine.seal → sweetGrass.braid.create`
    ///
    /// Falls back to direct `dag.event.append` if the signal is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if all paths fail.
    pub fn nest_store(&mut self, params: &serde_json::Value) -> Result<bool, IpcError> {
        // P0-C mitigation: biomeOS capability.call leaks FDs (14→58K after 4 calls).
        // Bypass signal dispatch and use direct dag.event.append until P0-C is resolved.
        // The signal path (nest.store → capability.call) will be re-enabled once biomeOS
        // ships the socket cleanup fix.
        self.provenance_append(params)
    }

    /// Atomic session finalization via `nest.commit` signal.
    ///
    /// When biomeOS routes the signal, it decomposes into:
    /// `rhizoCrypt.dehydrate → bearDog.sign → NestGate.store → loamSpine.seal`
    ///
    /// Falls back to direct `dag.session.complete` if the signal is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] if all paths fail.
    pub fn nest_commit(&mut self, params: &serde_json::Value) -> Result<(), IpcError> {
        // P0-C mitigation: biomeOS capability.call leaks FDs.
        // Direct path until fix ships.
        self.dag_session_complete(params)
    }

    // ── Gossip domain (swarmVine) ─────────────────────────────

    /// Inject a gossip event into the local mesh via swarmVine.
    ///
    /// Uses the `gossip.inject` method with `topic` (tower|data|compute),
    /// `key` (dot-namespaced event identifier), and `value` (event payload).
    /// Fire-and-forget: degrades silently if swarmVine is unavailable.
    ///
    /// # Errors
    ///
    /// Returns [`IpcError`] only on unexpected transport failures.
    pub fn gossip_inject(&mut self, topic: &str, key: &str, value: &str) -> Result<(), IpcError> {
        let params = serde_json::json!({
            "topic": topic,
            "key": key,
            "value": value,
        });
        self.call_fire(domain::GOSSIP, METHOD_GOSSIP_INJECT, params)
    }
}
