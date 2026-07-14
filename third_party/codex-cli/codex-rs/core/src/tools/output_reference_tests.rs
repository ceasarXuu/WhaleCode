use super::*;

#[test]
fn policy_uses_inline_summarized_and_referenced_thresholds() {
    assert_eq!(
        policy_for_raw_output(&vec![b'a'; OUTPUT_INLINE_THRESHOLD_BYTES]),
        OutputReferencePolicy::Inline
    );
    assert_eq!(
        policy_for_raw_output(&vec![b'a'; OUTPUT_INLINE_THRESHOLD_BYTES + 1]),
        OutputReferencePolicy::Summarized
    );
    assert_eq!(
        policy_for_raw_output(&vec![b'a'; OUTPUT_REFERENCE_THRESHOLD_BYTES + 1]),
        OutputReferencePolicy::Referenced
    );
}

#[test]
fn reference_text_preserves_hash_and_bounded_edges() {
    let mut raw_output = Vec::new();
    raw_output.extend_from_slice(b"head-visible\n");
    raw_output.extend_from_slice("middle-secret-marker\n".repeat(4_000).as_bytes());
    raw_output.extend_from_slice(b"tail-visible\n");
    let text = reference_text_for_raw_output(&raw_output, Some("output-ref://sha256/test"))
        .expect("large output should be referenceized");
    assert!(text.contains("OutputReferenceV1:"));
    assert!(text.contains("output_ref: output-ref://sha256/test"));
    assert!(text.contains("policy: referenced_large_output"));
    assert!(text.contains("raw_output_elided: true"));
    assert!(text.contains("head-visible"));
    assert!(text.contains("tail-visible"));
    assert!(text.matches("middle-secret-marker").count() < 300);
}

#[test]
fn reference_text_redacts_sensitive_head_and_tail_values() {
    let mut raw_output = Vec::new();
    raw_output.extend_from_slice(b"api_key = sk-live-secret\n");
    raw_output.extend_from_slice("middle\n".repeat(20_000).as_bytes());
    raw_output.extend_from_slice(b"Authorization: Bearer tail-secret-token\n");
    let text = reference_text_for_raw_output(&raw_output, Some("output-ref://sha256/test"))
        .expect("large output should be referenceized");
    assert!(text.contains("api_key = [REDACTED]"));
    assert!(text.contains("Authorization: Bearer [REDACTED]"));
    assert!(!text.contains("sk-live-secret"));
    assert!(!text.contains("tail-secret-token"));
}

#[test]
fn read_output_bytes_slice_verifies_content_addressed_archive_bytes() {
    let raw_output = br#"{"schema_version":"LargeToolOutputV1","items":["a"]}"#;
    let sha = format!("{:x}", Sha256::digest(raw_output));
    let output_ref = format!("output-ref://sha256/{sha}");
    let slice = read_output_bytes_slice(
        &output_ref,
        raw_output,
        OutputSliceRequest {
            mode: OutputSliceMode::Head,
            max_bytes: 4096,
        },
    )
    .expect("read archive bytes");
    assert!(slice.contains("LargeToolOutputV1"));

    let error = read_output_bytes_slice(
        &output_ref,
        b"corrupt",
        OutputSliceRequest {
            mode: OutputSliceMode::Head,
            max_bytes: 4096,
        },
    )
    .expect_err("corrupt archive bytes must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn output_artifact_is_portable_across_rollout_paths() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let writer = temp.path().join("sessions/2026/07/12/rollout-source.jsonl");
    let reader = temp.path().join("sessions/2026/07/13/rollout-fork.jsonl");
    let raw_output = "artifact-line\n".repeat(9000).into_bytes();
    let artifact_ref = write_output_artifact_for_rollout(Some(&writer), &raw_output)
        .await
        .expect("write artifact")
        .expect("artifact ref");
    let sha = artifact_ref.strip_prefix("output-ref://sha256/").unwrap();
    let artifact_path = temp
        .path()
        .join("session-store/output-refs/sha256")
        .join(format!("{sha}.stdout"));
    assert_eq!(tokio::fs::read(artifact_path).await.unwrap(), raw_output);
    let slice = read_output_artifact_slice(
        Some(&reader),
        &artifact_ref,
        OutputSliceRequest {
            mode: OutputSliceMode::Head,
            max_bytes: 128,
        },
    )
    .await
    .expect("read through fork path");
    assert!(slice.contains("artifact-line"));
}

#[tokio::test]
async fn read_output_artifact_slice_rejects_corrupt_content() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let rollout_path = temp.path().join("rollout-test.jsonl");
    let raw_output = "artifact-line\n".repeat(9000).into_bytes();
    let artifact_ref = write_output_artifact_for_rollout(Some(&rollout_path), &raw_output)
        .await
        .unwrap()
        .unwrap();
    let sha = artifact_ref.strip_prefix("output-ref://sha256/").unwrap();
    let artifact_path = output_artifact_dir(&rollout_path).join(format!("{sha}.stdout"));
    tokio::fs::write(artifact_path, b"corrupt").await.unwrap();
    let error = read_output_artifact_slice(
        Some(&rollout_path),
        &artifact_ref,
        OutputSliceRequest {
            mode: OutputSliceMode::Head,
            max_bytes: 128,
        },
    )
    .await
    .expect_err("corrupt artifact must fail");
    assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
}

#[tokio::test]
async fn read_output_artifact_slice_returns_bounded_grep() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let rollout_path = temp.path().join("rollout-test.jsonl");
    let raw_output = "alpha\nneedle one\nbeta\nneedle two\n"
        .repeat(1000)
        .into_bytes();
    let artifact_ref = write_output_artifact_for_rollout(Some(&rollout_path), &raw_output)
        .await
        .unwrap()
        .unwrap();
    let slice = read_output_artifact_slice(
        Some(&rollout_path),
        &artifact_ref,
        OutputSliceRequest {
            mode: OutputSliceMode::Grep {
                pattern: "needle".to_string(),
            },
            max_bytes: 128,
        },
    )
    .await
    .unwrap();
    assert!(slice.contains("needle one"));
    assert!(slice.len() < 512);
}

#[tokio::test]
async fn read_output_artifact_slice_redacts_sensitive_values() {
    let temp = tempfile::tempdir().expect("create temp dir");
    let rollout_path = temp.path().join("rollout-test.jsonl");
    let raw_output = "alpha\npassword: hunter2\nbeta\n".repeat(3000).into_bytes();
    let artifact_ref = write_output_artifact_for_rollout(Some(&rollout_path), &raw_output)
        .await
        .unwrap()
        .unwrap();
    let slice = read_output_artifact_slice(
        Some(&rollout_path),
        &artifact_ref,
        OutputSliceRequest {
            mode: OutputSliceMode::Grep {
                pattern: "password".to_string(),
            },
            max_bytes: 256,
        },
    )
    .await
    .unwrap();
    assert!(slice.contains("password: [REDACTED]"));
    assert!(!slice.contains("hunter2"));
}
