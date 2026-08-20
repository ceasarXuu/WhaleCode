use std::collections::BTreeMap;
use std::collections::BTreeSet;

use serde_json::Map;
use serde_json::Value;

const APPLY_PATCH: &str = "apply_patch";
const BEGIN_PATCH: &str = "*** Begin Patch";
const END_PATCH: &str = "*** End Patch";

pub(super) fn normalized_apply_patch_candidates(
    arguments: &str,
    positions: &BTreeSet<usize>,
) -> Vec<(String, usize)> {
    let mut candidates = BTreeMap::new();
    for &byte_index in positions {
        let mut candidate = String::with_capacity(arguments.len() + 1);
        candidate.push_str(&arguments[..byte_index]);
        candidate.push('}');
        candidate.push_str(&arguments[byte_index..]);

        let Ok(mut value) = serde_json::from_str::<Value>(&candidate) else {
            continue;
        };
        if normalize_exactly_one_action(&mut value) != Some(()) {
            continue;
        }
        let Ok(normalized) = serde_json::to_string(&value) else {
            continue;
        };
        candidates.entry(normalized).or_insert(byte_index);
    }
    candidates.into_iter().collect()
}

fn normalize_exactly_one_action(plan: &mut Value) -> Option<()> {
    let actions = plan.get_mut("tools")?.as_array_mut()?;
    let mut normalized = 0;
    for action in actions {
        if normalize_action(action) {
            normalized += 1;
        }
    }
    (normalized == 1).then_some(())
}

fn normalize_action(action: &mut Value) -> bool {
    let Some(object) = action.as_object_mut() else {
        return false;
    };
    if normalize_function_style_input(object) {
        return true;
    }
    normalize_collapsed_action(object)
}

fn normalize_function_style_input(action: &mut Map<String, Value>) -> bool {
    if action.len() != 3
        || action.get("tool").and_then(Value::as_str) != Some(APPLY_PATCH)
        || action.get("node_id").and_then(Value::as_str).is_none()
    {
        return false;
    }
    let Some(input) = action.get("input").and_then(Value::as_object) else {
        return false;
    };
    if input.len() != 1 {
        return false;
    }
    let Some(patch) = input.get("cmd").and_then(Value::as_str) else {
        return false;
    };
    if !is_complete_patch(patch) {
        return false;
    }
    action.insert("input".into(), Value::String(patch.into()));
    true
}

fn normalize_collapsed_action(action: &mut Map<String, Value>) -> bool {
    if action.len() != 1 {
        return false;
    }
    let Some(input) = action.get("input").and_then(Value::as_object) else {
        return false;
    };
    if input.len() != 3 || input.get("tool").and_then(Value::as_str) != Some(APPLY_PATCH) {
        return false;
    }
    let Some(node_id) = input.get("node_id").and_then(Value::as_str) else {
        return false;
    };
    let Some(patch) = input.get("cmd").and_then(Value::as_str) else {
        return false;
    };
    if !is_complete_patch(patch) {
        return false;
    }
    *action = Map::from_iter([
        ("tool".into(), Value::String(APPLY_PATCH.into())),
        ("node_id".into(), Value::String(node_id.into())),
        ("input".into(), Value::String(patch.into())),
    ]);
    true
}

fn is_complete_patch(input: &str) -> bool {
    let trimmed = input.trim();
    trimmed.starts_with(BEGIN_PATCH)
        && trimmed.ends_with(END_PATCH)
        && trimmed.matches(BEGIN_PATCH).count() == 1
        && trimmed.matches(END_PATCH).count() == 1
}
