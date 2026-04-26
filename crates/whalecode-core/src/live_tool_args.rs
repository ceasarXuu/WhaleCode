use serde::Deserialize;
use serde_json::Value;
use whalecode_tools::ToolRequest;

#[derive(Debug, Deserialize)]
struct ListFilesArgs {
    max_entries: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ReadFileArgs {
    path: String,
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct SearchTextArgs {
    query: String,
    max_matches: Option<usize>,
    max_bytes: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct EditFileArgs {
    pub(crate) path: String,
    #[serde(alias = "old")]
    pub(crate) old_string: String,
    #[serde(alias = "new")]
    pub(crate) new_string: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct WriteFileArgs {
    pub(crate) path: String,
    pub(crate) content: String,
    pub(crate) create_parent_dirs: Option<bool>,
}

pub(crate) fn parse_list_files(arguments: &str) -> Result<ToolRequest, String> {
    let args = parse_args::<ListFilesArgs>(arguments)?;
    Ok(ToolRequest::ListFiles {
        max_entries: args.max_entries.unwrap_or(120).clamp(1, 500),
    })
}

pub(crate) fn parse_read_file(arguments: &str) -> Result<ToolRequest, String> {
    let args = parse_args::<ReadFileArgs>(arguments)?;
    Ok(ToolRequest::ReadFile {
        path: args.path,
        max_bytes: Some(args.max_bytes.unwrap_or(32 * 1024).clamp(1024, 128 * 1024)),
    })
}

pub(crate) fn parse_search_text(arguments: &str) -> Result<ToolRequest, String> {
    let args = parse_args::<SearchTextArgs>(arguments)?;
    Ok(ToolRequest::SearchText {
        query: args.query,
        max_matches: args.max_matches.unwrap_or(50).clamp(1, 200),
        max_bytes: Some(args.max_bytes.unwrap_or(32 * 1024).clamp(1024, 128 * 1024)),
    })
}

pub(crate) fn parse_edit_file(arguments: &str) -> Result<EditFileArgs, String> {
    parse_args(arguments).map_err(|error| format!("invalid edit_file arguments: {error}"))
}

pub(crate) fn parse_write_file(arguments: &str) -> Result<WriteFileArgs, String> {
    parse_args(arguments).map_err(|error| format!("invalid write_file arguments: {error}"))
}

pub(crate) fn argument_path(arguments: &str) -> String {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| value.get("path").and_then(Value::as_str).map(str::to_owned))
        .unwrap_or_default()
}

pub(crate) fn argument_query(arguments: &str) -> String {
    serde_json::from_str::<Value>(arguments)
        .ok()
        .and_then(|value| {
            value
                .get("query")
                .and_then(Value::as_str)
                .map(str::to_owned)
        })
        .unwrap_or_default()
}

pub(crate) fn tool_input_summary(tool_name: &str, arguments: &str) -> Option<String> {
    let value = serde_json::from_str::<Value>(arguments).ok()?;
    let summary = match tool_name {
        "list_files" => value
            .get("max_entries")
            .and_then(Value::as_u64)
            .map(|max| format!("max_entries={max}"))
            .unwrap_or_else(|| "workspace".to_owned()),
        "read_file" | "edit_file" | "write_file" => value
            .get("path")
            .and_then(Value::as_str)
            .map(|path| path.to_owned())
            .unwrap_or_else(|| "(missing path)".to_owned()),
        "search_text" => value
            .get("query")
            .and_then(Value::as_str)
            .map(|query| format!("query={query:?}"))
            .unwrap_or_else(|| "(missing query)".to_owned()),
        "run_command" => summarize_command(&value),
        _ => return None,
    };
    Some(truncate_summary(&summary, 160))
}

fn parse_args<T: for<'de> Deserialize<'de>>(arguments: &str) -> Result<T, String> {
    serde_json::from_str(arguments).map_err(|error| error.to_string())
}

fn summarize_command(value: &Value) -> String {
    let command = value
        .get("command")
        .and_then(Value::as_str)
        .unwrap_or("(missing command)");
    let mut parts = vec![command.to_owned()];
    if let Some(args) = value.get("args").and_then(Value::as_array) {
        parts.extend(args.iter().filter_map(Value::as_str).map(str::to_owned));
    }
    parts.join(" ")
}

fn truncate_summary(value: &str, max_len: usize) -> String {
    if value.len() <= max_len {
        return value.to_owned();
    }
    let mut boundary = max_len;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    format!("{}...", &value[..boundary])
}
