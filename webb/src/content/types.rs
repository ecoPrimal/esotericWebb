// SPDX-License-Identifier: AGPL-3.0-or-later
//! Content data model — YAML-authored types for worlds, scenes, NPCs, and abilities.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::narrative::effect::StateEffect;
use crate::narrative::predicate::StatePredicate;

/// World metadata.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldMeta {
    /// World name.
    pub name: String,
    /// Author or team name.
    pub author: String,
    /// Content version.
    pub version: String,
    /// Short description.
    pub description: String,
}

/// A location/world definition (loaded from `worlds/*.yaml`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LocationDef {
    /// Location identifier (derived from filename if not present).
    #[serde(default)]
    pub id: String,
    /// Display name of the location.
    #[serde(default)]
    pub name: String,
    /// Location description.
    #[serde(default)]
    pub description: String,
    /// Tags for categorization (e.g. "indoor", "urban", "occult").
    #[serde(default)]
    pub tags: Vec<String>,
    /// Ambient properties (mood, lighting, sound).
    #[serde(default)]
    pub ambient: HashMap<String, String>,
    /// Connected locations (exit targets).
    #[serde(default)]
    pub exits: Vec<String>,
}

/// A scene's content (loaded from YAML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneContent {
    /// Scene identifier (matches narrative node `content_ref`).
    pub id: String,
    /// Scene description text.
    pub description: String,
    /// NPCs present in this scene (by ID).
    #[serde(default)]
    pub npcs: Vec<String>,
    /// Items available in this scene (by ID).
    #[serde(default)]
    pub items: Vec<String>,
}

/// An ability/spell definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityDef {
    /// Ability identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Description.
    pub description: String,
    /// Preconditions to use this ability.
    #[serde(default)]
    pub preconditions: Vec<StatePredicate>,
    /// Effects when the ability is used.
    #[serde(default)]
    pub effects: Vec<StateEffect>,
    /// Hint for AI narration.
    #[serde(default)]
    pub narration_hint: Option<String>,
}

/// An NPC personality certificate — trust thresholds, knowledge, and dialogue hooks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpcDef {
    /// NPC identifier (short form, e.g. "maren").
    pub id: String,
    /// Display name.
    pub name: String,
    /// Narrative role.
    #[serde(default)]
    pub role: String,
    /// Knowledge this NPC possesses (for reference / AI prompting).
    #[serde(default)]
    pub knows: Vec<String>,
    /// Starting trust value.
    #[serde(default)]
    pub trust_initial: i32,
    /// Trust threshold rewards — ordered by threshold level.
    #[serde(default)]
    pub trust_rewards: Vec<NpcTrustReward>,
    /// Topics this NPC lies about.
    #[serde(default)]
    pub lies_about: HashMap<String, LieInfo>,
    /// Arc description (for AI narration context).
    #[serde(default)]
    pub arc: String,
}

impl NpcDef {
    /// Find the highest reward at or below the given trust level.
    #[must_use]
    pub fn reward_at_threshold(&self, trust: i32) -> Option<&NpcTrustReward> {
        self.trust_rewards
            .iter()
            .filter(|r| r.threshold <= trust)
            .max_by_key(|r| r.threshold)
    }

    /// Find all rewards that have been newly unlocked at the given trust level.
    #[must_use]
    pub fn rewards_between(&self, old_trust: i32, new_trust: i32) -> Vec<&NpcTrustReward> {
        self.trust_rewards
            .iter()
            .filter(|r| r.threshold > old_trust && r.threshold <= new_trust)
            .collect()
    }
}

/// What an NPC reveals at a given trust threshold.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NpcTrustReward {
    /// Trust level at which this reward unlocks.
    #[serde(default)]
    pub threshold: i32,
    /// Human-readable description of what happens.
    #[serde(default)]
    pub description: String,
    /// Knowledge granted to the player.
    #[serde(default)]
    pub grants_knowledge: Vec<String>,
    /// Items given to the player.
    #[serde(default)]
    pub grants_items: Vec<String>,
    /// Flags set on the game state.
    #[serde(default)]
    pub sets_flags: Vec<String>,
}

/// Information about a topic an NPC lies about.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LieInfo {
    /// Difficulty class to detect the lie.
    #[serde(default)]
    pub detection_dc: i32,
}

/// A per-plane ruleset certificate — defines the mechanical rules for a
/// narrative plane (Investigation, Dialogue, Tactical, Crafting, etc.).
///
/// Evolves GAP-009: replaces opaque `serde_json::Value` with a typed model
/// that validates at load time rather than deferring to runtime checks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RulesetCert {
    /// The narrative plane this ruleset governs (e.g. "investigation", "combat").
    pub plane: String,
    /// Human-readable description of this ruleset.
    #[serde(default)]
    pub description: String,
    /// Ruleset version (semver-ish).
    #[serde(default)]
    pub version: String,
    /// The rules comprising this ruleset.
    pub rules: Vec<RuleEntry>,
    /// Difficulty modifiers (optional, per-plane tuning knobs).
    #[serde(default)]
    pub modifiers: Vec<RuleModifier>,
}

/// A single rule within a ruleset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleEntry {
    /// Unique rule identifier within this plane.
    pub id: String,
    /// Human-readable name.
    #[serde(default)]
    pub name: String,
    /// What this rule does when triggered.
    #[serde(default)]
    pub effect: String,
    /// Conditions under which this rule applies.
    #[serde(default)]
    pub conditions: Vec<String>,
    /// Priority for conflict resolution (higher = evaluated first).
    #[serde(default)]
    pub priority: i32,
}

/// A difficulty/tuning modifier for a ruleset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleModifier {
    /// Modifier identifier.
    pub id: String,
    /// Target parameter being modified.
    #[serde(default)]
    pub target: String,
    /// Modification factor (e.g. 1.5 for +50%).
    #[serde(default = "default_factor")]
    pub factor: f64,
    /// Description.
    #[serde(default)]
    pub description: String,
}

/// Default modifier factor (no change).
const fn default_factor() -> f64 {
    1.0
}

impl RulesetCert {
    /// Validate structural correctness of this ruleset.
    #[must_use]
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.rules.is_empty() {
            issues.push(format!("ruleset '{}': no rules defined", self.plane));
        }
        for (i, rule) in self.rules.iter().enumerate() {
            if rule.id.is_empty() {
                issues.push(format!(
                    "ruleset '{}': rule[{i}] has empty 'id'",
                    self.plane
                ));
            }
        }
        issues
    }
}

/// Trait for content types that have an ID field.
pub trait HasId {
    /// Return the content type's unique identifier.
    fn get_id(&self) -> &str;
}

impl HasId for NpcDef {
    fn get_id(&self) -> &str {
        &self.id
    }
}

impl HasId for AbilityDef {
    fn get_id(&self) -> &str {
        &self.id
    }
}

impl HasId for SceneContent {
    fn get_id(&self) -> &str {
        &self.id
    }
}

impl HasId for LocationDef {
    fn get_id(&self) -> &str {
        &self.id
    }
}

impl HasId for RulesetCert {
    fn get_id(&self) -> &str {
        &self.plane
    }
}
