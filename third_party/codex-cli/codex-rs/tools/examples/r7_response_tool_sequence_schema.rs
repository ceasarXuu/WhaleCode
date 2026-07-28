use codex_tools::create_apply_patch_json_tool;
use codex_tools::create_taskspace_control_tool;
use serde_json::json;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let control = create_taskspace_control_tool();
    let patch = create_apply_patch_json_tool();
    serde_json::to_writer(
        std::io::stdout(),
        &json!({"taskspace_control": control, "apply_patch": patch}),
    )?;
    Ok(())
}
