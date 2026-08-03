// SPDX-License-Identifier: AGPL-3.0-or-later
//! IPC client for the AI primal (`ai.*` capabilities).
//!
//! Consumes `ai.*` methods for narration, dialogue, and analysis.
//! Method names align with the biomeOS capability registry
//! (`ai.query`, `ai.suggest`, `ai.analyze`) which routes to Squirrel's
//! native methods (`query`, `suggest`, `analyze`).
//!
//! The primal binary is resolved from `plasmidBin/` — no spring source
//! dependency. The `PrimalBridge` handles all IPC with graceful degradation.

use serde::{Deserialize, Serialize};

/// AI query (narration, dialogue, chat). Maps to Squirrel `query`.
pub const METHOD_AI_QUERY: &str = "ai.query";
/// AI suggest (summarization, short-form generation). Maps to Squirrel `suggest`.
pub const METHOD_AI_SUGGEST: &str = "ai.suggest";
/// AI analyze (voice checks, classification). Maps to Squirrel `analyze`.
pub const METHOD_AI_ANALYZE: &str = "ai.analyze";
/// Signal plan (G18 dispatch). Squirrel plans multi-step actions via tools.
pub const METHOD_SIGNAL_PLAN: &str = "signal.plan";
/// Graph validate (composition validation). Squirrel validates deploy graphs.
pub const METHOD_GRAPH_VALIDATE: &str = "graph.validate";

/// Chat response from the AI primal.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// Generated text.
    pub text: String,
    /// Model used for generation.
    pub model: String,
    /// Token count.
    pub tokens: u32,
}

/// NPC dialogue response (constructed from AI primal output).
///
/// When Webb calls `ai.query` with NPC personality context, it wraps
/// the response into this structure for the enrichment pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DialogueResponse {
    /// The NPC's spoken text.
    pub text: String,
    /// Internal voice interjections (if any fired).
    pub voice_notes: Vec<VoiceNote>,
    /// Whether any passive checks triggered.
    pub passive_checks_fired: bool,
    /// Whether this response is a degradation placeholder (primal unavailable).
    #[serde(default)]
    pub degraded: bool,
}

/// A voice interjection from the internal voice system.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VoiceNote {
    /// Voice identifier (e.g. "logic", "empathy").
    pub voice_id: String,
    /// The voice's interjection text.
    pub text: String,
    /// Priority (lower = more important).
    pub priority: u32,
}

/// A tool definition for signal.plan — describes an action squirrel can invoke.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    /// Tool name (e.g. "session.act", "webb.scene.current").
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON Schema for the tool's parameters.
    #[serde(default, skip_serializing_if = "serde_json::Value::is_null")]
    pub parameters: serde_json::Value,
}

/// Response from `signal.plan` — a planned sequence of tool calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalPlanResponse {
    /// The planned tool invocations in order.
    pub steps: Vec<PlannedStep>,
    /// Reasoning for the plan (optional, model-generated).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reasoning: Option<String>,
}

/// A single step in a signal plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedStep {
    /// Tool to invoke.
    pub tool: String,
    /// Arguments to pass to the tool.
    pub args: serde_json::Value,
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "tests use unwrap for brevity")]
mod tests {
    use super::*;

    #[test]
    fn dialogue_response_serde_round_trip() {
        let resp = DialogueResponse {
            text: "Hello traveler".to_owned(),
            voice_notes: vec![VoiceNote {
                voice_id: "logic".to_owned(),
                text: "Be careful.".to_owned(),
                priority: 1,
            }],
            passive_checks_fired: true,
            degraded: false,
        };
        let json = serde_json::to_string(&resp).unwrap();
        let back: DialogueResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.text, "Hello traveler");
        assert!(!back.degraded);
        assert_eq!(back.voice_notes.len(), 1);
        assert!(back.passive_checks_fired);
    }

    #[test]
    fn dialogue_degraded_defaults_to_false() {
        let json = r#"{"text":"hi","voice_notes":[],"passive_checks_fired":false}"#;
        let resp: DialogueResponse = serde_json::from_str(json).unwrap();
        assert!(!resp.degraded);
    }

    #[test]
    fn signal_plan_response_serde_round_trip() {
        let plan = SignalPlanResponse {
            steps: vec![
                PlannedStep {
                    tool: "session.act".to_owned(),
                    args: serde_json::json!({"kind": "exit", "id": "parlor"}),
                },
                PlannedStep {
                    tool: "session.act".to_owned(),
                    args: serde_json::json!({"kind": "talk", "id": "maren"}),
                },
            ],
            reasoning: Some("Explore the parlor then talk to Maren.".to_owned()),
        };
        let json = serde_json::to_string(&plan).unwrap();
        let back: SignalPlanResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(back.steps.len(), 2);
        assert_eq!(back.steps[0].tool, "session.act");
        assert!(back.reasoning.is_some());
    }

    #[test]
    fn tool_def_serde() {
        let tool = ToolDef {
            name: "session.act".to_owned(),
            description: "Perform a player action".to_owned(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": {"type": "string"},
                    "id": {"type": "string"}
                }
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        let back: ToolDef = serde_json::from_str(&json).unwrap();
        assert_eq!(back.name, "session.act");
        assert!(!back.parameters.is_null());
    }
}
