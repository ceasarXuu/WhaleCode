use std::collections::BTreeMap;

use codex_protocol::models::ResponseItem;

const WEB_SEARCH_CALL_TYPE: &str = "web_search_call";
const IMAGE_GENERATION_CALL_TYPE: &str = "image_generation_call";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ProviderFactRef {
    pub(crate) provider_item_type: String,
    pub(crate) provider_item_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderFact {
    pub(crate) fact_ref: ProviderFactRef,
    pub(crate) provider_status: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderBinding {
    pub(crate) fact_ref: ProviderFactRef,
    pub(crate) node_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderReconciliationFinding {
    pub(crate) reason_code: &'static str,
    pub(crate) fact_ref: Option<ProviderFactRef>,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderFactCollection {
    pub(crate) facts: Vec<ProviderFact>,
    pub(crate) findings: Vec<ProviderReconciliationFinding>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderReconciliationReport {
    pub(crate) exact: bool,
    pub(crate) bindings: Vec<ProviderBinding>,
    pub(crate) findings: Vec<ProviderReconciliationFinding>,
}

pub(crate) fn collect_provider_facts(items: &[ResponseItem]) -> ProviderFactCollection {
    let mut fact_by_ref = BTreeMap::new();
    let mut findings = Vec::new();
    for item in items {
        let extracted = match item {
            ResponseItem::WebSearchCall { id, status, .. } => {
                Some((WEB_SEARCH_CALL_TYPE, id.as_deref(), status.as_deref()))
            }
            ResponseItem::ImageGenerationCall { id, status, .. } => Some((
                IMAGE_GENERATION_CALL_TYPE,
                Some(id.as_str()),
                Some(status.as_str()),
            )),
            _ => None,
        };
        let Some((provider_item_type, provider_item_id, status)) = extracted else {
            continue;
        };
        let Some(provider_item_id) = provider_item_id.filter(|id| !id.trim().is_empty()) else {
            findings.push(finding(
                "provider_item_identity_missing",
                None,
                format!("Hosted `{provider_item_type}` fact has no provider item ID"),
            ));
            continue;
        };
        let fact_ref = ProviderFactRef {
            provider_item_type: provider_item_type.to_string(),
            provider_item_id: provider_item_id.to_string(),
        };
        let fact = ProviderFact {
            fact_ref: fact_ref.clone(),
            provider_status: status.map(str::to_string),
        };
        if fact_by_ref.insert(fact_ref.clone(), fact).is_some() {
            findings.push(finding(
                "provider_fact_duplicate",
                Some(fact_ref),
                "Provider response contains a duplicate hosted fact identity",
            ));
        }
    }

    ProviderFactCollection {
        facts: fact_by_ref.into_values().collect(),
        findings,
    }
}

pub(crate) fn reconcile_provider_scope(
    collection: ProviderFactCollection,
    hosted_node_id: Option<&str>,
) -> ProviderReconciliationReport {
    let mut findings = collection.findings;
    let hosted_node_id = hosted_node_id
        .map(str::trim)
        .filter(|node_id| !node_id.is_empty());

    if collection.facts.is_empty() {
        if hosted_node_id.is_some() {
            findings.push(finding(
                "provider_scope_without_fact",
                None,
                "TaskSpace declares a hosted node but this response has no hosted fact",
            ));
        }
        return ProviderReconciliationReport {
            exact: findings.is_empty(),
            bindings: Vec::new(),
            findings,
        };
    }

    let bindings = match hosted_node_id {
        Some(node_id) => collection
            .facts
            .into_iter()
            .map(|fact| ProviderBinding {
                fact_ref: fact.fact_ref,
                node_id: node_id.to_string(),
            })
            .collect(),
        None => {
            findings.extend(collection.facts.into_iter().map(|fact| {
                finding(
                    "provider_fact_unbound",
                    Some(fact.fact_ref),
                    "Provider hosted fact has no Agent-declared TaskSpace node",
                )
            }));
            Vec::new()
        }
    };

    ProviderReconciliationReport {
        exact: findings.is_empty(),
        bindings,
        findings,
    }
}

fn finding(
    reason_code: &'static str,
    fact_ref: Option<ProviderFactRef>,
    message: impl Into<String>,
) -> ProviderReconciliationFinding {
    ProviderReconciliationFinding {
        reason_code,
        fact_ref,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
            result: "fixture".to_string(),
        }
    }

    #[test]
    fn agent_declares_one_node_while_runtime_reuses_every_provider_identity() {
        let facts = collect_provider_facts(&[
            web(Some("call-1"), "completed"),
            web(Some("call-2"), "failed"),
            image("image-1", "completed"),
        ]);
        let report = reconcile_provider_scope(facts, Some("research"));

        assert!(report.exact);
        assert_eq!(report.bindings.len(), 3);
        assert!(
            report
                .bindings
                .iter()
                .all(|binding| binding.node_id == "research")
        );
        assert_eq!(
            report
                .bindings
                .iter()
                .map(|binding| binding.fact_ref.provider_item_id.as_str())
                .collect::<Vec<_>>(),
            vec!["image-1", "call-1", "call-2"]
        );
    }

    #[test]
    fn provider_status_does_not_change_identity_or_node_lifecycle() {
        let completed = collect_provider_facts(&[web(Some("call-1"), "completed")]);
        let failed = collect_provider_facts(&[web(Some("call-1"), "failed")]);

        assert_eq!(completed.facts[0].fact_ref, failed.facts[0].fact_ref);
        assert_ne!(
            completed.facts[0].provider_status,
            failed.facts[0].provider_status
        );
    }

    #[test]
    fn missing_agent_scope_preserves_provider_facts_as_unbound() {
        let facts =
            collect_provider_facts(&[web(Some("call-1"), "completed"), image("image-1", "failed")]);
        let report = reconcile_provider_scope(facts, None);

        assert!(!report.exact);
        assert!(report.bindings.is_empty());
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.reason_code == "provider_fact_unbound")
                .count(),
            2
        );
    }

    #[test]
    fn duplicate_and_missing_provider_ids_are_not_guessed() {
        let facts = collect_provider_facts(&[
            web(Some("call-1"), "completed"),
            web(Some("call-1"), "failed"),
            web(None, "completed"),
        ]);
        let report = reconcile_provider_scope(facts, Some("research"));

        assert!(!report.exact);
        assert_eq!(report.bindings.len(), 1);
        assert_eq!(report.bindings[0].fact_ref.provider_item_id, "call-1");
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.reason_code == "provider_fact_duplicate")
        );
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.reason_code == "provider_item_identity_missing")
        );
    }

    #[test]
    fn node_declaration_without_provider_fact_is_detected() {
        let report = reconcile_provider_scope(collect_provider_facts(&[]), Some("research"));

        assert!(!report.exact);
        assert!(report.bindings.is_empty());
        assert_eq!(
            report.findings[0].reason_code,
            "provider_scope_without_fact"
        );
    }
}
