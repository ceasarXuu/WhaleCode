use std::collections::BTreeSet;

use codex_protocol::models::ResponseItem;

use super::plan::TaskspaceExecHostedBinding;

const WEB_SEARCH_CALL_TYPE: &str = "web_search_call";
const WEB_SEARCH_TOOL_NAME: &str = "web_search";
const IMAGE_GENERATION_CALL_TYPE: &str = "image_generation_call";
const IMAGE_GENERATION_TOOL_NAME: &str = "image_generation";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ProviderFactRef {
    pub(crate) provider_item_type: String,
    pub(crate) provider_item_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderFact {
    pub(crate) output_index: usize,
    pub(crate) hosted_index: usize,
    pub(crate) tool: String,
    pub(crate) fact_ref: ProviderFactRef,
    pub(crate) provider_status: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderBinding {
    pub(crate) output_index: usize,
    pub(crate) hosted_index: usize,
    pub(crate) fact_ref: ProviderFactRef,
    pub(crate) node_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ProviderReconciliationFinding {
    pub(crate) reason_code: &'static str,
    pub(crate) output_index: Option<usize>,
    pub(crate) hosted_index: Option<usize>,
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

pub(crate) fn collect_provider_facts<'a>(
    items: impl IntoIterator<Item = (usize, &'a ResponseItem)>,
) -> ProviderFactCollection {
    let mut facts = Vec::new();
    let mut seen_refs = BTreeSet::new();
    let mut seen_output_indexes = BTreeSet::new();
    let mut findings = Vec::new();
    for (output_index, item) in items {
        let extracted = match item {
            ResponseItem::WebSearchCall { id, status, .. } => Some((
                WEB_SEARCH_CALL_TYPE,
                WEB_SEARCH_TOOL_NAME,
                id.as_deref(),
                status.as_deref(),
            )),
            ResponseItem::ImageGenerationCall { id, status, .. } => Some((
                IMAGE_GENERATION_CALL_TYPE,
                IMAGE_GENERATION_TOOL_NAME,
                Some(id.as_str()),
                Some(status.as_str()),
            )),
            _ => None,
        };
        let Some((provider_item_type, tool, provider_item_id, status)) = extracted else {
            continue;
        };
        if !seen_output_indexes.insert(output_index) {
            findings.push(finding(
                "provider_output_index_duplicate",
                Some(output_index),
                None,
                None,
                format!("Provider response repeats output_index {output_index}"),
            ));
        }
        let Some(provider_item_id) = provider_item_id.filter(|id| !id.trim().is_empty()) else {
            findings.push(finding(
                "provider_item_identity_missing",
                Some(output_index),
                None,
                None,
                format!(
                    "Hosted output_index {output_index} (`{provider_item_type}`) has no provider item ID"
                ),
            ));
            continue;
        };
        let fact_ref = ProviderFactRef {
            provider_item_type: provider_item_type.to_string(),
            provider_item_id: provider_item_id.to_string(),
        };
        let fact = ProviderFact {
            output_index,
            hosted_index: 0,
            tool: tool.to_string(),
            fact_ref: fact_ref.clone(),
            provider_status: status.map(str::to_string),
        };
        if !seen_refs.insert(fact_ref.clone()) {
            findings.push(finding(
                "provider_fact_duplicate",
                Some(output_index),
                None,
                Some(fact_ref.clone()),
                format!("Hosted output_index {output_index} duplicates a provider fact identity"),
            ));
        }
        facts.push(fact);
    }

    facts.sort_by_key(|fact| fact.output_index);
    for (hosted_index, fact) in facts.iter_mut().enumerate() {
        fact.hosted_index = hosted_index;
    }

    ProviderFactCollection { facts, findings }
}

pub(crate) fn reconcile_provider_bindings(
    collection: ProviderFactCollection,
    declarations: &[TaskspaceExecHostedBinding],
) -> ProviderReconciliationReport {
    let mut findings = collection.findings;
    if collection.facts.len() != declarations.len() {
        findings.push(finding(
            "provider_binding_count_mismatch",
            None,
            None,
            None,
            format!(
                "Provider response has {} hosted items but TaskSpace declares {} bindings",
                collection.facts.len(),
                declarations.len()
            ),
        ));
    }

    for (fact, declaration) in collection.facts.iter().zip(declarations) {
        if fact.tool != declaration.tool.trim() {
            findings.push(finding(
                "provider_binding_tool_mismatch",
                Some(fact.output_index),
                Some(fact.hosted_index),
                Some(fact.fact_ref.clone()),
                format!(
                    "Hosted item at index {} is `{}` but TaskSpace declares `{}`",
                    fact.hosted_index,
                    fact.tool,
                    declaration.tool.trim()
                ),
            ));
        }
        if declaration.node_ids.is_empty() {
            findings.push(finding(
                "provider_binding_nodes_missing",
                Some(fact.output_index),
                Some(fact.hosted_index),
                Some(fact.fact_ref.clone()),
                format!(
                    "Hosted item at index {} has no Agent-declared nodes",
                    fact.hosted_index
                ),
            ));
        }
        let mut node_ids = BTreeSet::new();
        for node_id in &declaration.node_ids {
            let node_id = node_id.trim();
            if node_id.is_empty() {
                findings.push(finding(
                    "provider_binding_node_missing",
                    Some(fact.output_index),
                    Some(fact.hosted_index),
                    Some(fact.fact_ref.clone()),
                    format!(
                        "Hosted item at index {} has an empty Agent-declared node",
                        fact.hosted_index
                    ),
                ));
            } else if !node_ids.insert(node_id) {
                findings.push(finding(
                    "provider_binding_node_duplicate",
                    Some(fact.output_index),
                    Some(fact.hosted_index),
                    Some(fact.fact_ref.clone()),
                    format!(
                        "Hosted item at index {} repeats Agent-declared node `{node_id}`",
                        fact.hosted_index
                    ),
                ));
            }
        }
    }

    let exact = findings.is_empty();
    let bindings = if exact {
        collection
            .facts
            .into_iter()
            .zip(declarations)
            .map(|(fact, declaration)| ProviderBinding {
                output_index: fact.output_index,
                hosted_index: fact.hosted_index,
                fact_ref: fact.fact_ref,
                node_ids: declaration
                    .node_ids
                    .iter()
                    .map(|node_id| node_id.trim().to_string())
                    .collect(),
            })
            .collect()
    } else {
        Vec::new()
    };

    ProviderReconciliationReport {
        exact,
        bindings,
        findings,
    }
}

fn finding(
    reason_code: &'static str,
    output_index: Option<usize>,
    hosted_index: Option<usize>,
    fact_ref: Option<ProviderFactRef>,
    message: impl Into<String>,
) -> ProviderReconciliationFinding {
    ProviderReconciliationFinding {
        reason_code,
        output_index,
        hosted_index,
        fact_ref,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn collect(items: &[ResponseItem]) -> ProviderFactCollection {
        collect_provider_facts(items.iter().enumerate())
    }

    fn binding(tool: &str, node_ids: &[&str]) -> TaskspaceExecHostedBinding {
        TaskspaceExecHostedBinding {
            tool: tool.to_string(),
            node_ids: node_ids
                .iter()
                .map(|node_id| (*node_id).to_string())
                .collect(),
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
            result: "fixture".to_string(),
        }
    }

    fn assert_rejected(report: &ProviderReconciliationReport, reason_code: &str) {
        assert!(!report.exact);
        assert!(report.bindings.is_empty());
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding.reason_code == reason_code)
        );
    }

    #[test]
    fn ordered_declarations_bind_same_response_items_to_different_nodes() {
        let facts = collect(&[
            web(Some("call-1"), "completed"),
            web(Some("call-2"), "failed"),
            image("image-1", "completed"),
        ]);
        let report = reconcile_provider_bindings(
            facts,
            &[
                binding(WEB_SEARCH_TOOL_NAME, &["research-a", "compare"]),
                binding(WEB_SEARCH_TOOL_NAME, &["research-b"]),
                binding(IMAGE_GENERATION_TOOL_NAME, &["design"]),
            ],
        );

        assert!(report.exact);
        assert_eq!(report.bindings.len(), 3);
        assert_eq!(
            report
                .bindings
                .iter()
                .map(|binding| (
                    binding.output_index,
                    binding.hosted_index,
                    binding.fact_ref.provider_item_id.as_str(),
                    binding
                        .node_ids
                        .iter()
                        .map(String::as_str)
                        .collect::<Vec<_>>(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (0, 0, "call-1", vec!["research-a", "compare"]),
                (1, 1, "call-2", vec!["research-b"]),
                (2, 2, "image-1", vec!["design"]),
            ]
        );
    }

    #[test]
    fn provider_output_index_restores_order_when_done_events_arrive_out_of_order() {
        let web_a = web(Some("call-z"), "completed");
        let web_b = web(Some("call-a"), "completed");
        let image = image("image-m", "completed");
        let facts = collect_provider_facts([(2, &image), (0, &web_a), (1, &web_b)]);

        assert_eq!(
            facts
                .facts
                .iter()
                .map(|fact| (
                    fact.output_index,
                    fact.hosted_index,
                    fact.fact_ref.provider_item_id.as_str(),
                ))
                .collect::<Vec<_>>(),
            vec![(0, 0, "call-z"), (1, 1, "call-a"), (2, 2, "image-m")]
        );
    }

    #[test]
    fn provider_status_does_not_change_identity_or_node_lifecycle() {
        let completed = collect(&[web(Some("call-1"), "completed")]);
        let failed = collect(&[web(Some("call-1"), "failed")]);

        assert_eq!(completed.facts[0].fact_ref, failed.facts[0].fact_ref);
        assert_ne!(
            completed.facts[0].provider_status,
            failed.facts[0].provider_status
        );
    }

    #[test]
    fn missing_or_extra_declarations_reject_the_whole_set() {
        let facts = collect(&[web(Some("call-1"), "completed"), image("image-1", "failed")]);
        let report =
            reconcile_provider_bindings(facts, &[binding(WEB_SEARCH_TOOL_NAME, &["research"])]);

        assert_rejected(&report, "provider_binding_count_mismatch");

        let report = reconcile_provider_bindings(
            collect(&[web(Some("call-1"), "completed")]),
            &[
                binding(WEB_SEARCH_TOOL_NAME, &["research"]),
                binding(IMAGE_GENERATION_TOOL_NAME, &["design"]),
            ],
        );
        assert_rejected(&report, "provider_binding_count_mismatch");
    }

    #[test]
    fn duplicate_and_missing_provider_ids_reject_without_partial_bindings() {
        let facts = collect(&[
            web(Some("call-1"), "completed"),
            web(Some("call-1"), "failed"),
            web(None, "completed"),
        ]);
        let report = reconcile_provider_bindings(
            facts,
            &[
                binding(WEB_SEARCH_TOOL_NAME, &["research-a"]),
                binding(WEB_SEARCH_TOOL_NAME, &["research-b"]),
                binding(WEB_SEARCH_TOOL_NAME, &["research-c"]),
            ],
        );

        assert_rejected(&report, "provider_fact_duplicate");
        assert_rejected(&report, "provider_item_identity_missing");
    }

    #[test]
    fn declaration_without_provider_fact_is_rejected() {
        let report = reconcile_provider_bindings(
            collect(&[]),
            &[binding(WEB_SEARCH_TOOL_NAME, &["research"])],
        );

        assert_rejected(&report, "provider_binding_count_mismatch");
    }

    #[test]
    fn tool_order_mismatch_rejects_without_guessing() {
        let facts = collect(&[
            web(Some("call-1"), "completed"),
            image("image-1", "completed"),
        ]);
        let report = reconcile_provider_bindings(
            facts,
            &[
                binding(IMAGE_GENERATION_TOOL_NAME, &["design"]),
                binding(WEB_SEARCH_TOOL_NAME, &["research"]),
            ],
        );

        assert!(!report.exact);
        assert!(report.bindings.is_empty());
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.reason_code == "provider_binding_tool_mismatch")
                .count(),
            2
        );
    }

    #[test]
    fn no_provider_facts_and_no_declarations_is_exact() {
        let report = reconcile_provider_bindings(collect(&[]), &[]);

        assert!(report.exact);
        assert!(report.bindings.is_empty());
        assert!(report.findings.is_empty());
    }

    #[test]
    fn duplicate_output_index_rejects_without_partial_bindings() {
        let web = web(Some("call-1"), "completed");
        let image = image("image-1", "completed");
        let report = reconcile_provider_bindings(
            collect_provider_facts([(0, &web), (0, &image)]),
            &[
                binding(WEB_SEARCH_TOOL_NAME, &["research"]),
                binding(IMAGE_GENERATION_TOOL_NAME, &["design"]),
            ],
        );

        assert_rejected(&report, "provider_output_index_duplicate");
    }

    #[test]
    fn empty_or_duplicate_node_sets_reject_without_partial_bindings() {
        let cases: [(&[&str], &str); 2] = [
            (&[], "provider_binding_nodes_missing"),
            (&["research", "research"], "provider_binding_node_duplicate"),
        ];
        for (node_ids, reason_code) in cases {
            let report = reconcile_provider_bindings(
                collect(&[web(Some("call-1"), "completed")]),
                &[binding(WEB_SEARCH_TOOL_NAME, node_ids)],
            );
            assert_rejected(&report, reason_code);
        }
    }
}
