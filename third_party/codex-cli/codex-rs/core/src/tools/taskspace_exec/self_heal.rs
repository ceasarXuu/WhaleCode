use std::collections::BTreeMap;
use std::collections::BTreeSet;

use codex_protocol::models::ResponseItem;
use serde_json::Value;
use sha2::Digest;
use sha2::Sha256;

use super::TASKSPACE_EXEC_TOOL_NAME;
use super::TaskSpaceExecCatalog;

const ERROR_WINDOW_BYTES: usize = 24;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TaskSpaceExecSelfHeal {
    pub(crate) call_id: String,
    pub(crate) inserted_delimiter: char,
    pub(crate) insertion_byte_index: usize,
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

    let repaired = repair_single_missing_closing_delimiter(arguments, catalog)?;
    let original_arguments_sha256 = sha256(arguments);
    let repaired_arguments_sha256 = sha256(&repaired.arguments);
    *arguments = repaired.arguments;
    Some(TaskSpaceExecSelfHeal {
        call_id: call_id.clone(),
        inserted_delimiter: repaired.delimiter,
        insertion_byte_index: repaired.byte_index,
        original_arguments_sha256,
        repaired_arguments_sha256,
    })
}

struct RepairedArguments {
    arguments: String,
    delimiter: char,
    byte_index: usize,
}

fn repair_single_missing_closing_delimiter(
    arguments: &str,
    catalog: &TaskSpaceExecCatalog,
) -> Option<RepairedArguments> {
    let error = serde_json::from_str::<Value>(arguments).err()?;
    let positions = candidate_positions(arguments, error.line(), error.column());
    let mut repairs = BTreeMap::new();

    for byte_index in positions {
        if is_inside_json_string(arguments, byte_index) {
            continue;
        }
        for delimiter in ['}', ']'] {
            let mut candidate = String::with_capacity(arguments.len() + 1);
            candidate.push_str(&arguments[..byte_index]);
            candidate.push(delimiter);
            candidate.push_str(&arguments[byte_index..]);
            if catalog.decode_plan(&candidate).is_ok() {
                repairs.entry(candidate).or_insert((delimiter, byte_index));
            }
        }
    }

    if repairs.len() != 1 {
        return None;
    }
    repairs
        .into_iter()
        .next()
        .map(|(arguments, (delimiter, byte_index))| RepairedArguments {
            arguments,
            delimiter,
            byte_index,
        })
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
    arguments[line_start..line_end]
        .char_indices()
        .nth(column.saturating_sub(1))
        .map(|(offset, _)| line_start + offset)
        .unwrap_or(line_end)
}

fn is_inside_json_string(arguments: &str, byte_index: usize) -> bool {
    let mut inside = false;
    let mut escaped = false;
    for byte in arguments.as_bytes().iter().take(byte_index) {
        if escaped {
            escaped = false;
            continue;
        }
        match *byte {
            b'\\' if inside => escaped = true,
            b'"' => inside = !inside,
            _ => {}
        }
    }
    inside
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
    use codex_tools::JsonSchema;
    use codex_tools::ResponsesApiTool;
    use codex_tools::ToolSpec;

    use super::*;

    fn catalog() -> TaskSpaceExecCatalog {
        TaskSpaceExecCatalog::build(&[ToolSpec::Function(ResponsesApiTool {
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
        })])
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
        serde_json::json!({
            "calls": [{
                "map": {
                    "operation": "read_map",
                    "input": {}
                }
            }]
        })
        .to_string()
    }

    #[test]
    fn repairs_one_missing_outer_brace() {
        let valid = valid_arguments();
        let malformed = valid.strip_suffix('}').expect("outer brace").to_string();
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.inserted_delimiter, '}');
        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn repairs_missing_call_envelope_brace_before_next_call() {
        let valid = serde_json::json!({
            "calls": [
                {"map": {"operation": "read_map", "input": {}}},
                {"client": {"name": "exec_command", "node_id": "work", "input": {}}}
            ]
        })
        .to_string();
        let boundary = valid.find(",{\"client\"").expect("client boundary");
        assert_eq!(valid.as_bytes()[boundary - 1], b'}');
        let mut malformed = valid.clone();
        malformed.remove(boundary - 1);
        let mut item = call(malformed);

        self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        let ResponseItem::FunctionCall { arguments, .. } = item else {
            panic!("function call")
        };
        assert_eq!(arguments, valid);
    }

    #[test]
    fn repairs_one_missing_calls_array_bracket() {
        let valid = valid_arguments();
        let boundary = valid.rfind("]}").expect("calls array boundary");
        let mut malformed = valid.clone();
        malformed.remove(boundary);
        let mut item = call(malformed);

        let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog()).expect("repair");

        assert_eq!(repair.inserted_delimiter, ']');
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
        let original = "{\"not_calls\":[]".to_string();
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
}
