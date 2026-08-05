use std::collections::BTreeSet;

use codex_code_mode::CodeModeToolKind;

use super::TASKSPACE_EXEC_PLAN_VERSION;
use super::TASKSPACE_EXEC_TOOL_NAME;
use super::catalog::TaskspaceExecCatalog;
use super::plan::TaskspaceExecCall;
use super::plan::TaskspaceExecPlan;
use super::preflight_support::TaskspaceExecPreflightError;
use super::preflight_support::call_error;
use super::preflight_support::map_action;
use super::preflight_support::plan_error;
use super::preflight_support::validate_hosted_bindings;
use super::preflight_support::validate_map_call;
use super::preflight_support::validate_plan_identity;
use super::preflight_support::validate_work_binding;

const TASKSPACE_CONTROL_TOOL_NAME: &str = "taskspace_control";
const APPLY_PATCH_TOOL_NAME: &str = "apply_patch";

#[derive(Debug, PartialEq)]
pub(crate) struct ValidatedTaskspaceExecPlan(TaskspaceExecPlan);

impl ValidatedTaskspaceExecPlan {
    pub(crate) fn as_plan(&self) -> &TaskspaceExecPlan {
        &self.0
    }
}

pub(crate) fn preflight_taskspace_exec_plan(
    plan: TaskspaceExecPlan,
    catalog: &TaskspaceExecCatalog,
) -> Result<ValidatedTaskspaceExecPlan, TaskspaceExecPreflightError> {
    validate_plan_identity(&plan, catalog)?;
    if plan.calls.is_empty() && plan.hosted_bindings.is_empty() {
        return Err(plan_error(
            "plan_empty",
            "TaskSpace Exec plan has no client calls or hosted binding declarations",
        ));
    }
    validate_hosted_bindings(&plan.hosted_bindings)?;

    let mut item_ids = BTreeSet::new();
    let mut patch_count = 0;
    let mut terminal_map_index = None;
    let mut initialization_index = None;
    for (index, call) in plan.calls.iter().enumerate() {
        validate_call_identity(call, index, &mut item_ids)?;
        validate_call_contract(call, index, catalog)?;
        if call.tool == APPLY_PATCH_TOOL_NAME {
            patch_count += 1;
        }
        if call.tool == TASKSPACE_CONTROL_TOOL_NAME {
            let action = map_action(call, index)?;
            validate_map_call(call, index, action)?;
            match action {
                "initialize_and_execute" | "reopen_map" => {
                    if initialization_index.replace(index).is_some() {
                        return Err(call_error(
                            "map_initialization_multiple",
                            index,
                            call,
                            "TaskSpace Exec plan contains multiple initialize/reopen calls",
                        ));
                    }
                }
                "finish_map" => {
                    if terminal_map_index.replace(index).is_some() {
                        return Err(call_error(
                            "map_finish_multiple",
                            index,
                            call,
                            "TaskSpace Exec plan contains multiple finish_map calls",
                        ));
                    }
                }
                _ => {}
            }
        } else {
            validate_work_binding(call, index)?;
        }
    }

    if patch_count > 1 {
        return Err(plan_error(
            "multiple_apply_patch_calls",
            format!("TaskSpace Exec plan contains {patch_count} apply_patch calls; maximum is 1"),
        ));
    }
    if let Some(index) = initialization_index {
        if index != 0 || plan.calls.len() == 1 {
            return Err(call_error(
                "map_initialization_boundary_invalid",
                index,
                &plan.calls[index],
                "initialize/reopen must be the first call and must be followed by work",
            ));
        }
    }
    if let Some(index) = terminal_map_index {
        if index + 1 != plan.calls.len() {
            return Err(call_error(
                "map_finish_not_terminal",
                index,
                &plan.calls[index],
                "finish_map must be the final client call",
            ));
        }
    }

    Ok(ValidatedTaskspaceExecPlan(plan))
}

fn validate_call_identity(
    call: &TaskspaceExecCall,
    index: usize,
    item_ids: &mut BTreeSet<String>,
) -> Result<(), TaskspaceExecPreflightError> {
    if call.item_id.trim().is_empty() {
        return Err(call_error(
            "item_id_empty",
            index,
            call,
            "TaskSpace Exec call requires a non-empty item_id",
        ));
    }
    if !item_ids.insert(call.item_id.clone()) {
        return Err(call_error(
            "item_id_duplicate",
            index,
            call,
            format!("TaskSpace Exec item_id `{}` is duplicated", call.item_id),
        ));
    }
    if call.tool.trim().is_empty() {
        return Err(call_error(
            "tool_name_empty",
            index,
            call,
            "TaskSpace Exec call requires a non-empty tool name",
        ));
    }
    Ok(())
}

