use std::path::Path;

use anyhow::Result;
use tempfile::TempDir;

fn whale_command(codex_home: &Path, descriptor_key: &Path) -> Result<assert_cmd::Command> {
    let mut cmd = assert_cmd::Command::new(codex_utils_cargo_bin::cargo_bin("whale")?);
    cmd.env("CODEX_HOME", codex_home);
    cmd.env("WHALE_PROVIDER_DESCRIPTOR_HMAC_KEY_FILE", descriptor_key);
    Ok(cmd)
}

fn alias_args() -> Vec<&'static str> {
    vec![
        "-m",
        "deepseek-v4-flash",
        "debug",
        "provider",
        "-c",
        "model_provider=\"deepseek-boundary\"",
        "-c",
        "model_providers.deepseek-boundary.name=\"DeepSeek\"",
        "-c",
        "model_providers.deepseek-boundary.base_url=\"http://provider-proxy:8080\"",
        "-c",
        "model_providers.deepseek-boundary.env_key=\"DEEPSEEK_API_KEY\"",
        "-c",
        "model_providers.deepseek-boundary.env_key_instructions=\"Set DEEPSEEK_API_KEY to a DeepSeek API key before starting Whale.\"",
        "-c",
        "model_providers.deepseek-boundary.wire_api=\"responses\"",
    ]
}

fn drifted_args(token: &'static str) -> Vec<&'static str> {
    let mut args = alias_args();
    args.extend([
        "-c",
        "model_providers.deepseek-boundary.query_params.review_probe=\"secret-query\"",
        "-c",
        "model_providers.deepseek-boundary.http_headers.x-review-probe=\"secret-header\"",
        "-c",
        token,
    ]);
    args
}

#[test]
fn debug_provider_loads_custom_alias_without_exposing_secret() -> Result<()> {
    let codex_home = TempDir::new()?;
    let descriptor_key = codex_home.path().join("provider-descriptor.key");
    std::fs::write(&descriptor_key, "test-only-hmac-key")?;

    let mut cmd = whale_command(codex_home.path(), &descriptor_key)?;
    cmd.env("DEEPSEEK_API_KEY", "must-not-appear");
    let output = cmd.args(alias_args()).output()?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8(output.stdout)?;
    assert!(!stdout.contains("must-not-appear"));
    assert!(!stdout.contains("http://provider-proxy:8080"));
    assert!(!stdout.contains("test-only-hmac-key"));
    let value: serde_json::Value = serde_json::from_str(&stdout)?;
    assert_eq!(value["schema_version"], "whalecode-resolved-provider-v1");
    assert_eq!(value["model_provider_id"], "deepseek-boundary");
    assert_eq!(value["model"], "deepseek-v4-flash");
    assert_eq!(value["provider"]["name"], "DeepSeek");
    assert_eq!(value["provider"]["env_key"], "DEEPSEEK_API_KEY");
    assert_eq!(value["provider"]["wire_api"], "responses");
    assert_eq!(value["provider"]["is_deepseek"], true);
    for field in [
        "experimental_bearer_token_hmac_sha256",
        "query_params_hmac_sha256",
        "http_headers_hmac_sha256",
    ] {
        assert_eq!(value["provider"][field], serde_json::Value::Null);
    }
    assert!(value["provider"]["base_url_hmac_sha256"].is_string());
    assert_eq!(
        value["provider_descriptor_sha256"]
            .as_str()
            .expect("descriptor sha")
            .len(),
        64
    );

    let mut drifted = whale_command(codex_home.path(), &descriptor_key)?;
    let drifted_output = drifted
        .args(drifted_args(
            "model_providers.deepseek-boundary.experimental_bearer_token=\"secret-bearer\"",
        ))
        .output()?;
    assert!(drifted_output.status.success());
    let drifted_stdout = String::from_utf8(drifted_output.stdout)?;
    for secret in ["secret-query", "secret-header", "secret-bearer"] {
        assert!(!drifted_stdout.contains(secret));
    }
    let drifted_value: serde_json::Value = serde_json::from_str(&drifted_stdout)?;
    assert_ne!(
        drifted_value["provider_descriptor_sha256"],
        value["provider_descriptor_sha256"]
    );
    for field in [
        "experimental_bearer_token_hmac_sha256",
        "query_params_hmac_sha256",
        "http_headers_hmac_sha256",
    ] {
        assert!(drifted_value["provider"][field].is_string());
    }

    let mut changed_token = whale_command(codex_home.path(), &descriptor_key)?;
    let changed_token_output = changed_token
        .args(drifted_args(
            "model_providers.deepseek-boundary.experimental_bearer_token=\"different-bearer\"",
        ))
        .output()?;
    assert!(changed_token_output.status.success());
    let changed_token_value: serde_json::Value =
        serde_json::from_slice(&changed_token_output.stdout)?;
    assert_ne!(
        changed_token_value["provider"]["experimental_bearer_token_hmac_sha256"],
        drifted_value["provider"]["experimental_bearer_token_hmac_sha256"]
    );

    let other_key = codex_home.path().join("other-provider-descriptor.key");
    std::fs::write(&other_key, "different-test-only-hmac-key")?;
    let mut rekeyed = whale_command(codex_home.path(), &other_key)?;
    let rekeyed_output = rekeyed
        .args(drifted_args(
            "model_providers.deepseek-boundary.experimental_bearer_token=\"secret-bearer\"",
        ))
        .output()?;
    assert!(rekeyed_output.status.success());
    let rekeyed_value: serde_json::Value = serde_json::from_slice(&rekeyed_output.stdout)?;
    for field in [
        "experimental_bearer_token_hmac_sha256",
        "query_params_hmac_sha256",
        "http_headers_hmac_sha256",
    ] {
        assert_ne!(
            rekeyed_value["provider"][field],
            drifted_value["provider"][field]
        );
    }
    Ok(())
}
