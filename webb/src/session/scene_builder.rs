// SPDX-License-Identifier: AGPL-3.0-or-later
//! Scene graph builder for petalTongue's `visualization.render.scene`.
//!
//! Pure functions that construct petalTongue-compatible `SceneGraph` JSON with
//! typed `SceneNode` payloads (Rect, Text, Point primitives) and animation
//! metadata (fade-in, staggered NPC entrance).

/// Build a single NPC scene node with circle avatar and name label.
#[expect(
    clippy::cast_precision_loss,
    reason = "NPC count is realistically small"
)]
fn build_npc_scene_node(index: usize, npc: &str) -> (String, serde_json::Value) {
    let npc_id = format!("npc_{npc}");
    let x = (index as f64).mul_add(150.0, 100.0);
    let y = 400.0;

    let hue = (index as f64 * 0.3).fract();
    let npc_color = [hue.mul_add(0.4, 0.3), hue.mul_add(0.3, 0.5), 0.7, 1.0];

    let node = serde_json::json!({
        "id": npc_id,
        "transform": {"tx": x, "ty": y, "sx": 1.0, "sy": 1.0, "rotation": 0.0},
        "primitives": [
            {
                "Point": {
                    "x": 0.0, "y": 0.0, "radius": 25.0,
                    "fill": {"Rgba": npc_color},
                    "stroke": {"color": {"Rgba": [0.9, 0.9, 0.9, 0.8]}, "width": 2.0},
                    "data_id": npc
                }
            },
            {
                "Text": {
                    "x": 0.0, "y": 35.0, "content": npc,
                    "font_size": 12.0,
                    "color": {"Rgba": [0.85, 0.85, 0.90, 1.0]},
                    "anchor": "TopCenter", "bold": false, "italic": false,
                    "data_id": null
                }
            }
        ],
        "children": [],
        "visible": true,
        "opacity": 1.0,
        "label": npc,
        "data_source": npc
    });
    (npc_id, node)
}

/// Build scene chrome: background, title, description, and turn indicator.
fn build_scene_chrome(
    node_id: &str,
    description: &str,
    turn: u32,
    is_ending: bool,
) -> (Vec<String>, serde_json::Map<String, serde_json::Value>) {
    let mut ids = vec![];
    let mut nodes = serde_json::Map::new();

    let bg_color = if is_ending {
        [0.15, 0.08, 0.08, 1.0]
    } else {
        [0.06, 0.07, 0.09, 1.0]
    };
    nodes.insert(
        "background".to_owned(),
        serde_json::json!({
            "id": "background",
            "transform": {"tx": 0.0, "ty": 0.0, "sx": 1.0, "sy": 1.0, "rotation": 0.0},
            "primitives": [{"Rect": {
                "x": 0.0, "y": 0.0, "width": 800.0, "height": 600.0,
                "fill": {"Rgba": bg_color},
                "stroke": null, "corner_radius": 0.0, "data_id": null
            }}],
            "children": [], "visible": true, "opacity": 1.0,
            "label": "scene_background", "data_source": null
        }),
    );
    ids.push("background".to_owned());

    let title_text = if is_ending {
        format!("[ENDING] {node_id}")
    } else {
        node_id.to_owned()
    };
    nodes.insert(
        "title".to_owned(),
        serde_json::json!({
            "id": "title",
            "transform": {"tx": 20.0, "ty": 30.0, "sx": 1.0, "sy": 1.0, "rotation": 0.0},
            "primitives": [{"Text": {
                "x": 0.0, "y": 0.0, "content": title_text,
                "font_size": 24.0,
                "color": {"Rgba": [0.88, 0.92, 0.96, 1.0]},
                "anchor": "TopLeft", "bold": true, "italic": false, "data_id": null
            }}],
            "children": [], "visible": true, "opacity": 1.0,
            "label": "scene_title", "data_source": null
        }),
    );
    ids.push("title".to_owned());

    let desc_short = if description.len() > 200 {
        format!("{}...", &description[..197])
    } else {
        description.to_owned()
    };
    nodes.insert(
        "description".to_owned(),
        serde_json::json!({
            "id": "description",
            "transform": {"tx": 20.0, "ty": 70.0, "sx": 1.0, "sy": 1.0, "rotation": 0.0},
            "primitives": [{"Text": {
                "x": 0.0, "y": 0.0, "content": desc_short,
                "font_size": 14.0,
                "color": {"Rgba": [0.75, 0.78, 0.82, 1.0]},
                "anchor": "TopLeft", "bold": false, "italic": false, "data_id": null
            }}],
            "children": [], "visible": true, "opacity": 0.9,
            "label": "scene_description", "data_source": null
        }),
    );
    ids.push("description".to_owned());

    nodes.insert(
        "turn_indicator".to_owned(),
        serde_json::json!({
            "id": "turn_indicator",
            "transform": {"tx": 720.0, "ty": 20.0, "sx": 1.0, "sy": 1.0, "rotation": 0.0},
            "primitives": [{"Text": {
                "x": 0.0, "y": 0.0, "content": format!("Turn {turn}"),
                "font_size": 16.0,
                "color": {"Rgba": [0.53, 0.66, 0.83, 1.0]},
                "anchor": "TopRight", "bold": false, "italic": false, "data_id": null
            }}],
            "children": [], "visible": true, "opacity": 1.0,
            "label": "turn_counter", "data_source": null
        }),
    );
    ids.push("turn_indicator".to_owned());

    (ids, nodes)
}

