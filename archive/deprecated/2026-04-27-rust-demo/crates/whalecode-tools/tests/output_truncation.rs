use tempfile::tempdir;
use whalecode_tools::{ToolRequest, ToolRuntime};

#[test]
fn preserves_head_and_tail_when_output_is_truncated() {
    let dir = tempdir().expect("tempdir");
    let content = format!("{}{}", "a".repeat(64), "z".repeat(64));
    std::fs::write(dir.path().join("large.txt"), content).expect("write large file");

    let runtime = ToolRuntime::new(dir.path()).expect("runtime");
    let result = runtime
        .execute(ToolRequest::ReadFile {
            path: "large.txt".to_owned(),
            max_bytes: Some(40),
        })
        .expect("read file");

    assert!(result.truncated);
    assert_eq!(result.original_len, 128);
    assert!(result.content.starts_with("aaaaaaaa"));
    assert!(result.content.ends_with("zzzzzzzz"));
    assert!(result.content.contains("[truncated]"));
}
