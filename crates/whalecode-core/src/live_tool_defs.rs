use serde_json::{json, Value};

pub(crate) fn live_tool_defs() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "list_files",
                "description": "List gitignore-aware workspace files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "max_entries": {"type": "integer", "minimum": 1, "maximum": 500}
                    }
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read a UTF-8 workspace file by relative path.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "max_bytes": {"type": "integer", "minimum": 1024, "maximum": 131072}
                    },
                    "required": ["path"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "search_text",
                "description": "Search for literal text in gitignore-aware workspace files.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "query": {"type": "string"},
                        "max_matches": {"type": "integer", "minimum": 1, "maximum": 200},
                        "max_bytes": {"type": "integer", "minimum": 1024, "maximum": 131072}
                    },
                    "required": ["query"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "edit_file",
                "description": "Patch one UTF-8 workspace file by replacing one exact old string with a new string. Requires --allow-write.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"},
                        "old_string": {"type": "string"},
                        "new_string": {"type": "string"}
                    },
                    "required": ["path", "old_string", "new_string"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "write_file",
                "description": "Create or fully overwrite a UTF-8 workspace file. Use this for new files or full rewrites. Requires --allow-write.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string", "description": "Path relative to the workspace root."},
                        "content": {"type": "string"},
                        "create_parent_dirs": {"type": "boolean", "description": "Create missing parent directories. Defaults to true."}
                    },
                    "required": ["path", "content"]
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "run_command",
                "description": "Run a bounded verification command in the workspace. Requires --allow-command. Arguments are passed without a shell.",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "command": {"type": "string"},
                        "args": {
                            "type": "array",
                            "items": {"type": "string"}
                        },
                        "timeout_secs": {"type": "integer", "minimum": 1, "maximum": 300}
                    },
                    "required": ["command"]
                }
            }
        }),
    ]
}
