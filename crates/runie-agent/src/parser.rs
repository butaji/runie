#[derive(Debug, serde::Deserialize)]
struct ToolCall {
    name: String,
    arguments: serde_json::Map<String, serde_json::Value>,
}

fn arg_str(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> String {
    args.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn arg_opt_str(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
}

fn arg_bool(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> bool {
    args.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
}

fn arg_usize(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> usize {
    args.get(key).and_then(|v| v.as_u64()).unwrap_or(0) as usize
}

fn arg_opt_usize(args: &serde_json::Map<String, serde_json::Value>, key: &str) -> Option<usize> {
    let v = args.get(key)?;
    v.as_u64().map(|n| n as usize)
}

fn parse_legacy_tool(payload: &str) -> Option<crate::Tool> {
    let parts: Vec<&str> = payload.splitn(3, ':').collect();
    if parts.len() < 2 {
        return None;
    }
    let tool_name = parts[0];
    let arg1 = parts.get(1).unwrap_or(&"");
    let arg2 = parts.get(2).unwrap_or(&"");
    Some(match tool_name {
        "read_file" => crate::Tool::ReadFile {
            path: arg1.to_string(),
            offset: None,
            limit: None,
        },
        "list_dir" => crate::Tool::ListDir {
            path: arg1.to_string(),
        },
        "write_file" => crate::Tool::WriteFile {
            path: arg1.to_string(),
            content: arg2.to_string(),
        },
        "bash" => crate::Tool::Bash {
            command: arg1.to_string(),
        },
        _ => return None,
    })
}

fn parse_structured_tool(line: &str) -> Option<crate::Tool> {
    let call: ToolCall = serde_json::from_str(line).ok()?;
    let args = &call.arguments;
    Some(match call.name.as_str() {
        "read_file" => crate::Tool::ReadFile {
            path: arg_str(args, "path"),
            offset: arg_opt_usize(args, "offset"),
            limit: arg_opt_usize(args, "limit"),
        },
        "list_dir" => crate::Tool::ListDir {
            path: arg_str(args, "path"),
        },
        "write_file" => crate::Tool::WriteFile {
            path: arg_str(args, "path"),
            content: arg_str(args, "content"),
        },
        "edit_file" => crate::Tool::EditFile {
            path: arg_str(args, "path"),
            search: arg_str(args, "search"),
            replace: arg_str(args, "replace"),
        },
        "bash" => crate::Tool::Bash {
            command: arg_str(args, "command"),
        },
        "grep" => crate::Tool::Grep {
            pattern: arg_str(args, "pattern"),
            path: arg_str(args, "path"),
            glob: arg_opt_str(args, "glob"),
            ignore_case: arg_bool(args, "ignore_case"),
            literal: arg_bool(args, "literal"),
            context: arg_usize(args, "context"),
            limit: arg_usize(args, "limit").max(1),
        },
        "find" => crate::Tool::Find {
            pattern: arg_str(args, "pattern"),
            path: arg_str(args, "path"),
            limit: arg_usize(args, "limit").max(1),
        },
        "fetch_docs" => crate::Tool::FetchDocs {
            library: arg_str(args, "library"),
        },
        _ => return None,
    })
}

pub fn parse_tool_calls(text: &str) -> Vec<crate::Tool> {
    let mut tools = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let tool = if line.starts_with("TOOL:") {
            parse_legacy_tool(line.strip_prefix("TOOL:").unwrap_or(""))
        } else if line.starts_with('{') {
            parse_structured_tool(line)
        } else {
            None
        };
        if let Some(t) = tool {
            tools.push(t);
        }
    }
    tools
}

pub fn has_tool_calls(text: &str) -> bool {
    !parse_tool_calls(text).is_empty()
}
