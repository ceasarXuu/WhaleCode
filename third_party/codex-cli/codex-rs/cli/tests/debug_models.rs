use std::path::Path;

use anyhow::Result;
use tempfile::TempDir;

fn whale_command(codex_home: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("whale")?);
    cmd.env("CODEX_HOME", codex_home);
    Ok(cmd)
}

fn assert_whale_scoped_models(value: &serde_json::Value) {
    let models = value["models"].as_array().expect("models array");
    assert!(!models.is_empty());
    assert!(
        models.iter().all(|model| {
            model["slug"]
                .as_str()
                .is_some_and(|slug| slug.starts_with("deepseek-"))
        }),
        "debug models must expose only Whale DeepSeek models"
    );
}

#[test]
fn debug_models_bundled_prints_json() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = whale_command(codex_home.path())?;
    let output = cmd.args(["debug", "models", "--bundled"]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_whale_scoped_models(&value);

    Ok(())
}

#[test]
fn debug_models_default_prints_json_without_auth() -> Result<()> {
    let codex_home = TempDir::new()?;
    let mut cmd = whale_command(codex_home.path())?;
    let output = cmd.args(["debug", "models"]).output()?;

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_whale_scoped_models(&value);

    Ok(())
}
