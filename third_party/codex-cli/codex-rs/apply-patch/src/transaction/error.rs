use std::fmt;
use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq)]
pub struct PatchCommitError {
    pub cause: String,
    pub committed_paths: Vec<PathBuf>,
    pub pending_paths: Vec<PathBuf>,
    pub rollback_restored_paths: Vec<PathBuf>,
    pub rollback_failed_paths: Vec<PathBuf>,
}

impl fmt::Display for PatchCommitError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "patch commit failed: {}; committed_paths={}; pending_paths={}; rollback_status={}; rollback_restored_paths={}; rollback_failed_paths={}",
            self.cause,
            format_paths(&self.committed_paths),
            format_paths(&self.pending_paths),
            if self.rollback_failed_paths.is_empty() {
                "best_effort_restored"
            } else {
                "best_effort_partial"
            },
            format_paths(&self.rollback_restored_paths),
            format_paths(&self.rollback_failed_paths),
        )
    }
}

impl std::error::Error for PatchCommitError {}

fn format_paths(paths: &[PathBuf]) -> String {
    let values = paths
        .iter()
        .map(|path| path.display().to_string())
        .collect::<Vec<_>>();
    format!("[{}]", values.join(","))
}
