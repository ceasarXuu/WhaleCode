use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_protocol::models::ResponseItem;

use super::plan::TaskspaceExecHostedRecord;

const WEB_SEARCH_CALL_TYPE: &str = "web_search_call";
const IMAGE_GENERATION_CALL_TYPE: &str = "image_generation_call";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ProviderFactRef {
    pub(crate) response_id: String,
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

pub(crate) fn collect_provider_facts(
    response_id: &str,
    items: &[ResponseItem],
) -> ProviderFactCollection {
    let mut facts = Vec::new();
    let mut findings = Vec::new();
    let response_id = response_id.trim();
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
        if response_id.is_empty() {
            findings.push(finding(
                "provider_response_scope_missing",
                None,
                "Hosted provider fact has no response_id scope",
            ));
            continue;
        }
        let Some(provider_item_id) = provider_item_id.filter(|id| !id.trim().is_empty()) else {
            findings.push(finding(
                "provider_item_identity_missing",
                None,
                format!("Hosted `{provider_item_type}` fact has no provider item ID"),
            ));
            continue;
        };
        facts.push(ProviderFact {
            fact_ref: ProviderFactRef {
                response_id: response_id.to_string(),
                provider_item_type: provider_item_type.to_string(),
                provider_item_id: provider_item_id.to_string(),
            },
            provider_status: status.map(str::to_string),
        });
    }
    facts.sort_by(|left, right| left.fact_ref.cmp(&right.fact_ref));
    for pair in facts.windows(2) {
        if pair[0].fact_ref == pair[1].fact_ref {
            findings.push(finding(
                "provider_fact_duplicate",
                Some(pair[0].fact_ref.clone()),
                "Provider response contains a duplicate hosted fact identity",
            ));
        }
    }
    ProviderFactCollection { facts, findings }
}

pub(crate) fn reconcile_provider_records(
    collection: ProviderFactCollection,
    records: &[TaskspaceExecHostedRecord],
) -> ProviderReconciliationReport {
    let mut findings = collection.findings;
    let mut fact_by_ref = BTreeMap::new();
    for fact in collection.facts {
        if fact_by_ref.insert(fact.fact_ref.clone(), fact).is_some() {
            continue;
        }
    }

    let mut record_refs = BTreeSet::new();
    let mut record_node_by_ref = BTreeMap::new();
    for record in records {
        let fact_ref = ProviderFactRef {
            response_id: record.response_id.trim().to_string(),
            provider_item_type: record.provider_item_type.trim().to_string(),
            provider_item_id: record.provider_item_id.trim().to_string(),
        };
        if fact_ref.response_id.is_empty()
            || fact_ref.provider_item_type.is_empty()
            || fact_ref.provider_item_id.is_empty()
            || record.node_id.trim().is_empty()
        {
            findings.push(finding(
                "provider_record_incomplete",
                Some(fact_ref),
                "Provider record requires response, type, item, and node identities",
            ));
            continue;
        }
        if !record_refs.insert(fact_ref.clone()) {
            findings.push(finding(
                "provider_record_duplicate",
                Some(fact_ref),
                "TaskSpace Exec plan declares a provider fact more than once",
            ));
            continue;
        }
        record_node_by_ref.insert(fact_ref, record.node_id.trim().to_string());
    }

    for fact_ref in fact_by_ref.keys() {
        if !record_node_by_ref.contains_key(fact_ref) {
            findings.push(finding(
                "provider_fact_unbound",
                Some(fact_ref.clone()),
                "Provider hosted fact has no exact TaskSpace node declaration",
            ));
        }
    }
    for fact_ref in record_node_by_ref.keys() {
        if !fact_by_ref.contains_key(fact_ref) {
            findings.push(finding(
                "provider_record_unmatched",
                Some(fact_ref.clone()),
                "TaskSpace provider declaration does not match a provider hosted fact",
            ));
        }
    }

    let bindings = if findings.is_empty() {
        record_node_by_ref
            .into_iter()
            .map(|(fact_ref, node_id)| ProviderBinding { fact_ref, node_id })
            .collect()
    } else {
        Vec::new()
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

    fn record(item_type: &str, item_id: &str, node_id: &str) -> TaskspaceExecHostedRecord {
        TaskspaceExecHostedRecord {
            response_id: "resp-1".to_string(),
            provider_item_type: item_type.to_string(),
            provider_item_id: item_id.to_string(),
            node_id: node_id.to_string(),
        }
    }

    #[test]
    fn mixed_provider_facts_reconcile_by_exact_scoped_identity() {
        let facts = collect_provider_facts(
            "resp-1",
            &[web(Some("ws-1"), "completed"), image("ig-1", "failed")],
        );
        let report = reconcile_provider_records(
            facts,
            &[
                record(WEB_SEARCH_CALL_TYPE, "ws-1", "research"),
                record(IMAGE_GENERATION_CALL_TYPE, "ig-1", "design"),
            ],
        );

        assert!(report.exact);
        assert_eq!(report.bindings.len(), 2);
    }

    #[test]
    fn provider_status_does_not_change_identity_or_node_lifecycle() {
        let completed = collect_provider_facts("resp-1", &[web(Some("ws-1"), "completed")]);
        let failed = collect_provider_facts("resp-1", &[web(Some("ws-1"), "failed")]);

        assert_eq!(completed.facts[0].fact_ref, failed.facts[0].fact_ref);
        assert_ne!(
            completed.facts[0].provider_status,
            failed.facts[0].provider_status
        );
    }

    #[test]
    fn missing_scope_or_item_identity_cannot_be_guessed() {
        let no_scope = collect_provider_facts("", &[web(Some("ws-1"), "completed")]);
        assert_eq!(
            no_scope.findings[0].reason_code,
            "provider_response_scope_missing"
        );
        let no_item = collect_provider_facts("resp-1", &[web(None, "completed")]);
        assert_eq!(
            no_item.findings[0].reason_code,
            "provider_item_identity_missing"
        );
    }

    #[test]
    fn missing_duplicate_wrong_type_and_forged_records_fail_closed() {
        let facts = collect_provider_facts(
            "resp-1",
            &[
                web(Some("ws-1"), "completed"),
                web(Some("ws-1"), "completed"),
                image("ig-1", "completed"),
            ],
        );
        let report = reconcile_provider_records(
            facts,
            &[
                record(IMAGE_GENERATION_CALL_TYPE, "ws-1", "wrong-type"),
                record(WEB_SEARCH_CALL_TYPE, "forged", "research"),
                record(WEB_SEARCH_CALL_TYPE, "forged", "research"),
            ],
        );
        let reasons = report
            .findings
            .iter()
            .map(|finding| finding.reason_code)
            .collect::<BTreeSet<_>>();

        assert!(!report.exact);
        assert!(report.bindings.is_empty());
        assert!(reasons.contains("provider_fact_duplicate"));
        assert!(reasons.contains("provider_record_duplicate"));
        assert!(reasons.contains("provider_fact_unbound"));
        assert!(reasons.contains("provider_record_unmatched"));
    }
}
