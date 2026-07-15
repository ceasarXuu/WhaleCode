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

    assert_schema_trees_match("typescript", &fixture_tree, &generated_tree)
}

#[test]
fn json_schema_fixtures_match_generated() -> Result<()> {
    assert_schema_fixtures_match_generated("json", |output_dir| {
        generate_json_with_experimental(output_dir, /*experimental_api*/ false)
    })
}

#[test]
fn action_map_result_schema_is_mechanical_and_source_linked() -> Result<()> {
    let schema_root = schema_root()?;
    let typescript_tree = read_tree(&schema_root, "typescript")?;
    let result_ts = fixture_utf8(
        &typescript_tree,
        Path::new("ActionMapSnapshotResult.ts"),
        "typescript",
    )?;
    assert!(result_ts.contains("toolSuccess: boolean | null"));
    assert!(result_ts.contains("sourceEventRef: string"));
    assert!(result_ts.contains("artifactRefs: Array<string>"));
    assert!(!result_ts.contains("evidencePackage"));
    assert!(!result_ts.contains("tool_success"));

    for (bundle_path, value) in json_bundles(&schema_root)? {
        let result = schema(&value, "ActionMapSnapshotResult")
            .with_context(|| format!("locate ActionMapSnapshotResult in {bundle_path}"))?;
        let properties = properties(result, "ActionMapSnapshotResult")?;
        for field in [
            "toolSuccess",
            "sourceEventRef",
            "artifactRefs",
            "sourceThreadId",
        ] {
            anyhow::ensure!(
                properties.contains_key(field),
                "{bundle_path} must expose result {field}"
            );
        }
        anyhow::ensure!(
            !properties.contains_key("evidencePackage"),
            "{bundle_path} must not expose semantic result evidencePackage"
        );
        assert_eq!(
            properties.get("toolSuccess"),
            Some(&serde_json::json!({
                "default": null,
                "type": ["boolean", "null"],
            }))
        );
    }
    Ok(())
}

