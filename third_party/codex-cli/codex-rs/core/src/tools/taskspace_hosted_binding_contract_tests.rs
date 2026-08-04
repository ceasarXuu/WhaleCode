use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_protocol::models::ResponseItem;
use serde::Deserialize;

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct HostedScopeFragment {
    hosted_node_id: String,
}

#[derive(Debug, PartialEq, Eq)]
struct HostedFact {
    provider_item_id: Option<String>,
    item_type: &'static str,
    provider_status: Option<String>,
}

#[derive(Debug, PartialEq, Eq)]
enum BindingDisposition {
    Bound,
    AlreadyBound,
    Unbound,
    MissingProviderIdentity,
    DuplicateInResponse,
    ConflictingExistingBinding,
}

#[derive(Debug, PartialEq, Eq)]
struct BindingObservation {
    fact: HostedFact,
    declared_node_id: Option<String>,
    disposition: BindingDisposition,
}

#[derive(Default)]
struct BindingLedger {
    provider_item_to_node: BTreeMap<String, String>,
}

impl BindingLedger {
    fn reconcile_response(
        &mut self,
        scope: Option<&HostedScopeFragment>,
        items: &[ResponseItem],
    ) -> Vec<BindingObservation> {
        let declared_node_id = scope
            .map(|scope| scope.hosted_node_id.trim())
            .filter(|node_id| !node_id.is_empty());
        let mut response_ids = BTreeSet::new();

        items
            .iter()
            .filter_map(hosted_fact)
            .map(|fact| {
                let disposition = match fact.provider_item_id.as_deref() {
                    None | Some("") => BindingDisposition::MissingProviderIdentity,
                    Some(provider_item_id)
                        if !response_ids.insert(provider_item_id.to_string()) =>
                    {
                        BindingDisposition::DuplicateInResponse
                    }
                    Some(provider_item_id) => match (
                        self.provider_item_to_node.get(provider_item_id),
                        declared_node_id,
                    ) {
                        (Some(existing), Some(node_id)) if existing == node_id => {
                            BindingDisposition::AlreadyBound
                        }
                        (Some(_), Some(_)) => BindingDisposition::ConflictingExistingBinding,
                        (Some(_), None) => BindingDisposition::AlreadyBound,
                        (None, Some(node_id)) => {
                            self.provider_item_to_node
                                .insert(provider_item_id.to_string(), node_id.to_string());
                            BindingDisposition::Bound
                        }
                        (None, None) => BindingDisposition::Unbound,
                    },
                };
                BindingObservation {
                    fact,
                    declared_node_id: declared_node_id.map(str::to_string),
                    disposition,
                }
            })
            .collect()
    }
}

fn hosted_fact(item: &ResponseItem) -> Option<HostedFact> {
    match item {
        ResponseItem::WebSearchCall { id, status, .. } => Some(HostedFact {
            provider_item_id: id.clone(),
            item_type: "web_search_call",
            provider_status: status.clone(),
        }),
        ResponseItem::ImageGenerationCall { id, status, .. } => Some(HostedFact {
            provider_item_id: Some(id.clone()),
            item_type: "image_generation_call",
            provider_status: Some(status.clone()),
        }),
        _ => None,
    }
}

fn web(id: Option<&str>, status: &str) -> ResponseItem {
    ResponseItem::WebSearchCall {
        id: id.map(str::to_string),
        status: Some(status.to_string()),
        action: None,
    }
}

fn image(id: &str, status: &str) -> ResponseItem {
    ResponseItem::ImageGenerationCall {
        id: id.to_string(),
        status: status.to_string(),
        revised_prompt: None,
        result: "fixture-image".to_string(),
    }
}

