use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_protocol::models::ResponseItem;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

use super::TASKSPACE_EXEC_TOOL_NAME;
use super::TaskSpaceExecCatalog;
use super::self_heal_apply_patch::normalized_apply_patch_candidates;

const ERROR_WINDOW_BYTES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceExecSelfHeal {
    pub(crate) call_id: String,
    pub(crate) operation: &'static str,
    pub(crate) repair_token: &'static str,
    pub(crate) byte_index: usize,
    pub(crate) original_arguments_sha256: String,
    pub(crate) repaired_arguments_sha256: String,
}

pub(crate) fn self_heal_taskspace_exec_response_item(
    item: &mut ResponseItem,
    catalog: &TaskSpaceExecCatalog,
) -> Option<TaskSpaceExecSelfHeal> {
    let ResponseItem::FunctionCall {
        name,
        namespace,
        arguments,
        call_id,
        ..
    } = item
    else {
        return None;
    };
    if name != TASKSPACE_EXEC_TOOL_NAME || namespace.is_some() {
        return None;
    }

    let repaired = repair_syntax_error(arguments, catalog)?;
    let original_arguments_sha256 = sha256(arguments);
    let repaired_arguments_sha256 = sha256(&repaired.arguments);
    *arguments = repaired.arguments;
    Some(TaskSpaceExecSelfHeal {
        call_id: call_id.clone(),
        operation: repaired.operation,
        repair_token: repaired.repair_token,
        byte_index: repaired.byte_index,
        original_arguments_sha256,
        repaired_arguments_sha256,
    })
}

struct RepairedArguments {
    arguments: String,
    operation: &'static str,
    repair_token: &'static str,
    byte_index: usize,
}

fn repair_syntax_error(
    arguments: &str,
    catalog: &TaskSpaceExecCatalog,
) -> Option<RepairedArguments> {
    let error = serde_json::from_str::<Value>(arguments).err()?;

    if let Some((candidate, byte_index)) = escape_raw_newlines_in_json_strings(arguments) {
        if catalog.decode_plan(&candidate).is_ok() {
            return Some(RepairedArguments {
                arguments: candidate,
                operation: "escape",
                repair_token: "\\n",
                byte_index,
            });
        }
        if raw_newline_is_inside_complete_apply_patch(arguments, byte_index)
            && let Some(candidate) = delete_one_extra_brace_from_complete_patch(&candidate, catalog)
        {
            return Some(RepairedArguments {
                arguments: candidate,
                operation: "normalize",
                repair_token: "raw_patch_newline_plus_extra_brace",
                byte_index,
            });
        }
    }

    if let Some((candidate, byte_index)) = encode_raw_apply_patch_input(arguments)
        && catalog.decode_plan(&candidate).is_ok()
    {
        return Some(RepairedArguments {
            arguments: candidate,
            operation: "encode",
            repair_token: "apply_patch_input",
            byte_index,
        });
    }

    let positions = candidate_positions(arguments, error.line(), error.column());
    let mut repairs = BTreeMap::new();

    for &byte_index in &positions {
        for (delimiter, repair_token) in [('}', "}"), (']', "]")] {
            let mut candidate = String::with_capacity(arguments.len() + 1);
            candidate.push_str(&arguments[..byte_index]);
            candidate.push(delimiter);
            candidate.push_str(&arguments[byte_index..]);
            if catalog.decode_plan(&candidate).is_ok() {
                repairs
                    .entry(candidate)
                    .or_insert(("insert", repair_token, byte_index));
            }
        }

        let Some(delimiter @ ('}' | ']')) = arguments[byte_index..].chars().next() else {
            continue;
        };
        let repair_token = match delimiter {
            '}' => "}",
            ']' => "]",
            _ => unreachable!(),
        };
        let mut candidate = String::with_capacity(arguments.len() - delimiter.len_utf8());
        candidate.push_str(&arguments[..byte_index]);
        candidate.push_str(&arguments[byte_index + delimiter.len_utf8()..]);
        if catalog.decode_plan(&candidate).is_ok() {
            repairs
                .entry(candidate)
                .or_insert(("delete", repair_token, byte_index));
        }
    }

    for (candidate, byte_index) in normalized_apply_patch_candidates(arguments, &positions) {
        if catalog.decode_plan(&candidate).is_ok() {
            repairs
                .entry(candidate)
                .or_insert(("normalize", "apply_patch_wrapper", byte_index));
        }
    }

    if repairs.len() != 1 {
        return None;
    }
    repairs
        .into_iter()
        .next()
        .map(
            |(arguments, (operation, repair_token, byte_index))| RepairedArguments {
                arguments,
                operation,
                repair_token,
                byte_index,
            },
        )
}

