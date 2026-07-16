use super::*;
use pretty_assertions::assert_eq;
use serde_json::json;

fn section(cost: &ProviderWireSectionCost, kind: SectionKind) -> &ProviderWireSection {
    &cost.sections[kind.index()]
}

#[test]
fn every_message_is_classified_exactly_once() {
    let wire = json!({
        "messages": [
            {"role": "system", "content": "stable"},
            {
                "role": "developer",
                "content": "TaskSpaceMapEpochSnapshotR6V1:\n- map: none\n- bootstrap_required: true\nTaskSpaceMapEpochSnapshotR6V1 end."
            },
            {"role": "user", "content": "request"},
            {"role": "assistant", "content": "response"},
            {"role": "observer", "content": "other non-tool"},
            {
                "role": "tool",
                "content": "{\"schema_version\":\"TaskSpaceControlResultR6V1\",\"ok\":true}"
            },
            {"role": "tool", "content": "ordinary result"}
        ]
    });

    let cost = ProviderWireSectionCost::measure(&wire, "messages");
    let message_counts = SectionKind::ALL[..5]
        .iter()
        .map(|kind| section(&cost, *kind).count)
        .collect::<Vec<_>>();

    assert_eq!(message_counts, vec![1, 3, 1, 1, 1]);
    assert_eq!(message_counts.iter().sum::<usize>(), 7);
}

#[test]
fn standard_payload_has_zero_taskspace_section_cost() {
    let wire = json!({
        "model": "deepseek-v4-flash",
        "messages": [
            {"role": "system", "content": "stable"},
            {"role": "user", "content": "request"},
            {"role": "tool", "content": "ordinary result"}
        ],
        "tools": [{"type": "function", "function": {"name": "shell"}}],
        "tool_choice": "auto"
    });

    let cost = ProviderWireSectionCost::measure(&wire, "messages");

    for kind in [
        SectionKind::ActiveProjection,
        SectionKind::TaskspaceControlFeedback,
    ] {
        assert_eq!(
            (section(&cost, kind).count, section(&cost, kind).bytes),
            (0, 0)
        );
    }
}

#[test]
fn tool_output_containing_projection_block_remains_ordinary_feedback() {
    let wire = json!({
        "messages": [{
            "role": "tool",
            "content": "source text:\nTaskSpaceMapEpochSnapshotR6V1:\n- map_id: copied-map\n- revision: 41\nTaskSpaceMapEpochSnapshotR6V1 end."
        }]
    });

    let cost = ProviderWireSectionCost::measure(&wire, "messages");

    assert_eq!(section(&cost, SectionKind::ActiveProjection).count, 0);
    assert_eq!(section(&cost, SectionKind::OrdinaryToolFeedback).count, 1);
    assert_eq!(cost.active_projection_identity.count, 0);
}

#[test]
fn tool_output_containing_control_marker_remains_ordinary_feedback() {
    let wire = json!({
        "messages": [{
            "role": "tool",
            "content": "source text: {\"schema_version\":\"TaskSpaceControlResultR6V1\"}"
        }]
    });

    let cost = ProviderWireSectionCost::measure(&wire, "messages");

    assert_eq!(
        section(&cost, SectionKind::TaskspaceControlFeedback).count,
        0
    );
    assert_eq!(section(&cost, SectionKind::OrdinaryToolFeedback).count, 1);
}

#[test]
fn section_bytes_reconcile_with_provider_payload_bytes() {
    let wire = json!({
        "model": "deepseek-v4-pro",
        "stream": true,
        "messages": [
            {"role": "developer", "content": "stable"},
            {"role": "developer", "content": "TaskSpaceMapEpochSnapshotR6V1"},
            {"role": "tool", "content": "TaskSpaceControlResultR6V1"}
        ],
        "tools": [{"type": "function", "function": {"name": "taskspace_control"}}],
        "tool_choice": {"type": "function", "function": {"name": "taskspace_control"}}
    });

    let cost = ProviderWireSectionCost::measure(&wire, "messages");
    let accounted = cost
        .sections
        .iter()
        .map(|section| section.bytes)
        .sum::<usize>();

    assert_eq!(cost.availability, "measured");
    assert_eq!(cost.unavailable_reason, None);
    assert_eq!(cost.section_bytes_total, json_bytes(&wire).len());
    assert_eq!(accounted, cost.section_bytes_total);
}

#[test]
fn missing_message_array_is_explicitly_unavailable_and_reconciled() {
    let wire = json!({
        "model": "deepseek-v4-flash",
        "tools": [],
        "tool_choice": "auto"
    });

    let cost = ProviderWireSectionCost::measure(&wire, "messages");

    assert_eq!(cost.availability, "unavailable");
    assert_eq!(cost.unavailable_reason, Some("message_array_missing"));
    assert_eq!(
        cost.sections
            .iter()
            .map(|section| section.bytes)
            .sum::<usize>(),
        cost.section_bytes_total
    );
    assert_eq!(cost.section_bytes_total, json_bytes(&wire).len());
}

#[test]
fn projection_revision_changes_identity_hash_without_changing_count() {
    let wire = |revision| {
        json!({
            "messages": [{
                "role": "developer",
                "content": format!(
                    "TaskSpaceMapEpochSnapshotR6V1:\n- map_id: map-1\n- revision: {revision}\nTaskSpaceMapEpochSnapshotR6V1 end."
                )
            }]
        })
    };

    let revision_7 = ProviderWireSectionCost::measure(&wire(7), "messages");
    let revision_8 = ProviderWireSectionCost::measure(&wire(8), "messages");
    let left = &revision_7.active_projection_identity;
    let right = &revision_8.active_projection_identity;

    assert_eq!(
        (left.count, left.kind, left.revision),
        (1, "active", Some(7))
    );
    assert_eq!(
        (right.count, right.kind, right.revision),
        (1, "active", Some(8))
    );
    assert_eq!(left.map_id_sha256, right.map_id_sha256);
    assert_ne!(left.projection_sha256, right.projection_sha256);
}

#[test]
fn serialized_section_cost_never_contains_raw_payload_content() {
    let secrets = [
        "raw-system-secret-419",
        "raw-user-secret-527",
        "raw-tool-secret-631",
        "raw-choice-secret-743",
        "raw-model-secret-859",
        "raw-map-secret-967",
    ];
    let wire = json!({
        "model": secrets[4],
        "messages": [
            {"role": "system", "content": secrets[0]},
            {
                "role": "developer",
                "content": format!(
                    "TaskSpaceMapEpochSnapshotR6V1:\n- map_id: {}\n- revision: 9\nTaskSpaceMapEpochSnapshotR6V1 end.",
                    secrets[5]
                )
            },
            {"role": "user", "content": secrets[1]},
            {"role": "tool", "content": secrets[2]}
        ],
        "tools": [{"description": secrets[2]}],
        "tool_choice": {"name": secrets[3]}
    });

    let serialized = serde_json::to_string(&ProviderWireSectionCost::measure(&wire, "messages"))
        .expect("section cost serializes");

    assert!(secrets.iter().all(|secret| !serialized.contains(secret)));
    assert!(serialized.contains("provider-wire-section-cost-v1"));
    assert_eq!(
        serde_json::from_str::<Value>(&serialized).expect("valid JSON")["sections"]
            .as_array()
            .map(Vec::len),
        Some(8)
    );
}