#[test]
fn scope_fragment_declares_one_node_and_never_asks_the_agent_for_provider_ids() {
    let scope: HostedScopeFragment =
        serde_json::from_value(serde_json::json!({"hosted_node_id": "research"}))
            .expect("single hosted node scope");
    assert_eq!(scope.hosted_node_id, "research");

    assert!(
        serde_json::from_value::<HostedScopeFragment>(serde_json::json!({
            "hosted_node_id": ["research", "design"]
        }))
        .is_err()
    );
    assert!(
        serde_json::from_value::<HostedScopeFragment>(serde_json::json!({
            "hosted_node_id": "research",
            "provider_item_id": "call_00_provider_owned"
        }))
        .is_err()
    );
}

#[test]
fn one_scope_binds_every_hosted_fact_without_reading_tool_outcome() {
    let scope = HostedScopeFragment {
        hosted_node_id: "research".to_string(),
    };
    let mut ledger = BindingLedger::default();
    let observations = ledger.reconcile_response(
        Some(&scope),
        &[
            web(Some("call_00"), "completed"),
            web(Some("call_01"), "failed"),
            image("ig_00", "completed"),
        ],
    );

    assert_eq!(observations.len(), 3);
    assert!(
        observations
            .iter()
            .all(|item| item.disposition == BindingDisposition::Bound)
    );
    assert_eq!(
        observations
            .iter()
            .map(|item| item.fact.provider_status.as_deref())
            .collect::<Vec<_>>(),
        vec![Some("completed"), Some("failed"), Some("completed")]
    );
    assert_eq!(
        ledger.provider_item_to_node,
        BTreeMap::from([
            ("call_00".to_string(), "research".to_string()),
            ("call_01".to_string(), "research".to_string()),
            ("ig_00".to_string(), "research".to_string()),
        ])
    );
}

#[test]
fn missing_scope_preserves_every_provider_fact_as_unbound() {
    let mut ledger = BindingLedger::default();
    let observations = ledger.reconcile_response(
        None,
        &[web(Some("call_00"), "completed"), image("ig_00", "failed")],
    );

    assert_eq!(observations.len(), 2);
    assert!(
        observations
            .iter()
            .all(|item| item.disposition == BindingDisposition::Unbound)
    );
    assert!(ledger.provider_item_to_node.is_empty());
}

#[test]
fn duplicate_missing_and_replayed_ids_are_mechanical_facts() {
    let scope = HostedScopeFragment {
        hosted_node_id: "research".to_string(),
    };
    let mut ledger = BindingLedger::default();
    let first = ledger.reconcile_response(
        Some(&scope),
        &[
            web(Some("call_00"), "completed"),
            web(Some("call_00"), "failed"),
            web(None, "completed"),
        ],
    );
    assert_eq!(first.len(), 3);
    assert_eq!(first[0].disposition, BindingDisposition::Bound);
    assert_eq!(
        first[1].disposition,
        BindingDisposition::DuplicateInResponse
    );
    assert_eq!(
        first[2].disposition,
        BindingDisposition::MissingProviderIdentity
    );

    let replay = ledger.reconcile_response(Some(&scope), &[web(Some("call_00"), "completed")]);
    assert_eq!(replay[0].disposition, BindingDisposition::AlreadyBound);
    assert_eq!(ledger.provider_item_to_node.len(), 1);
}

#[test]
fn an_existing_provider_identity_cannot_be_rebound_to_another_node() {
    let research = HostedScopeFragment {
        hosted_node_id: "research".to_string(),
    };
    let design = HostedScopeFragment {
        hosted_node_id: "design".to_string(),
    };
    let mut ledger = BindingLedger::default();
    ledger.reconcile_response(Some(&research), &[web(Some("call_00"), "completed")]);

    let conflict = ledger.reconcile_response(Some(&design), &[web(Some("call_00"), "completed")]);
    assert_eq!(
        conflict[0].disposition,
        BindingDisposition::ConflictingExistingBinding
    );
    assert_eq!(
        ledger
            .provider_item_to_node
            .get("call_00")
            .map(String::as_str),
        Some("research")
    );
}
