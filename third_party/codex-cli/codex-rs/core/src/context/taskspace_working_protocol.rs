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
pub(crate) const TASKSPACE_WORKING_PROTOCOL_VERSION: &str = "1.0.3";
pub(crate) const TASKSPACE_WORKING_PROTOCOL_RULES_SHA256: &str =
    "6d4dd3049120ac78dec8b4bfb6098c8c12416748606aa8be130551343a15aa97";

const TASKSPACE_WORKING_PROTOCOL_RULES: &str = concat!(
    "1. Use the TaskSpace Map as the mandatory ledger for task topology and lifecycle; natural conversation remains the detailed evidence and work history.\n",
    "2. If bootstrap_required=true, the first top-level tool call must be initialize_map. Declare continuation=next_tool or next_apply_patch and emit the required ordinary top-level tool immediately after taskspace_control in the same response.\n",
    "3. Update the Map at meaningful task-phase boundaries, not after every ordinary tool result. Keep ordinary work under the bound Work node.\n",
    "4. A running Work node cannot be completed alone. When completion makes another node Ready, use complete_then_continue with the current node, your selected next node, and continuation. Emit the selected next top-level tool immediately after control in the same response; use next_apply_patch only when that call is direct apply_patch.\n",
    "5. At the final Work boundary, use complete_then_end with the current node and your exact final summary. Use finish_end only when Finish is already Ready and no running Work node needs completion.\n",
    "6. Use read_map only when the current revision, binding, or Ready frontier is not established by the latest visible Map or control result, including after rejection or context recovery. Do not read on a fixed cadence.\n",
    "7. You choose task decomposition, completion, Ready nodes, and actions. Runtime only validates hard graph and lifecycle rules and never infers or rewrites those choices.\n",
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
        assert!(rendered.contains("- protocol_version: 1.0.3\n"));
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
