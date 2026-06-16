use codex_utils_string::take_bytes_at_char_boundary;
use sha2::Digest;
use sha2::Sha256;
use std::path::Path;
use std::path::PathBuf;

pub(crate) const OUTPUT_INLINE_THRESHOLD_BYTES: usize = 8 * 1024;
pub(crate) const OUTPUT_REFERENCE_THRESHOLD_BYTES: usize = 50 * 1024;
pub(crate) const OUTPUT_REFERENCE_SLICE_BYTES: usize = 2 * 1024;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum OutputReferencePolicy {
    Inline,
    Summarized,
    Referenced,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct OutputReferenceV1 {
    pub(crate) policy: OutputReferencePolicy,
    pub(crate) sha256: String,
    pub(crate) artifact_ref: Option<String>,
    pub(crate) bytes: usize,
    pub(crate) lines: usize,
    pub(crate) head: String,
    pub(crate) tail: String,
}

impl OutputReferenceV1 {
    pub(crate) fn from_raw_output(raw_output: &[u8], policy: OutputReferencePolicy) -> Self {
        let text = String::from_utf8_lossy(raw_output);
        let sha256 = Sha256::digest(raw_output);
        Self {
            policy,
            sha256: format!("{sha256:x}"),
            artifact_ref: None,
            bytes: raw_output.len(),
            lines: text.lines().count(),
            head: take_bytes_at_char_boundary(&text, OUTPUT_REFERENCE_SLICE_BYTES).to_string(),
            tail: take_tail_bytes_at_char_boundary(&text, OUTPUT_REFERENCE_SLICE_BYTES),
        }
    }

    pub(crate) fn to_model_visible_text(&self) -> String {
        let artifact_ref = self.artifact_ref.as_deref().unwrap_or("unavailable");
        format!(
            "OutputReferenceV1:\nsha256: {}\nartifact_ref: {}\nbytes: {}\nlines: {}\npolicy: {}\ninline_head_bytes: {}\ninline_tail_bytes: {}\n\n[head]\n{}\n\n[tail]\n{}",
            self.sha256,
            artifact_ref,
            self.bytes,
            self.lines,
            self.policy.as_str(),
            self.head.len(),
            self.tail.len(),
            self.head,
            self.tail,
        )
    }
}

impl OutputReferencePolicy {
    fn as_str(self) -> &'static str {
        match self {
            Self::Inline => "inline_output",
            Self::Summarized => "summarized_medium_output",
            Self::Referenced => "referenced_large_output",
        }
    }
}

impl OutputReferenceV1 {
    pub(crate) fn with_artifact_ref(mut self, artifact_ref: Option<String>) -> Self {
        self.artifact_ref = artifact_ref;
        self
    }
}

pub(crate) fn policy_for_raw_output(raw_output: &[u8]) -> OutputReferencePolicy {
    if raw_output.len() <= OUTPUT_INLINE_THRESHOLD_BYTES {
        OutputReferencePolicy::Inline
    } else if raw_output.len() <= OUTPUT_REFERENCE_THRESHOLD_BYTES {
        OutputReferencePolicy::Summarized
    } else {
        OutputReferencePolicy::Referenced
    }
}

pub(crate) fn reference_text_for_raw_output(
    raw_output: &[u8],
    artifact_ref: Option<&str>,
) -> Option<String> {
    let policy = policy_for_raw_output(raw_output);
    if policy == OutputReferencePolicy::Inline {
        return None;
    }
    Some(
        OutputReferenceV1::from_raw_output(raw_output, policy)
            .with_artifact_ref(artifact_ref.map(str::to_string))
            .to_model_visible_text(),
    )
}

pub(crate) async fn write_output_artifact_for_rollout(
    rollout_path: Option<&Path>,
    raw_output: &[u8],
) -> std::io::Result<Option<String>> {
    if policy_for_raw_output(raw_output) == OutputReferencePolicy::Inline {
        return Ok(None);
    }
    let Some(rollout_path) = rollout_path else {
        return Ok(None);
    };

    let output_ref =
        OutputReferenceV1::from_raw_output(raw_output, policy_for_raw_output(raw_output));
    let artifact_dir = output_artifact_dir(rollout_path);
    tokio::fs::create_dir_all(&artifact_dir).await?;
    let artifact_path = artifact_dir.join(format!("{}.stdout", output_ref.sha256));
    tokio::fs::write(&artifact_path, raw_output).await?;
    Ok(Some(format!("output-ref://sha256/{}", output_ref.sha256)))
}

fn output_artifact_dir(rollout_path: &Path) -> PathBuf {
    let stem = rollout_path
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("thread");
    let parent = rollout_path.parent().unwrap_or_else(|| Path::new("."));
    parent.join(format!("{stem}-artifacts")).join("output-refs")
}

fn take_tail_bytes_at_char_boundary(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let start = text.len().saturating_sub(max_bytes);
    let start = (start..=text.len())
        .find(|idx| text.is_char_boundary(*idx))
        .unwrap_or(text.len());
    text[start..].to_string()
}

#[cfg(test)]
mod tests {
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
        assert!(text.contains("policy: referenced_large_output"));
        assert!(text.contains("artifact_ref: output-ref://sha256/test"));
        assert!(text.contains("sha256:"));
        assert!(text.contains("head-visible"));
        assert!(text.contains("tail-visible"));
        assert!(text.matches("middle-secret-marker").count() < 300);
    }

    #[tokio::test]
    async fn write_output_artifact_uses_rollout_sibling_directory() {
        let temp = tempfile::tempdir().expect("create temp dir");
        let rollout_path = temp.path().join("rollout-test.jsonl");
        let raw_output = "artifact-line\n".repeat(9000).into_bytes();

        let artifact_ref = write_output_artifact_for_rollout(Some(&rollout_path), &raw_output)
            .await
            .expect("write artifact")
            .expect("large output should produce artifact ref");
        let sha = artifact_ref
            .strip_prefix("output-ref://sha256/")
            .expect("artifact ref prefix");
        let artifact_path = temp
            .path()
            .join("rollout-test-artifacts")
            .join("output-refs")
            .join(format!("{sha}.stdout"));

        assert_eq!(
            tokio::fs::read(&artifact_path)
                .await
                .expect("read artifact"),
            raw_output
        );
    }
}
