use std::{
    collections::BTreeMap,
    fs,
    path::{Component, Path, PathBuf},
};

use ignore::{DirEntry, WalkBuilder};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use whalecode_protocol::{ToolExecutionMode, ToolSpec};

const DEFAULT_MAX_BYTES: usize = 16 * 1024;
const TRUNCATION_NOTICE: &str = "\n...[truncated]...\n";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ToolResultEnvelope {
    pub content: String,
    pub truncated: bool,
    pub original_len: usize,
    pub metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolRequest {
    ListFiles {
        max_entries: usize,
    },
    ReadFile {
        path: String,
        max_bytes: Option<usize>,
    },
    SearchText {
        query: String,
        max_matches: usize,
        max_bytes: Option<usize>,
    },
}

#[derive(Debug, Error)]
pub enum ToolError {
    #[error("workspace root does not exist: {0}")]
    MissingWorkspace(String),
    #[error("path is outside workspace: {0}")]
    OutsideWorkspace(String),
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to list {path}: {source}")]
    List {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to walk workspace: {0}")]
    Walk(String),
    #[error("file is not valid UTF-8: {0}")]
    NonUtf8(PathBuf),
    #[error("path is intentionally hidden from tools: {0}")]
    HiddenPath(String),
}

#[derive(Debug, Clone)]
pub struct ToolRuntime {
    workspace_root: PathBuf,
    canonical_root: PathBuf,
}

impl ToolRuntime {
    pub fn new(workspace_root: impl AsRef<Path>) -> Result<Self, ToolError> {
        let workspace_root = workspace_root.as_ref().to_path_buf();
        let canonical_root = workspace_root
            .canonicalize()
            .map_err(|_| ToolError::MissingWorkspace(workspace_root.display().to_string()))?;
        Ok(Self {
            workspace_root,
            canonical_root,
        })
    }

    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    pub fn execute(&self, request: ToolRequest) -> Result<ToolResultEnvelope, ToolError> {
        match request {
            ToolRequest::ListFiles { max_entries } => self.list_files(max_entries),
            ToolRequest::ReadFile { path, max_bytes } => self.read_file(&path, max_bytes),
            ToolRequest::SearchText {
                query,
                max_matches,
                max_bytes,
            } => self.search_text(&query, max_matches, max_bytes),
        }
    }

    pub fn list_files(&self, max_entries: usize) -> Result<ToolResultEnvelope, ToolError> {
        let files = self.list_file_paths(max_entries)?;
        let original_len = files.len();
        let content = files
            .iter()
            .map(|path| self.relative_display(path))
            .collect::<Vec<_>>()
            .join("\n");
        Ok(ToolResultEnvelope {
            content,
            truncated: original_len >= max_entries,
            original_len,
            metadata: BTreeMap::from([
                ("tool".to_owned(), "list_files".to_owned()),
                (
                    "workspace".to_owned(),
                    self.workspace_root.display().to_string(),
                ),
            ]),
        })
    }

    pub fn read_file(
        &self,
        path: &str,
        max_bytes: Option<usize>,
    ) -> Result<ToolResultEnvelope, ToolError> {
        let path = self.resolve_existing(path)?;
        let content = fs::read_to_string(&path).map_err(|source| {
            if source.kind() == std::io::ErrorKind::InvalidData {
                ToolError::NonUtf8(path.clone())
            } else {
                ToolError::Read {
                    path: path.clone(),
                    source,
                }
            }
        })?;
        let relative_path = self.relative_display(&path);
        let truncated = truncate_text(&content, max_bytes.unwrap_or(DEFAULT_MAX_BYTES));
        Ok(ToolResultEnvelope {
            content: truncated.content,
            truncated: truncated.truncated,
            original_len: truncated.original_len,
            metadata: BTreeMap::from([
                ("tool".to_owned(), "read_file".to_owned()),
                ("path".to_owned(), relative_path),
            ]),
        })
    }

    pub fn search_text(
        &self,
        query: &str,
        max_matches: usize,
        max_bytes: Option<usize>,
    ) -> Result<ToolResultEnvelope, ToolError> {
        let files = self.list_file_paths(max_matches.saturating_mul(20).max(32))?;
        let mut matches = Vec::new();
        for path in files {
            if matches.len() >= max_matches {
                break;
            }
            let Ok(content) = fs::read_to_string(&path) else {
                continue;
            };
            for (line_index, line) in content.lines().enumerate() {
                if line.contains(query) {
                    matches.push(format!(
                        "{}:{}:{}",
                        self.relative_display(&path),
                        line_index + 1,
                        line.trim()
                    ));
                    if matches.len() >= max_matches {
                        break;
                    }
                }
            }
        }

        let content = matches.join("\n");
        let truncated = truncate_text(&content, max_bytes.unwrap_or(DEFAULT_MAX_BYTES));
        Ok(ToolResultEnvelope {
            content: truncated.content,
            truncated: truncated.truncated || matches.len() >= max_matches,
            original_len: truncated.original_len,
            metadata: BTreeMap::from([
                ("tool".to_owned(), "search_text".to_owned()),
                ("query".to_owned(), query.to_owned()),
            ]),
        })
    }

    fn list_file_paths(&self, max_entries: usize) -> Result<Vec<PathBuf>, ToolError> {
        let mut files = Vec::new();
        let walker = WalkBuilder::new(&self.canonical_root)
            .hidden(false)
            .git_ignore(true)
            .git_exclude(true)
            .parents(true)
            .filter_entry(|entry| !entry_is_hidden_from_tools(entry))
            .build();
        for entry in walker {
            if files.len() >= max_entries {
                break;
            }
            let entry = entry.map_err(|source| ToolError::Walk(source.to_string()))?;
            if entry.depth() == 0 {
                continue;
            }
            if entry
                .file_type()
                .is_some_and(|file_type| file_type.is_file())
            {
                files.push(entry.path().to_path_buf());
            }
        }
        files.sort();
        Ok(files)
    }

    fn resolve_existing(&self, path: &str) -> Result<PathBuf, ToolError> {
        let candidate = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            self.canonical_root.join(path)
        };
        if candidate
            .components()
            .any(|part| matches!(part, Component::ParentDir))
        {
            return Err(ToolError::OutsideWorkspace(path.to_owned()));
        }
        if candidate
            .components()
            .any(|part| part.as_os_str().to_str().is_some_and(should_skip_name))
        {
            return Err(ToolError::HiddenPath(path.to_owned()));
        }
        let canonical = candidate.canonicalize().map_err(|source| ToolError::Read {
            path: candidate.clone(),
            source,
        })?;
        if !canonical.starts_with(&self.canonical_root) {
            return Err(ToolError::OutsideWorkspace(path.to_owned()));
        }
        Ok(canonical)
    }

    fn relative_display(&self, path: &Path) -> String {
        path.strip_prefix(&self.canonical_root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/")
    }
}

pub fn builtin_tool_specs() -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "read_file".to_owned(),
            description: "Read a UTF-8 file from the workspace".to_owned(),
            execution_mode: ToolExecutionMode::ReadOnly,
        },
        ToolSpec {
            name: "search_text".to_owned(),
            description: "Search workspace text".to_owned(),
            execution_mode: ToolExecutionMode::ReadOnly,
        },
        ToolSpec {
            name: "edit_file".to_owned(),
            description: "Edit a file through patch-safe workspace logic".to_owned(),
            execution_mode: ToolExecutionMode::Write,
        },
    ]
}

