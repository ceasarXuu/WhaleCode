use codex_protocol::models::FunctionCallOutputBody;
use codex_protocol::models::FunctionCallOutputContentItem;
use codex_protocol::models::FunctionCallOutputPayload;
use codex_protocol::models::ImageDetail;
use codex_protocol::models::ResponseInputItem;
use serde::Deserialize;
use serde::Serialize;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SettlementStatus {
    Succeeded,
    Failed,
    Cancelled,
    Skipped,
    OutcomeUnknown,
    NotExecuted,
    ProtocolRejected,
    Bound,
    AlreadyBound,
    Unbound,
    IdentityMissing,
    BindingConflict,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SettlementKind {
    ClientCall,
    ProviderResult,
    MapCall,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SettlementEntry {
    item_id: String,
    kind: SettlementKind,
    status: SettlementStatus,
    result_ref: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct SequenceSettlement {
    canonical_revision: u64,
    items: Vec<SettlementEntry>,
}

#[test]
fn native_multimodal_tool_output_round_trips_without_text_projection() {
    let expected_body = FunctionCallOutputBody::ContentItems(vec![
        FunctionCallOutputContentItem::InputText {
            text: "exact text\nwith ordering".into(),
        },
        FunctionCallOutputContentItem::InputImage {
            image_url: "data:image/png;base64,fixture".into(),
            detail: Some(ImageDetail::Original),
        },
    ]);
    let output = ResponseInputItem::FunctionCallOutput {
        call_id: "call-image".into(),
        output: FunctionCallOutputPayload {
            body: expected_body.clone(),
            success: Some(false),
        },
    };

    let encoded = serde_json::to_value(&output).expect("serialize native output");
    let decoded: ResponseInputItem =
        serde_json::from_value(encoded).expect("deserialize native output");
    let ResponseInputItem::FunctionCallOutput { output, .. } = decoded else {
        panic!("function output");
    };
    assert_eq!(
        output.success, None,
        "success is internal, body is authoritative"
    );
    assert_eq!(output.body, expected_body);
    assert!(matches!(
        output.body,
        FunctionCallOutputBody::ContentItems(items)
            if matches!(&items[0], FunctionCallOutputContentItem::InputText { text } if text == "exact text\nwith ordering")
                && matches!(&items[1], FunctionCallOutputContentItem::InputImage { image_url, detail: Some(ImageDetail::Original) } if image_url == "data:image/png;base64,fixture")
    ));
}

#[test]
fn settlement_manifest_indexes_results_without_copying_business_content() {
    let settlement = SequenceSettlement {
        canonical_revision: 19,
        items: vec![
            SettlementEntry {
                item_id: "client-1".into(),
                kind: SettlementKind::ClientCall,
                status: SettlementStatus::Failed,
                result_ref: Some("tool-result://call/client-1".into()),
            },
            SettlementEntry {
                item_id: "hosted-call-1".into(),
                kind: SettlementKind::ProviderResult,
                status: SettlementStatus::Bound,
                result_ref: Some("provider-result://call/hosted-call-1".into()),
            },
            SettlementEntry {
                item_id: "map-1".into(),
                kind: SettlementKind::MapCall,
                status: SettlementStatus::Succeeded,
                result_ref: None,
            },
        ],
    };

    let encoded = serde_json::to_value(&settlement).expect("settlement json");
    assert_eq!(encoded["canonical_revision"], 19);
    assert!(encoded["items"][0].get("content").is_none());
    assert!(encoded["items"][0].get("output").is_none());
    assert_eq!(
        serde_json::from_value::<SequenceSettlement>(encoded).expect("settlement round trip"),
        settlement
    );
}

#[test]
fn every_non_success_status_remains_distinct() {
    let statuses = [
        SettlementStatus::Failed,
        SettlementStatus::Cancelled,
        SettlementStatus::Skipped,
        SettlementStatus::OutcomeUnknown,
        SettlementStatus::NotExecuted,
        SettlementStatus::ProtocolRejected,
        SettlementStatus::Unbound,
        SettlementStatus::IdentityMissing,
        SettlementStatus::BindingConflict,
    ];
    let encoded = statuses
        .iter()
        .map(|status| serde_json::to_string(status).expect("status"))
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(encoded.len(), statuses.len());
}

#[test]
fn canonical_revision_exists_only_once_at_the_settlement_boundary() {
    let invalid = serde_json::json!({
        "canonical_revision": 7,
        "items": [{
            "item_id": "map-1",
            "kind": "map_call",
            "status": "succeeded",
            "result_ref": null,
            "canonical_revision": 8
        }]
    });
    assert!(serde_json::from_value::<SequenceSettlement>(invalid).is_err());
}