#[test]
fn action_map_snapshot_schema_is_one_rooted_revisioned_map() -> Result<()> {
    let schema_root = schema_root()?;
    let typescript_tree = read_tree(&schema_root, "typescript")?;
    let snapshot_ts = fixture_utf8(
        &typescript_tree,
        Path::new("ActionMapSnapshot.ts"),
        "typescript",
    )?;
    let map_ts = fixture_utf8(
        &typescript_tree,
        Path::new("ActionMapSnapshotMap.ts"),
        "typescript",
    )?;
    let node_ts = fixture_utf8(
        &typescript_tree,
        Path::new("ActionMapSnapshotNode.ts"),
        "typescript",
    )?;
    assert!(snapshot_ts.contains("schemaVersion: string"));
    assert!(snapshot_ts.contains("map: ActionMapSnapshotMap | null"));
    assert!(!snapshot_ts.contains("maps:"));
    assert!(!snapshot_ts.contains("tasks:"));
    assert!(!snapshot_ts.contains("cognitiveSchemaVersion"));
    for field in [
        "rootNodeId: string",
        "finishNodeId: string",
        "revision: bigint",
        "complete: boolean",
        "nodes: Array<ActionMapSnapshotNode>",
        "edges: Array<ActionMapSnapshotEdge>",
    ] {
        assert!(map_ts.contains(field), "map fixture must expose {field}");
    }
    for field in ["role: string", "goal: string", "status: string"] {
        assert!(node_ts.contains(field), "node fixture must expose {field}");
    }
    assert!(!node_ts.contains("kind:"));

    for (bundle_path, value) in json_bundles(&schema_root)? {
        let snapshot = schema(&value, "ActionMapSnapshot")
            .with_context(|| format!("locate ActionMapSnapshot in {bundle_path}"))?;
        let snapshot_properties = properties(snapshot, "ActionMapSnapshot")?;
        for field in ["schemaVersion", "mode", "map", "traceEvents"] {
            anyhow::ensure!(
                snapshot_properties.contains_key(field),
                "{bundle_path} must expose snapshot {field}"
            );
        }
        anyhow::ensure!(
            !snapshot_properties.contains_key("maps")
                && !snapshot_properties.contains_key("tasks")
                && !snapshot_properties.contains_key("cognitiveSchemaVersion"),
            "{bundle_path} must not expose R5 snapshot authorities"
        );

        let map = schema(&value, "ActionMapSnapshotMap")
            .with_context(|| format!("locate ActionMapSnapshotMap in {bundle_path}"))?;
        let map_properties = properties(map, "ActionMapSnapshotMap")?;
        for field in [
            "rootNodeId",
            "finishNodeId",
            "revision",
            "complete",
            "nodes",
            "edges",
            "nodeEvents",
        ] {
            anyhow::ensure!(
                map_properties.contains_key(field),
                "{bundle_path} must expose map {field}"
            );
        }

        let node = schema(&value, "ActionMapSnapshotNode")
            .with_context(|| format!("locate ActionMapSnapshotNode in {bundle_path}"))?;
        let node_properties = properties(node, "ActionMapSnapshotNode")?;
        for field in ["id", "role", "goal", "status", "sourceRefs"] {
            anyhow::ensure!(
                node_properties.contains_key(field),
                "{bundle_path} must expose node {field}"
            );
        }
        anyhow::ensure!(
            !node_properties.contains_key("kind"),
            "{bundle_path} must not expose NodeKind"
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
    assert_schema_trees_match(label, &fixture_tree, &generated_tree)
}

fn assert_schema_trees_match(
    label: &str,
    fixture_tree: &BTreeMap<PathBuf, Vec<u8>>,
    generated_tree: &BTreeMap<PathBuf, Vec<u8>>,
) -> Result<()> {
    let fixture_paths = fixture_tree
        .keys()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    let generated_paths = generated_tree
        .keys()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    if fixture_paths != generated_paths {
        let diff = TextDiff::from_lines(&fixture_paths.join("\n"), &generated_paths.join("\n"))
            .unified_diff()
            .header("fixture", "generated")
            .to_string();
        panic!(
            "Vendored {label} schema file set differs from generated output. Run `just write-app-server-schema`.\n\n{diff}"
        );
    }
    for (path, expected) in fixture_tree {
        let actual = generated_tree
            .get(path)
            .ok_or_else(|| anyhow::anyhow!("missing generated file: {}", path.display()))?;
        if expected != actual {
            let diff = TextDiff::from_lines(
                &String::from_utf8_lossy(expected),
                &String::from_utf8_lossy(actual),
            )
            .unified_diff()
            .header("fixture", "generated")
            .to_string();
            panic!(
                "Vendored {label} fixture {} differs from generated output. Run `just write-app-server-schema`.\n\n{diff}",
                path.display()
            );
        }
    }
    Ok(())
}

fn json_bundles(schema_root: &Path) -> Result<Vec<(&'static str, Value)>> {
    let tree = read_tree(schema_root, "json")?;
    [
        "codex_app_server_protocol.schemas.json",
        "codex_app_server_protocol.v2.schemas.json",
    ]
    .into_iter()
    .map(|path| {
        let text = fixture_utf8(&tree, Path::new(path), "json")?;
        let value =
            serde_json::from_str(text).with_context(|| format!("parse {path} schema fixture"))?;
        Ok((path, value))
    })
    .collect()
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

fn schema<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value
        .pointer(&format!("/definitions/v2/{name}"))
        .or_else(|| value.pointer(&format!("/definitions/{name}")))
}

fn properties<'a>(value: &'a Value, label: &str) -> Result<&'a serde_json::Map<String, Value>> {
    value
        .get("properties")
        .and_then(Value::as_object)
        .with_context(|| format!("{label} properties"))
}

fn schema_root() -> Result<PathBuf> {
    let typescript_index = codex_utils_cargo_bin::find_resource!("schema/typescript/index.ts")
        .context("resolve TypeScript schema index.ts")?;
    let schema_root = typescript_index
        .parent()
        .and_then(|path| path.parent())
        .context("derive schema root from schema/typescript/index.ts")?
        .to_path_buf();
    let json_bundle =
        codex_utils_cargo_bin::find_resource!("schema/json/codex_app_server_protocol.schemas.json")
            .context("resolve JSON schema bundle")?;
    let json_root = json_bundle
        .parent()
        .and_then(|path| path.parent())
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