fn raw_newline_is_inside_complete_apply_patch(arguments: &str, byte_index: usize) -> bool {
    const BEGIN: &str = "*** Begin Patch";
    const END: &str = "*** End Patch";

    let Some(patch_start) = arguments.find(BEGIN) else {
        return false;
    };
    if arguments[patch_start + BEGIN.len()..].contains(BEGIN) {
        return false;
    }
    let Some(end_offset) = arguments[patch_start..].find(END) else {
        return false;
    };
    let patch_end = patch_start + end_offset + END.len();
    if arguments[patch_end..].contains(END)
        || !(patch_start..patch_end).contains(&byte_index)
        || arguments.as_bytes().get(byte_index) != Some(&b'\n')
    {
        return false;
    }

    let Some(opening_quote) = patch_start.checked_sub(1) else {
        return false;
    };
    if arguments.as_bytes().get(opening_quote) != Some(&b'"') {
        return false;
    }
    let input_prefix = arguments[..opening_quote].trim_end();
    let Some(input_prefix) = input_prefix.strip_suffix(':').map(str::trim_end) else {
        return false;
    };
    input_prefix.ends_with("\"input\"")
}

fn encode_raw_apply_patch_input(arguments: &str) -> Option<(String, usize)> {
    const BEGIN: &str = "*** Begin Patch";
    const END: &str = "*** End Patch";

    let patch_start = arguments.find(BEGIN)?;
    if arguments[patch_start + BEGIN.len()..].contains(BEGIN) {
        return None;
    }
    let opening_quote = patch_start.checked_sub(1)?;
    if arguments.as_bytes().get(opening_quote) != Some(&b'"') {
        return None;
    }

    let input_prefix = arguments[..opening_quote].trim_end();
    let input_prefix = input_prefix.strip_suffix(':')?.trim_end();
    if !input_prefix.ends_with("\"input\"") {
        return None;
    }
    let tool_field = arguments[..opening_quote].rfind("\"tool\"")?;
    if !arguments[tool_field..opening_quote].contains("\"apply_patch\"") {
        return None;
    }

    let end_start = arguments[patch_start..].rfind(END)? + patch_start;
    let mut closing_quote = end_start + END.len();
    while matches!(arguments.as_bytes().get(closing_quote), Some(b'\r' | b'\n')) {
        closing_quote += 1;
    }
    if arguments.as_bytes().get(closing_quote) != Some(&b'"')
        || !arguments[patch_start..closing_quote].contains('\n')
    {
        return None;
    }
    let suffix = arguments[closing_quote + 1..].trim_start();
    if !suffix.starts_with('}') {
        return None;
    }

    let encoded_patch = serde_json::to_string(&arguments[patch_start..closing_quote]).ok()?;
    let mut candidate = String::with_capacity(arguments.len() + encoded_patch.len());
    candidate.push_str(&arguments[..opening_quote]);
    candidate.push_str(&encoded_patch);
    candidate.push_str(&arguments[closing_quote + 1..]);
    Some((candidate, opening_quote))
}

fn escape_raw_newlines_in_json_strings(arguments: &str) -> Option<(String, usize)> {
    let mut repaired = String::with_capacity(arguments.len());
    let mut inside_string = false;
    let mut escaped = false;
    let mut first_newline = None;

    for (byte_index, character) in arguments.char_indices() {
        if escaped {
            repaired.push(character);
            escaped = false;
            continue;
        }
        match character {
            '\\' if inside_string => {
                repaired.push(character);
                escaped = true;
            }
            '"' => {
                repaired.push(character);
                inside_string = !inside_string;
            }
            '\n' if inside_string => {
                repaired.push_str("\\n");
                first_newline.get_or_insert(byte_index);
            }
            _ => repaired.push(character),
        }
    }

    first_newline.map(|byte_index| (repaired, byte_index))
}

