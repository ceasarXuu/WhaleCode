use super::*;
use codex_protocol::models::ContentItem;

fn message(text: &str) -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "user".into(),
        content: vec![ContentItem::InputText { text: text.into() }],
        end_turn: None,
        phase: None,
    }
}

#[test]
fn reuses_same_scope_at_the_original_anchor() {
    let first = vec![message("task")];
    let epoch = TaskSpaceProviderProjectionEpoch::new(
        "active:map-1".into(),
        "projection-r2".into(),
        first.len(),
        &first,
    )
    .unwrap();
    let mut grown = first;
    grown.push(message("result"));

    assert_eq!(
        decide_taskspace_projection_epoch(Some(&epoch), "active:map-1", &grown).unwrap(),
        TaskSpaceProjectionEpochDecision::Reuse {
            context: "projection-r2".into(),
            anchor: 1,
        }
    );
}

#[test]
fn scope_change_replaces_the_same_anchor_but_prefix_change_starts_a_new_epoch() {
    let first = vec![message("task")];
    let epoch = TaskSpaceProviderProjectionEpoch::new(
        "bootstrap".into(),
        "blank".into(),
        first.len(),
        &first,
    )
    .unwrap();
    let mut grown = first;
    grown.push(message("initialized"));
    assert_eq!(
        decide_taskspace_projection_epoch(Some(&epoch), "active:map-1", &grown).unwrap(),
        TaskSpaceProjectionEpochDecision::Refresh {
            anchor: 1,
            reason: "projection_scope_changed",
        }
    );

    let replaced = vec![message("compacted")];
    assert_eq!(
        decide_taskspace_projection_epoch(Some(&epoch), "bootstrap", &replaced).unwrap(),
        TaskSpaceProjectionEpochDecision::Refresh {
            anchor: 1,
            reason: "anchor_prefix_changed",
        }
    );
}