/// Build a complete `SceneGraph` JSON for petalTongue's `visualization.render.scene`.
///
/// Emits a proper petalTongue `SceneGraph` with typed `SceneNode` payloads
/// containing visual primitives (Rect, Text, Point). Uses 2D layout with
/// orthographic projection (z=0, `Transform3D` optional per Wave 150h).
///
/// Includes animation metadata for scene transitions (fade-in) and NPC
/// entrance effects (staggered slide-up with opacity ramp).
pub(super) fn build_game_scene_graph(
    node_id: &str,
    description: &str,
    npcs: &[String],
    turn: u32,
    is_ending: bool,
) -> serde_json::Value {
    let (mut root_children, mut nodes) = build_scene_chrome(node_id, description, turn, is_ending);

    let mut animations: Vec<serde_json::Value> = vec![];

    animations.push(serde_json::json!({
        "target": {"Opacity": {"node_id": "background", "from": 0.0, "to": 1.0}},
        "duration_secs": 0.4,
        "easing": "EaseInOut",
        "delay_secs": 0.0
    }));
    animations.push(serde_json::json!({
        "target": {"Opacity": {"node_id": "title", "from": 0.0, "to": 1.0}},
        "duration_secs": 0.3,
        "easing": "EaseOut",
        "delay_secs": 0.1
    }));
    animations.push(serde_json::json!({
        "target": {"Opacity": {"node_id": "description", "from": 0.0, "to": 0.9}},
        "duration_secs": 0.5,
        "easing": "EaseOut",
        "delay_secs": 0.2
    }));

    for (i, npc) in npcs.iter().enumerate() {
        let (npc_id, node) = build_npc_scene_node(i, npc);
        nodes.insert(npc_id.clone(), node);
        root_children.push(npc_id.clone());

        #[expect(clippy::cast_precision_loss, reason = "NPC count is small")]
        let i_f = i as f64;
        let delay = i_f.mul_add(0.12, 0.3);
        let x = i_f.mul_add(150.0, 100.0);
        animations.push(serde_json::json!({
            "target": {"Translate": {"node_id": npc_id, "from": [x, 440.0], "to": [x, 400.0]}},
            "duration_secs": 0.4,
            "easing": "EaseOut",
            "delay_secs": delay
        }));
        animations.push(serde_json::json!({
            "target": {"Opacity": {"node_id": npc_id, "from": 0.0, "to": 1.0}},
            "duration_secs": 0.3,
            "easing": "EaseIn",
            "delay_secs": delay
        }));
    }

    nodes.insert(
        "root".to_owned(),
        serde_json::json!({
            "id": "root",
            "transform": {"tx": 0.0, "ty": 0.0, "sx": 1.0, "sy": 1.0, "rotation": 0.0},
            "primitives": [],
            "children": root_children,
            "visible": true,
            "opacity": 1.0,
            "label": node_id,
            "data_source": null
        }),
    );

    serde_json::json!({
        "root": "root",
        "nodes": nodes,
        "animations": animations
    })
}