fn delete_one_extra_brace_from_complete_patch(
    escaped_arguments: &str,
    catalog: &TaskSpaceExecCatalog,
) -> Option<String> {
    let error = serde_json::from_str::<Value>(escaped_arguments).err()?;
    let positions = candidate_positions(escaped_arguments, error.line(), error.column());
    let mut repairs = BTreeSet::new();

    for byte_index in positions {
        if escaped_arguments.as_bytes().get(byte_index) != Some(&b'}') {
            continue;
        }
        let mut candidate = String::with_capacity(escaped_arguments.len() - 1);
        candidate.push_str(&escaped_arguments[..byte_index]);
        candidate.push_str(&escaped_arguments[byte_index + 1..]);
        if catalog.decode_plan(&candidate).is_ok() && has_one_complete_apply_patch(&candidate) {
            repairs.insert(candidate);
        }
    }

    if repairs.len() != 1 {
        return None;
    }
    repairs.into_iter().next()
}

fn has_one_complete_apply_patch(arguments: &str) -> bool {
    const BEGIN: &str = "*** Begin Patch";
    const END: &str = "*** End Patch";

    let Ok(plan) = serde_json::from_str::<Value>(arguments) else {
        return false;
    };
    let Some(actions) = plan.get("tools").and_then(Value::as_array) else {
        return false;
    };
    let patches = actions
        .iter()
        .filter(|action| action.get("tool").and_then(Value::as_str) == Some("apply_patch"))
        .filter_map(|action| action.get("input").and_then(Value::as_str))
        .collect::<Vec<_>>();
    patches.len() == 1
        && patches[0].trim().starts_with(BEGIN)
        && patches[0].trim().ends_with(END)
        && patches[0].matches(BEGIN).count() == 1
        && patches[0].matches(END).count() == 1
}

fn candidate_positions(arguments: &str, line: usize, column: usize) -> BTreeSet<usize> {
    let error_offset = json_error_byte_offset(arguments, line, column);
    let start = error_offset.saturating_sub(ERROR_WINDOW_BYTES);
    let end = arguments
        .len()
        .min(error_offset.saturating_add(ERROR_WINDOW_BYTES));
    let mut positions = arguments
        .char_indices()
        .map(|(index, _)| index)
        .filter(|index| *index >= start && *index <= end)
        .collect::<BTreeSet<_>>();
    positions.insert(arguments.trim_end().len());
    positions
}

fn json_error_byte_offset(arguments: &str, line: usize, column: usize) -> usize {
    let line_start = arguments
        .match_indices('\n')
        .take(line.saturating_sub(1))
        .map(|(index, _)| index + 1)
        .last()
        .unwrap_or(0);
    let line_end = arguments[line_start..]
        .find('\n')
        .map(|offset| line_start + offset)
        .unwrap_or(arguments.len());
    line_start
        .saturating_add(column.saturating_sub(1))
        .min(line_end)
}

