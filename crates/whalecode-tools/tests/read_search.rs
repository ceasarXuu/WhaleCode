use tempfile::tempdir;
use whalecode_tools::{ToolError, ToolRequest, ToolRuntime};

#[test]
fn lists_workspace_files_deterministically_and_ignores_runtime_dirs() {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".git")).expect("mkdir git metadata");
    std::fs::write(dir.path().join("README.md"), "hello").expect("write readme");
    std::fs::create_dir(dir.path().join("src")).expect("mkdir src");
    std::fs::write(dir.path().join("src/lib.rs"), "pub fn answer() {}").expect("write lib");
    std::fs::create_dir(dir.path().join("target")).expect("mkdir target");
    std::fs::write(dir.path().join("target/generated.txt"), "skip").expect("write target");

    let runtime = ToolRuntime::new(dir.path()).expect("runtime");
    let result = runtime
        .execute(ToolRequest::ListFiles { max_entries: 10 })
        .expect("list files");

    assert_eq!(result.content, "README.md\nsrc/lib.rs");
    assert!(!result.truncated);
}

#[test]
fn list_files_respects_local_privacy_boundaries() {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".git")).expect("mkdir git metadata");
    std::fs::write(dir.path().join("README.md"), "hello").expect("write readme");
    std::fs::write(dir.path().join(".gitignore"), "secret.log\n").expect("write gitignore");
    std::fs::write(dir.path().join("secret.log"), "token").expect("write secret");
    std::fs::create_dir(dir.path().join(".claude")).expect("mkdir claude");
    std::fs::write(dir.path().join(".claude/settings.local.json"), "{}")
        .expect("write claude settings");

    let runtime = ToolRuntime::new(dir.path()).expect("runtime");
    let result = runtime
        .execute(ToolRequest::ListFiles { max_entries: 10 })
        .expect("list files");

    assert!(result.content.contains("README.md"));
    assert!(!result.content.contains("secret.log"));
    assert!(!result.content.contains(".claude"));
}

#[test]
fn reads_files_with_metadata() {
    let dir = tempdir().expect("tempdir");
    std::fs::write(dir.path().join("README.md"), "hello").expect("write readme");

    let runtime = ToolRuntime::new(dir.path()).expect("runtime");
    let result = runtime
        .execute(ToolRequest::ReadFile {
            path: "README.md".to_owned(),
            max_bytes: None,
        })
        .expect("read file");

    assert_eq!(result.content, "hello");
    assert_eq!(result.metadata["tool"], "read_file");
    assert_eq!(result.metadata["path"], "README.md");
}

#[test]
fn refuses_to_read_hidden_agent_config_paths() {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join(".claude")).expect("mkdir claude");
    std::fs::write(dir.path().join(".claude/settings.local.json"), "{}")
        .expect("write claude settings");

    let runtime = ToolRuntime::new(dir.path()).expect("runtime");
    let err = runtime
        .execute(ToolRequest::ReadFile {
            path: ".claude/settings.local.json".to_owned(),
            max_bytes: None,
        })
        .expect_err("hidden path");

    assert!(matches!(err, ToolError::HiddenPath(_)));
}

#[test]
fn rejects_parent_directory_escape() {
    let dir = tempdir().expect("tempdir");
    let runtime = ToolRuntime::new(dir.path()).expect("runtime");

    let err = runtime
        .execute(ToolRequest::ReadFile {
            path: "../outside.txt".to_owned(),
            max_bytes: None,
        })
        .expect_err("reject escape");

    assert!(matches!(err, ToolError::OutsideWorkspace(_)));
}

#[test]
fn searches_text_with_file_and_line_metadata() {
    let dir = tempdir().expect("tempdir");
    std::fs::create_dir(dir.path().join("src")).expect("mkdir src");
    std::fs::write(
        dir.path().join("src/lib.rs"),
        "pub fn whale() {}\npub fn other() {}",
    )
    .expect("write lib");

    let runtime = ToolRuntime::new(dir.path()).expect("runtime");
    let result = runtime
        .execute(ToolRequest::SearchText {
            query: "whale".to_owned(),
            max_matches: 10,
            max_bytes: None,
        })
        .expect("search text");

    assert_eq!(result.content, "src/lib.rs:1:pub fn whale() {}");
}