struct TruncatedText {
    content: String,
    truncated: bool,
    original_len: usize,
}

fn truncate_text(content: &str, max_bytes: usize) -> TruncatedText {
    let original_len = content.len();
    if original_len <= max_bytes {
        return TruncatedText {
            content: content.to_owned(),
            truncated: false,
            original_len,
        };
    }
    let notice_len = TRUNCATION_NOTICE.len();
    let budget = max_bytes.saturating_sub(notice_len).max(2);
    let head_len = previous_char_boundary(content, budget / 2);
    let tail_start = next_char_boundary(content, original_len.saturating_sub(budget - head_len));
    TruncatedText {
        content: format!(
            "{}{}{}",
            &content[..head_len],
            TRUNCATION_NOTICE,
            &content[tail_start..]
        ),
        truncated: true,
        original_len,
    }
}

fn should_skip_name(name: &str) -> bool {
    matches!(
        name,
        ".git"
            | ".whale"
            | ".claude"
            | ".codex"
            | ".opencode"
            | ".env"
            | ".env.local"
            | "target"
            | "tmp"
            | ".DS_Store"
    )
}

fn entry_is_hidden_from_tools(entry: &DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(should_skip_name)
}

fn previous_char_boundary(value: &str, mut index: usize) -> usize {
    while index > 0 && !value.is_char_boundary(index) {
        index -= 1;
    }
    index
}

fn next_char_boundary(value: &str, mut index: usize) -> usize {
    while index < value.len() && !value.is_char_boundary(index) {
        index += 1;
    }
    index
}
