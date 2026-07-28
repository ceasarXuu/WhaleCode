use codex_tools::create_taskspace_control_tool;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let tool = create_taskspace_control_tool();
    serde_json::to_writer(std::io::stdout(), &tool)?;
    Ok(())
}