fn sha256(value: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(value.as_bytes());
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use codex_tools::AdditionalProperties;
    use codex_tools::FreeformTool;
    use codex_tools::FreeformToolFormat;
    use codex_tools::JsonSchema;
    use codex_tools::ResponsesApiTool;
    use codex_tools::ToolSpec;

    use super::*;

    fn catalog() -> TaskSpaceExecCatalog {
        TaskSpaceExecCatalog::build(&[
            ToolSpec::Function(ResponsesApiTool {
                name: "exec_command".to_string(),
                description: "execute".to_string(),
                strict: false,
                parameters: JsonSchema::object(
                    BTreeMap::new(),
                    None,
                    Some(AdditionalProperties::Boolean(true)),
                ),
                output_schema: None,
                defer_loading: None,
            }),
            ToolSpec::Freeform(FreeformTool {
                name: "apply_patch".to_string(),
                description: "apply one patch".to_string(),
                format: FreeformToolFormat {
                    r#type: "grammar".to_string(),
                    syntax: "lark".to_string(),
                    definition: "start: /.+/".to_string(),
                },
            }),
        ])
        .expect("catalog")
    }

    fn call(arguments: String) -> ResponseItem {
        ResponseItem::FunctionCall {
            id: None,
            name: TASKSPACE_EXEC_TOOL_NAME.to_string(),
            namespace: None,
            arguments,
            call_id: "call-1".to_string(),
        }
    }

    fn valid_arguments() -> String {
        r#"{"type":"work","tools":[{"tool":"exec_command","node_id":"work","input":{}}]}"#
            .to_string()
    }

    #[test]
    fn repairs_one_missing_outer_brace() {
        let valid = valid_arguments();
        let malformed = valid.strip_suffix('}').expect("outer brace").to_string();
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "insert");
        assert_eq!(repair.repair_token, "}");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn repairs_missing_tool_action_brace_before_next_action() {
        let valid = serde_json::json!({
            "type": "work",
            "tools": [
                {"tool": "exec_command", "node_id": "work", "input": {}},
                {"tool": "exec_command", "node_id": "work", "input": {}}
            ]
        })
        .to_string();
        let boundary = valid.rfind("},{").expect("action boundary");
        let mut malformed = valid.clone();
        malformed.remove(boundary);
        let mut item = call(malformed);

        self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn repairs_missing_node_patch_brace_after_non_ascii_content() {
        let valid = r#"{"type":"update_and_work","update_map":{"add_work_nodes":[],"node_patches":[{"node_id":"work","state":"completed","content":"已定位到实现中的舍入精度问题，需要修改并验证。"}]},"tools":[{"tool":"exec_command","node_id":"work","input":{}}]}"#.to_string();
        let boundary = valid.find("}]},\"tools\"").expect("node patch boundary");
        assert_eq!(valid.as_bytes()[boundary], b'}');
        let mut malformed = valid.clone();
        malformed.remove(boundary);
        let mut item = call(malformed);

        self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn repairs_one_missing_tools_array_bracket() {
        let valid = valid_arguments();
        let boundary = valid.rfind("]}").expect("tools array boundary");
        let mut malformed = valid.clone();
        malformed.remove(boundary);
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "insert");
        assert_eq!(repair.repair_token, "]");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn leaves_valid_or_non_taskspace_calls_unchanged() {
        let valid = valid_arguments();
        let mut taskspace = call(valid.clone());
        assert!(self_heal_taskspace_exec_response_item(&mut taskspace, &catalog()).is_none());

        let mut ordinary = call(valid);
        let ResponseItem::FunctionCall { name, .. } = &mut ordinary else {
            panic!("function call")
        };
        *name = "ordinary".to_string();
        let before = ordinary.clone();
        assert!(self_heal_taskspace_exec_response_item(&mut ordinary, &catalog()).is_none());
        assert_eq!(ordinary, before);
    }

    #[test]
    fn refuses_repairs_that_do_not_decode_as_exec_plan() {
        let original = "{\"not_tools\":[]".to_string();
        let mut item = call(original.clone());

        assert!(self_heal_taskspace_exec_response_item(&mut item, &catalog()).is_none());

        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, original);
    }

    #[test]
    fn refuses_multiple_missing_closing_delimiters() {
        let valid = valid_arguments();
        let malformed = valid
            .strip_suffix("]}")
            .expect("array and object suffix")
            .to_string();
        let mut item = call(malformed.clone());

        assert!(self_heal_taskspace_exec_response_item(&mut item, &catalog()).is_none());
        assert_eq!(item, call(malformed));
    }

    #[test]
    fn escapes_one_raw_newline_inside_a_tool_string() {
        let valid = r#"{"type":"work","tools":[{"tool":"apply_patch","node_id":"fix","input":"*** Begin Patch\n*** End Patch"}]}"#.to_string();
        let malformed = valid.replacen("\\n", "\n", 1);
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "escape");
        assert_eq!(repair.repair_token, "\\n");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn escapes_multiple_raw_newlines_inside_tool_strings() {
        let valid = r#"{"type":"work","tools":[{"tool":"apply_patch","node_id":"fix","input":"line one\nline two\nline three"}]}"#;
        let malformed = valid.replace("\\n", "\n");
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "escape");
        assert_eq!(repair.repair_token, "\\n");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn repairs_raw_patch_newline_combined_with_one_extra_action_brace() {
        let patch = concat!(
            "*** Begin Patch\n",
            "*** Update File: /workspace/inventory.py\n",
            "@@\n",
            "-old_inventory\n",
            "+new_inventory\n",
            "*** Update File: /workspace/shipping.py\n",
            "@@\n",
            "-old_shipping\n",
            "+new_shipping\n",
            "*** End Patch\n",
        );
        let valid = serde_json::json!({
            "type": "update_and_work",
            "update_map": {
                "add_work_nodes": [],
                "node_patches": [{
                    "node_id": "inspect",
                    "state": "completed",
                    "content": "inspection complete"
                }]
            },
            "tools": [{
                "tool": "apply_patch",
                "node_id": "fix",
                "input": patch
            }]
        })
        .to_string();
        let mut malformed = valid.replacen(
            "shipping.py\\n@@\\n-old_shipping",
            "shipping.py\\n@@\n-old_shipping",
            1,
        );
        let action_end = malformed.rfind("]}").expect("tools suffix");
        malformed.insert(action_end, '}');
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "normalize");
        assert_eq!(repair.repair_token, "raw_patch_newline_plus_extra_brace");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn refuses_raw_patch_newline_combined_with_two_extra_action_braces() {
        let valid = r#"{"type":"work","tools":[{"tool":"apply_patch","node_id":"fix","input":"*** Begin Patch\n*** End Patch"}]}"#;
        let mut malformed = valid.replacen("\\n", "\n", 1);
        let action_end = malformed.rfind("]}").expect("tools suffix");
        malformed.insert_str(action_end, "}}");
        let mut item = call(malformed.clone());

        assert!(self_heal_taskspace_exec_response_item(&mut item, &catalog()).is_none());
        assert_eq!(item, call(malformed));
    }

    #[test]
    fn encodes_observed_raw_multiline_apply_patch_input() {
        let patch = "*** Begin Patch\n*** Update File: /workspace/example.py\n@@\n-value = \"old\"\n+value = \"new\"\n*** End Patch\n";
        let encoded_patch = serde_json::to_string(patch).expect("patch JSON");
        let valid = format!(
            r#"{{"type":"work","tools":[{{"tool":"apply_patch","node_id":"fix","input":{encoded_patch}}}]}}"#
        );
        let malformed = format!(
            "{{\"type\":\"work\",\"tools\":[{{\"tool\":\"apply_patch\",\"node_id\":\"fix\",\"input\":\"{patch}\"}}]}}"
        );
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "encode");
        assert_eq!(repair.repair_token, "apply_patch_input");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn refuses_raw_newlines_combined_with_another_syntax_error() {
        let valid = r#"{"type":"work","tools":[{"tool":"apply_patch","node_id":"fix","input":"line one\nline two"}]}"#;
        let malformed = valid
            .replace("\\n", "\n")
            .strip_suffix('}')
            .expect("outer brace")
            .to_string();
        let mut item = call(malformed.clone());

        assert!(self_heal_taskspace_exec_response_item(&mut item, &catalog()).is_none());
        assert_eq!(item, call(malformed));
    }

    #[test]
    fn removes_the_observed_extra_tool_action_brace() {
        let valid = r#"{"type": "work", "tools": [{"tool": "apply_patch", "node_id": "fix", "input": "*** Begin Patch\n*** Update File: /workspace/src/tax_calc.py\n@@\n-    return round(subtotal * RATES[region], 1)\n+    return round(subtotal * RATES[region], 2)\n*** End Patch"}]}"#.to_string();
        assert_eq!(
            sha256(&valid),
            "e9fa45fa32abdcf2d0da8b8c9b96c03077226e1f62c7fc056cec406dc3684db8"
        );
        let boundary = valid.rfind("}]}").expect("tool action boundary") + 1;
        let mut malformed = valid.clone();
        malformed.insert(boundary, '}');
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "delete");
        assert_eq!(repair.repair_token, "}");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn removes_one_extra_tools_array_bracket() {
        let valid = valid_arguments();
        let boundary = valid.rfind("]}").expect("tools array boundary") + 1;
        let mut malformed = valid.clone();
        malformed.insert(boundary, ']');
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.operation, "delete");
        assert_eq!(repair.repair_token, "]");
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }
}