fn validate_call_contract(
    call: &TaskspaceExecCall,
    index: usize,
    catalog: &TaskspaceExecCatalog,
) -> Result<(), TaskspaceExecPreflightError> {
    if matches!(
        call.tool.as_str(),
        TASKSPACE_EXEC_TOOL_NAME | "exec" | "wait"
    ) {
        return Err(call_error(
            "recursive_tool_forbidden",
            index,
            call,
            format!("Tool `{}` cannot be nested in TaskSpace Exec", call.tool),
        ));
    }
    let Some(capability) = catalog.capability(&call.tool) else {
        return Err(call_error(
            "tool_not_in_capability_catalog",
            index,
            call,
            format!(
                "Tool `{}` is not in the admitted capability catalog",
                call.tool
            ),
        ));
    };
    let input_matches = match capability.kind {
        CodeModeToolKind::Function => call.input.is_object(),
        CodeModeToolKind::Freeform => call.input.is_string(),
    };
    if !input_matches {
        return Err(call_error(
            "tool_input_kind_mismatch",
            index,
            call,
            format!(
                "Tool `{}` input does not match its {:?} contract",
                call.tool, capability.kind
            ),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::collections::BTreeMap;

    use codex_tools::FreeformTool;
    use codex_tools::FreeformToolFormat;
    use codex_tools::JsonSchema;
    use codex_tools::ResponsesApiTool;
    use codex_tools::ToolSpec;
    use serde_json::Value;
    use serde_json::json;

    use super::*;
    use crate::tools::taskspace_exec::catalog::TaskspaceExecCatalog;

    fn function(name: &str) -> ToolSpec {
        ToolSpec::Function(ResponsesApiTool {
            name: name.to_string(),
            description: format!("{name} description"),
            strict: false,
            parameters: JsonSchema::object(BTreeMap::new(), None, Some(false.into())),
            output_schema: None,
            defer_loading: None,
        })
    }

    fn catalog() -> TaskspaceExecCatalog {
        TaskspaceExecCatalog::from_tool_specs(&[
            function(TASKSPACE_CONTROL_TOOL_NAME),
            function("read_file"),
            ToolSpec::Freeform(FreeformTool {
                name: APPLY_PATCH_TOOL_NAME.to_string(),
                description: "Patch once.".to_string(),
                format: FreeformToolFormat {
                    r#type: "grammar".to_string(),
                    syntax: "lark".to_string(),
                    definition: "start: /[\\s\\S]+/".to_string(),
                },
            }),
        ])
        .expect("valid catalog")
    }

    fn call(item_id: &str, tool: &str, node_id: Option<&str>, input: Value) -> TaskspaceExecCall {
        TaskspaceExecCall {
            item_id: item_id.to_string(),
            tool: tool.to_string(),
            input,
            node_id: node_id.map(str::to_string),
        }
    }

    fn plan(catalog: &TaskspaceExecCatalog, calls: Vec<TaskspaceExecCall>) -> TaskspaceExecPlan {
        TaskspaceExecPlan {
            version: TASKSPACE_EXEC_PLAN_VERSION.to_string(),
            capability_id: catalog.identity.clone(),
            calls,
            hosted_bindings: Vec::new(),
        }
    }

    #[test]
    fn initialization_work_and_terminal_shapes_are_admitted() {
        let catalog = catalog();
        let valid = plan(
            &catalog,
            vec![
                call(
                    "init",
                    TASKSPACE_CONTROL_TOOL_NAME,
                    None,
                    json!({"action": "initialize_and_execute"}),
                ),
                call("read", "read_file", Some("inspect"), json!({})),
                call(
                    "finish",
                    TASKSPACE_CONTROL_TOOL_NAME,
                    None,
                    json!({"action": "finish_map", "expected_revision": 2}),
                ),
            ],
        );

        let admitted = preflight_taskspace_exec_plan(valid, &catalog).expect("valid plan");
        assert_eq!(admitted.as_plan().calls.len(), 3);
    }

    #[test]
    fn failures_happen_before_a_dispatch_boundary() {
        let catalog = catalog();
        let invalid = plan(&catalog, vec![call("read", "read_file", None, json!({}))]);
        let dispatch_count = Cell::new(0);

        let admitted = preflight_taskspace_exec_plan(invalid, &catalog);
        if admitted.is_ok() {
            dispatch_count.set(dispatch_count.get() + 1);
        }

        assert_eq!(dispatch_count.get(), 0);
        assert_eq!(
            admitted.expect_err("missing binding").reason_code,
            "work_node_binding_missing"
        );
    }

    #[test]
    fn version_catalog_recursion_and_input_kind_fail_closed() {
        let catalog = catalog();
        let mut wrong_version = plan(
            &catalog,
            vec![call("read", "read_file", Some("node"), json!({}))],
        );
        wrong_version.version = "unknown".to_string();
        assert_eq!(
            preflight_taskspace_exec_plan(wrong_version, &catalog)
                .expect_err("version")
                .reason_code,
            "plan_version_mismatch"
        );

        let mut wrong_catalog = plan(
            &catalog,
            vec![call("read", "read_file", Some("node"), json!({}))],
        );
        wrong_catalog.capability_id = "sha256:stale".to_string();
        assert_eq!(
            preflight_taskspace_exec_plan(wrong_catalog, &catalog)
                .expect_err("catalog")
                .reason_code,
            "capability_identity_mismatch"
        );

        let recursive = plan(
            &catalog,
            vec![call("nested", "taskspace_exec", Some("node"), json!({}))],
        );
        assert_eq!(
            preflight_taskspace_exec_plan(recursive, &catalog)
                .expect_err("recursion")
                .reason_code,
            "recursive_tool_forbidden"
        );

        let wrong_input = plan(
            &catalog,
            vec![call(
                "patch",
                APPLY_PATCH_TOOL_NAME,
                Some("node"),
                json!({}),
            )],
        );
        assert_eq!(
            preflight_taskspace_exec_plan(wrong_input, &catalog)
                .expect_err("input kind")
                .reason_code,
            "tool_input_kind_mismatch"
        );
    }

    #[test]
    fn duplicate_items_multiple_patches_and_invalid_map_boundaries_fail() {
        let catalog = catalog();
        let duplicate = plan(
            &catalog,
            vec![
                call("same", "read_file", Some("node"), json!({})),
                call("same", "read_file", Some("node"), json!({})),
            ],
        );
        assert_eq!(
            preflight_taskspace_exec_plan(duplicate, &catalog)
                .expect_err("duplicate")
                .reason_code,
            "item_id_duplicate"
        );

        let patches = plan(
            &catalog,
            vec![
                call("p1", APPLY_PATCH_TOOL_NAME, Some("node"), json!("a")),
                call("p2", APPLY_PATCH_TOOL_NAME, Some("node"), json!("b")),
            ],
        );
        assert_eq!(
            preflight_taskspace_exec_plan(patches, &catalog)
                .expect_err("patch count")
                .reason_code,
            "multiple_apply_patch_calls"
        );

        let late_init = plan(
            &catalog,
            vec![
                call("read", "read_file", Some("node"), json!({})),
                call(
                    "init",
                    TASKSPACE_CONTROL_TOOL_NAME,
                    None,
                    json!({"action": "reopen_map", "expected_revision": 1}),
                ),
            ],
        );
        assert_eq!(
            preflight_taskspace_exec_plan(late_init, &catalog)
                .expect_err("late init")
                .reason_code,
            "map_initialization_boundary_invalid"
        );

        let early_finish = plan(
            &catalog,
            vec![
                call(
                    "finish",
                    TASKSPACE_CONTROL_TOOL_NAME,
                    None,
                    json!({"action": "finish_map", "expected_revision": 2}),
                ),
                call("read", "read_file", Some("node"), json!({})),
            ],
        );
        assert_eq!(
            preflight_taskspace_exec_plan(early_finish, &catalog)
                .expect_err("early finish")
                .reason_code,
            "map_finish_not_terminal"
        );
    }

    #[test]
    fn hosted_only_plan_remains_structurally_admissible_for_reconciliation() {
        let catalog = catalog();
        let mut hosted = plan(&catalog, Vec::new());
        hosted.hosted_bindings = vec![super::super::plan::TaskspaceExecHostedBinding {
            tool: "web_search".to_string(),
            node_ids: vec!["research".to_string(), "compare".to_string()],
        }];

        assert!(preflight_taskspace_exec_plan(hosted, &catalog).is_ok());
    }

    #[test]
    fn hosted_binding_fields_cannot_be_empty() {
        let catalog = catalog();
        let mut hosted = plan(&catalog, Vec::new());
        hosted.hosted_bindings = vec![super::super::plan::TaskspaceExecHostedBinding {
            tool: "web_search".to_string(),
            node_ids: vec!["  ".to_string()],
        }];

        assert_eq!(
            preflight_taskspace_exec_plan(hosted, &catalog)
                .expect_err("empty hosted node")
                .reason_code,
            "hosted_binding_node_id_empty"
        );

        let mut hosted = plan(&catalog, Vec::new());
        hosted.hosted_bindings = vec![super::super::plan::TaskspaceExecHostedBinding {
            tool: " ".to_string(),
            node_ids: vec!["research".to_string()],
        }];
        assert_eq!(
            preflight_taskspace_exec_plan(hosted, &catalog)
                .expect_err("empty hosted tool")
                .reason_code,
            "hosted_binding_tool_empty"
        );

        let mut hosted = plan(&catalog, Vec::new());
        hosted.hosted_bindings = vec![super::super::plan::TaskspaceExecHostedBinding {
            tool: "web_search".to_string(),
            node_ids: Vec::new(),
        }];
        assert_eq!(
            preflight_taskspace_exec_plan(hosted, &catalog)
                .expect_err("empty hosted node set")
                .reason_code,
            "hosted_binding_node_ids_empty"
        );

        let mut hosted = plan(&catalog, Vec::new());
        hosted.hosted_bindings = vec![super::super::plan::TaskspaceExecHostedBinding {
            tool: "web_search".to_string(),
            node_ids: vec!["research".to_string(), "research".to_string()],
        }];
        assert_eq!(
            preflight_taskspace_exec_plan(hosted, &catalog)
                .expect_err("duplicate hosted node")
                .reason_code,
            "hosted_binding_node_id_duplicate"
        );
    }
}
