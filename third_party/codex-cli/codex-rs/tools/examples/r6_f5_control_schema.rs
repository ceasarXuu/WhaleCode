use codex_tools::CommandToolOptions;
use codex_tools::create_apply_patch_freeform_tool;
use codex_tools::create_exec_command_tool;
use codex_tools::create_list_dir_tool;
use codex_tools::create_taskspace_control_tool;
use codex_tools::create_write_stdin_tool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let visible_tools = vec![
        create_exec_command_tool(CommandToolOptions {
            allow_login_shell: true,
            exec_permission_approvals_enabled: false,
        }),
        create_write_stdin_tool(),
        create_list_dir_tool(),
        create_apply_patch_freeform_tool(),
    ];
    let tool = create_taskspace_control_tool(&visible_tools);
    serde_json::to_writer(std::io::stdout(), &tool)?;
    Ok(())
}
