use codex_protocol::models::ContentItem;
use codex_protocol::models::ResponseItem;
#[cfg(test)]
use sha2::Digest;
#[cfg(test)]
use sha2::Sha256;

pub(crate) const TASKSPACE_WORKING_PROTOCOL_START: &str = "TaskSpaceCoreWorkingProtocolV1:";
pub(crate) const TASKSPACE_WORKING_PROTOCOL_END: &str = "TaskSpaceCoreWorkingProtocolV1 end.";
pub(crate) const TASKSPACE_WORKING_PROTOCOL_SCHEMA_VERSION: &str =
    "taskspace-core-working-protocol-v1";
pub(crate) const TASKSPACE_WORKING_PROTOCOL_VERSION: &str = "1.0.0";
pub(crate) const TASKSPACE_WORKING_PROTOCOL_RULES_SHA256: &str =
    "d79723097841f2555c981663fb28bdca9099bbf7fd32246d81c609e21bd35efa";

const TASKSPACE_WORKING_PROTOCOL_RULES: &str = concat!(
    "1. Treat the TaskSpace Map as the mandatory authoritative ledger for task topology and lifecycle; natural conversation remains the detailed evidence and work history.\n",
    "2. When bootstrap_required=true, your first top-level tool call must initialize_map. Put immediate ordinary work in initialize_map.continuation.\n",
    "3. Keep the Map aligned at meaningful work boundaries, not after every ordinary tool call. All ordinary work must remain under a bound Work node.\n",
    "4. You decide when a node goal is fulfilled. Complete it, select a Ready successor yourself, and bind that successor with immediate continuation actions. Mutate the graph when your task decomposition or dependencies change.\n",
    "5. Use read_map when the current revision, binding, or Ready frontier is not established by the latest visible Map or control result, including after a state rejection or context recovery. Do not read on a fixed cadence.\n",
    "6. Before a final answer, ensure all Work nodes are closed and commit the exact final summary through finish_end. Do not emit a plain final answer while the Map is open.\n",
    "7. Runtime validates only hard graph and lifecycle rules. It will not infer task meaning, decide completion, choose a node, or rewrite your actions.\n",
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct TaskSpaceWorkingProtocolIdentity {
    pub(crate) schema_version: &'static str,
    pub(crate) protocol_version: &'static str,
    pub(crate) rules_sha256: &'static str,
    pub(crate) rendered_bytes: usize,
}

pub(crate) fn taskspace_working_protocol_identity() -> TaskSpaceWorkingProtocolIdentity {
    TaskSpaceWorkingProtocolIdentity {
        schema_version: TASKSPACE_WORKING_PROTOCOL_SCHEMA_VERSION,
        protocol_version: TASKSPACE_WORKING_PROTOCOL_VERSION,
        rules_sha256: TASKSPACE_WORKING_PROTOCOL_RULES_SHA256,
        rendered_bytes: render_taskspace_working_protocol().len(),
    }
}

pub(crate) fn render_taskspace_working_protocol() -> String {
    format!(
        "{TASKSPACE_WORKING_PROTOCOL_START}\n- schema_version: {TASKSPACE_WORKING_PROTOCOL_SCHEMA_VERSION}\n- protocol_version: {TASKSPACE_WORKING_PROTOCOL_VERSION}\n- rules_sha256: {TASKSPACE_WORKING_PROTOCOL_RULES_SHA256}\n- scope: all_taskspace_projection_policies\n- delivery: stable_developer_prefix\nRules:\n{TASKSPACE_WORKING_PROTOCOL_RULES}{TASKSPACE_WORKING_PROTOCOL_END}\n"
    )
}

pub(crate) fn prepend_taskspace_working_protocol(input: &mut Vec<ResponseItem>) -> usize {
    let original_len = input.len();
    input.retain(|item| !is_taskspace_working_protocol_message(item));
    let removed_duplicates = original_len.saturating_sub(input.len());
    input.insert(0, taskspace_working_protocol_message());
    removed_duplicates
}

pub(crate) fn is_taskspace_working_protocol_message(item: &ResponseItem) -> bool {
    let ResponseItem::Message { role, content, .. } = item else {
        return false;
    };
    role == "developer"
        && content.iter().any(|item| {
            matches!(item, ContentItem::InputText { text } if text.starts_with(TASKSPACE_WORKING_PROTOCOL_START))
        })
}

fn taskspace_working_protocol_message() -> ResponseItem {
    ResponseItem::Message {
        id: None,
        role: "developer".to_string(),
        content: vec![ContentItem::InputText {
            text: render_taskspace_working_protocol(),
        }],
        end_turn: None,
        phase: None,
    }
}

#[cfg(test)]
fn rules_sha256() -> String {
    let mut hasher = Sha256::new();
    hasher.update(TASKSPACE_WORKING_PROTOCOL_RULES.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(role: &str, text: &str) -> ResponseItem {
        ResponseItem::Message {
            id: None,
            role: role.to_string(),
            content: vec![ContentItem::InputText {
                text: text.to_string(),
            }],
            end_turn: None,
            phase: None,
        }
    }

    #[test]
    fn versioned_protocol_hash_matches_rules() {
        assert_eq!(rules_sha256(), TASKSPACE_WORKING_PROTOCOL_RULES_SHA256);
        let rendered = render_taskspace_working_protocol();
        assert!(rendered.starts_with(TASKSPACE_WORKING_PROTOCOL_START));
        assert!(rendered.ends_with(&format!("{TASKSPACE_WORKING_PROTOCOL_END}\n")));
        assert!(rendered.contains("- protocol_version: 1.0.0\n"));
        assert!(rendered.contains(TASKSPACE_WORKING_PROTOCOL_RULES_SHA256));
    }

    #[test]
    fn provider_prefix_is_exactly_once_and_preserves_user_quotes() {
        let quoted = message(
            "user",
            &format!("quoted: {TASKSPACE_WORKING_PROTOCOL_START}"),
        );
        let stale = taskspace_working_protocol_message();
        let mut input = vec![quoted.clone(), stale.clone(), stale];

        assert_eq!(prepend_taskspace_working_protocol(&mut input), 2);
        assert_eq!(input.len(), 2);
        assert!(is_taskspace_working_protocol_message(&input[0]));
        assert_eq!(input[1], quoted);
    }
}
