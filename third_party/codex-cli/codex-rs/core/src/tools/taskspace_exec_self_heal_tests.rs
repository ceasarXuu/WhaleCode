use codex_protocol::models::ResponseItem;
use codex_tools::FreeformTool;
use codex_tools::FreeformToolFormat;
use codex_tools::ToolSpec;

use super::*;

fn catalog() -> TaskSpaceExecCatalog {
    TaskSpaceExecCatalog::build(&[ToolSpec::Freeform(FreeformTool {
        name: "apply_patch".into(),
        description: "Apply one patch.".into(),
        defer_loading: None,
        format: FreeformToolFormat {
            r#type: "grammar".into(),
            syntax: "lark".into(),
            definition: "start: /.+/".into(),
        },
    })])
    .expect("catalog")
}

fn call(arguments: &str) -> ResponseItem {
    ResponseItem::FunctionCall {
        id: None,
        name: TASKSPACE_EXEC_TOOL_NAME.into(),
        namespace: None,
        arguments: arguments.into(),
        encrypted_function_args: None,
        call_id: "observed-wrapper".into(),
        internal_chat_message_metadata_passthrough: None,
    }
}

#[test]
fn repairs_observed_apply_patch_wrapper_and_missing_action_brace() {
    let patch =
        "*** Begin Patch\n*** Update File: /workspace/example.py\n@@\n-old\n+new\n*** End Patch";
    let malformed = format!(
        r#"{{"type":"update_and_work","update_map":{{"add_work_nodes":[],"node_patches":[{{"node_id":"explore","state":"completed","content":"inspection complete"}}]}},"tools":[{{"input":{{"cmd":{},"node_id":"fix","tool":"apply_patch"}}]}}"#,
        serde_json::to_string(patch).unwrap()
    );
    let catalog = catalog();
    let mut item = call(&malformed);

    let repair = self_heal_taskspace_exec_response_item(&mut item, &catalog).expect("repair");

    assert_eq!(repair.operation, "normalize");
    assert_eq!(repair.repair_token, "apply_patch_wrapper");
    let ResponseItem::FunctionCall { arguments, .. } = item else {
        panic!("function call")
    };
    let plan = catalog.decode_plan(&arguments).expect("repaired plan");
    assert_eq!(plan.sequence_type, "update_and_work");
    assert_eq!(plan.tools.len(), 1);
    assert_eq!(plan.tools[0].node_id, "fix");
    assert_eq!(plan.tools[0].tool_name.name, "apply_patch");
    assert_eq!(plan.tools[0].input, ClientCallInput::Freeform(patch.into()));
}

#[test]
fn refuses_to_normalize_a_non_patch_cmd_wrapper() {
    let malformed = r#"{"type":"work","tools":[{"input":{"cmd":"echo unsafe","node_id":"fix","tool":"apply_patch"}]}"#;
    let mut item = call(malformed);

    assert!(self_heal_taskspace_exec_response_item(&mut item, &catalog()).is_none());
    assert_eq!(item, call(malformed));
}
