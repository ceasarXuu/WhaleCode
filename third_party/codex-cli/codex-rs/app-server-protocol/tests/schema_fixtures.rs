use anyhow::Context;
use anyhow::Result;
use codex_app_server_protocol::generate_json_with_experimental;
use codex_app_server_protocol::generate_typescript_schema_fixture_subtree_for_tests;
use codex_app_server_protocol::read_schema_fixture_subtree;
use serde_json::Value;
use similar::TextDiff;
use std::collections::BTreeMap;
use std::path::Path;
use std::path::PathBuf;

#[test]
fn typescript_schema_fixtures_match_generated() -> Result<()> {
    let schema_root = schema_root()?;
    let fixture_tree = read_tree(&schema_root, "typescript")?;
    let generated_tree = generate_typescript_schema_fixture_subtree_for_tests()
        .context("generate in-memory typescript schema fixtures")?;

    assert_schema_trees_match("typescript", &fixture_tree, &generated_tree)?;

    Ok(())
}

#[test]
fn json_schema_fixtures_match_generated() -> Result<()> {
    assert_schema_fixtures_match_generated("json", |output_dir| {
        generate_json_with_experimental(output_dir, /*experimental_api*/ false)
    })
}

#[test]
fn action_map_snapshot_result_schema_exposes_tool_success() -> Result<()> {
    let schema_root = schema_root()?;

    let typescript_tree = read_tree(&schema_root, "typescript")?;
    let action_map_result_ts = fixture_utf8(
        &typescript_tree,
        Path::new("ActionMapSnapshotResult.ts"),
        "typescript",
    )?;
    assert!(
        action_map_result_ts.contains("toolSuccess: boolean | null"),
        "ActionMapSnapshotResult TypeScript fixture must expose toolSuccess"
    );
    assert!(
        !action_map_result_ts.contains("tool_success"),
        "ActionMapSnapshotResult TypeScript fixture must use camelCase"
    );

    let json_tree = read_tree(&schema_root, "json")?;
    for bundle_path in [
        "codex_app_server_protocol.schemas.json",
        "codex_app_server_protocol.v2.schemas.json",
    ] {
        let json = fixture_utf8(&json_tree, Path::new(bundle_path), "json")?;
        let value: Value = serde_json::from_str(json)
            .with_context(|| format!("parse {bundle_path} schema fixture"))?;
        let result_schema = action_map_result_schema(&value)
            .with_context(|| format!("locate ActionMapSnapshotResult in {bundle_path}"))?;
        let properties = result_schema
            .get("properties")
            .and_then(Value::as_object)
            .context("ActionMapSnapshotResult properties")?;

        anyhow::ensure!(
            properties.contains_key("toolSuccess"),
            "{bundle_path} must expose toolSuccess"
        );
        anyhow::ensure!(
            !properties.contains_key("tool_success"),
            "{bundle_path} must not expose snake_case tool_success"
        );
        assert_eq!(
            properties.get("toolSuccess"),
            Some(&serde_json::json!({
                "default": null,
                "type": ["boolean", "null"],
            })),
            "{bundle_path} must keep toolSuccess nullable boolean schema"
        );
    }

    Ok(())
}

fn assert_schema_fixtures_match_generated(
    label: &'static str,
    generate: impl FnOnce(&Path) -> Result<()>,
) -> Result<()> {
    let schema_root = schema_root()?;
    let fixture_tree = read_tree(&schema_root, label)?;

    let temp_dir = tempfile::tempdir().context("create temp dir")?;
    let generated_root = temp_dir.path().join(label);
    generate(&generated_root).with_context(|| {
        format!(
            "generate {label} schema fixtures into {}",
            generated_root.display()
        )
    })?;

    let generated_tree = read_tree(temp_dir.path(), label)?;

    assert_schema_trees_match(label, &fixture_tree, &generated_tree)?;

    Ok(())
}

fn assert_schema_trees_match(
    label: &str,
    fixture_tree: &BTreeMap<PathBuf, Vec<u8>>,
    generated_tree: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<()> {
    let fixture_paths = fixture_tree
        .keys()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>();
    let generated_paths = generated_tree
        .keys()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>();

    if fixture_paths != generated_paths {
        let expected = fixture_paths.join("\n");
        let actual = generated_paths.join("\n");
        let diff = TextDiff::from_lines(&expected, &actual)
            .unified_diff()
            .header("fixture", "generated")
            .to_string();

        panic!(
            "Vendored {label} app-server schema fixture file set doesn't match freshly generated output. \
Run `just write-app-server-schema` to overwrite with your changes.\n\n{diff}"
        );
    }

    // If the file sets match, diff contents for each file for a nicer error.
    for (path, expected) in fixture_tree {
        let actual = generated_tree
            .get(path)
            .ok_or_else(|| anyhow::anyhow!("missing generated file: {}", path.display()))?;

        if expected == actual {
            continue;
        }

        let expected_str = String::from_utf8_lossy(expected);
        let actual_str = String::from_utf8_lossy(actual);
        let diff = TextDiff::from_lines(&expected_str, &actual_str)
            .unified_diff()
            .header("fixture", "generated")
            .to_string();
        panic!(
            "Vendored {label} app-server schema fixture {} differs from generated output. \
Run `just write-app-server-schema` to overwrite with your changes.\n\n{diff}",
            path.display()
        );
    }

    Ok(())
}

fn fixture_utf8<'a>(
    tree: &'a BTreeMap<PathBuf, Vec<u8>>,
    path: &Path,
    label: &str,
) -> Result<&'a str> {
    let bytes = tree
        .get(path)
        .ok_or_else(|| anyhow::anyhow!("missing {label} fixture: {}", path.display()))?;
    std::str::from_utf8(bytes)
        .with_context(|| format!("read UTF-8 {label} fixture {}", path.display()))
}

fn action_map_result_schema(value: &Value) -> Option<&Value> {
    value
        .pointer("/definitions/v2/ActionMapSnapshotResult")
        .or_else(|| value.pointer("/definitions/ActionMapSnapshotResult"))
}

fn schema_root() -> Result<PathBuf> {
    // In Bazel runfiles (especially manifest-only mode), resolving directories is not
    // reliable. Resolve a known file, then walk up to the schema root.
    let typescript_index = codex_utils_cargo_bin::find_resource!("schema/typescript/index.ts")
        .context("resolve TypeScript schema index.ts")?;
    let schema_root = typescript_index
        .parent()
        .and_then(|p| p.parent())
        .context("derive schema root from schema/typescript/index.ts")?
        .to_path_buf();

    // Sanity check that the JSON fixtures resolve to the same schema root.
    let json_bundle =
        codex_utils_cargo_bin::find_resource!("schema/json/codex_app_server_protocol.schemas.json")
            .context("resolve JSON schema bundle")?;
    let json_root = json_bundle
        .parent()
        .and_then(|p| p.parent())
        .context("derive schema root from schema/json/codex_app_server_protocol.schemas.json")?;
    anyhow::ensure!(
        schema_root == json_root,
        "schema roots disagree: typescript={} json={}",
        schema_root.display(),
        json_root.display()
    );

    Ok(schema_root)
}

fn read_tree(root: &Path, label: &str) -> Result<BTreeMap<PathBuf, Vec<u8>>> {
    read_schema_fixture_subtree(root, label).with_context(|| {
        format!(
            "read {label} schema fixture subtree from {}",
            root.display()
        )
    })
}
