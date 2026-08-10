//! Bounded, read-only workspace tools.

use super::path_policy::{validate, PathOperation};
use crate::types::{AgentTool, AgentToolResult, ToolResultContent};
use std::path::Path;
use tokio::io::AsyncReadExt;

pub const READ_MAX_LINES: usize = 1_000;
pub const READ_MAX_BYTES: usize = 100 * 1024;

macro_rules! workspace_types { ($($name:ident),+ $(,)?) => { $(#[derive(Default)] pub struct $name;)+ }; }
workspace_types!(
    ReadFileTool,
    WriteFileTool,
    EditFileTool,
    GrepTool,
    GlobTool,
    BashTool
);

macro_rules! workspace_tool {
    ($name:literal, $label:literal, $description:literal, $schema:expr) => {
        fn name(&self) -> &str {
            $name
        }
        fn label(&self) -> &str {
            $label
        }
        fn description(&self) -> &str {
            $description
        }
        fn parameters(&self) -> Option<serde_json::Value> {
            Some($schema)
        }
    };
}

#[async_trait::async_trait]
impl AgentTool for ReadFileTool {
    fn name(&self) -> &str {
        "read"
    }
    fn label(&self) -> &str {
        "Read file"
    }
    fn description(&self) -> &str {
        "Read a text file from the working directory with bounded output."
    }
    fn parameters(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string" },
                "line_offset": { "type": "integer", "minimum": 1 },
                "n_lines": { "type": "integer", "minimum": 1 }
            },
            "required": ["path"]
        }))
    }
    async fn execute(
        &self,
        _tool_call_id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _on_update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let path = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "read requires a string `path` argument".to_owned())?;
        let path = validate(path, PathOperation::Read)?;
        let content = tokio::fs::read_to_string(Path::new(&path))
            .await
            .map_err(|error| format!("read {path:?}: {error}"))?;
        let start = args
            .get("line_offset")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1)
            .saturating_sub(1) as usize;
        let requested = args
            .get("n_lines")
            .and_then(serde_json::Value::as_u64)
            .map(|n| n as usize)
            .unwrap_or(READ_MAX_LINES)
            .min(READ_MAX_LINES);
        let output = bounded_lines(&content, start, requested);
        text_result_with_details(
            output,
            serde_json::json!({"path": path, "line_offset": start + 1, "line_count": requested}),
        )
    }
}

#[async_trait::async_trait]
impl AgentTool for WriteFileTool {
    workspace_tool!(
        "write",
        "Write file",
        "Create or replace a text file.",
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"content":{"type":"string"}},"required":["path","content"]})
    );
    fn resource_key(&self, args: &serde_json::Value) -> Option<String> {
        args.get("path")
            .and_then(serde_json::Value::as_str)
            .map(|path| format!("workspace:{path}"))
    }
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let path = validate(
            required_string(&args, "path", "write")?,
            PathOperation::Write,
        )?;
        let content = required_string(&args, "content", "write")?;
        tokio::fs::write(&path, content)
            .await
            .map_err(|error| error.to_string())?;
        text_result_with_details("written".into(), serde_json::json!({"path": path}))
    }
}

#[async_trait::async_trait]
impl AgentTool for EditFileTool {
    workspace_tool!(
        "edit",
        "Edit file",
        "Replace one exact text occurrence in a file.",
        serde_json::json!({"type":"object","properties":{"path":{"type":"string"},"old_string":{"type":"string"},"new_string":{"type":"string"},"replace_all":{"type":"boolean"}},"required":["path","old_string","new_string"]})
    );
    fn resource_key(&self, args: &serde_json::Value) -> Option<String> {
        args.get("path")
            .and_then(serde_json::Value::as_str)
            .map(|path| format!("workspace:{path}"))
    }
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let path = validate(
            required_string(&args, "path", "edit")?,
            PathOperation::Write,
        )?;
        let old = required_string(&args, "old_string", "edit")?;
        let new = required_string(&args, "new_string", "edit")?;
        apply_edit(
            path.to_str().unwrap_or_default(),
            old,
            new,
            replace_all(&args),
        )
        .await?;
        text_result_with_details(
            "edited".into(),
            serde_json::json!({"path": path, "replace_all": replace_all(&args)}),
        )
    }
}

fn replace_all(args: &serde_json::Value) -> bool {
    args.get("replace_all")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}

async fn apply_edit(path: &str, old: &str, new: &str, all: bool) -> Result<(), String> {
    if old == new {
        return Err("old_string and new_string must differ".into());
    }
    let source = tokio::fs::read_to_string(path)
        .await
        .map_err(|error| error.to_string())?;
    let count = source.matches(old).count();
    if count == 0 {
        return Err("old_string was not found".into());
    }
    if count > 1 && !all {
        return Err(format!("old_string matched {count} times; set replace_all"));
    }
    let output = if all {
        source.replace(old, new)
    } else {
        source.replacen(old, new, 1)
    };
    tokio::fs::write(path, output)
        .await
        .map_err(|error| error.to_string())
}

#[async_trait::async_trait]
impl AgentTool for GrepTool {
    workspace_tool!(
        "grep",
        "Search files",
        "Search text files recursively.",
        serde_json::json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]})
    );
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let pattern = required_string(&args, "pattern", "grep")?;
        let root = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");
        let root = validate(root, PathOperation::Search)?;
        let matches = grep_matches(root.to_str().unwrap_or("."), pattern);
        text_result_with_details(
            matches.join("\n"),
            serde_json::json!({"pattern": pattern, "match_count": matches.len()}),
        )
    }
}

fn grep_matches(root: &str, pattern: &str) -> Vec<String> {
    let mut matches = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        if matches.len() >= 250 {
            break;
        }
        let Ok(content) = std::fs::read_to_string(entry.path()) else {
            continue;
        };
        for (line, text) in content
            .lines()
            .enumerate()
            .filter(|(_, text)| text.contains(pattern))
        {
            matches.push(format!("{}:{}:{}", entry.path().display(), line + 1, text));
        }
    }
    matches
}

#[async_trait::async_trait]
impl AgentTool for GlobTool {
    workspace_tool!(
        "glob",
        "Find files",
        "Find files recursively by a simple glob pattern.",
        serde_json::json!({"type":"object","properties":{"pattern":{"type":"string"},"path":{"type":"string"}},"required":["pattern"]})
    );
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        _signal: Option<tokio_util::sync::CancellationToken>,
        _update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let pattern = required_string(&args, "pattern", "glob")?;
        let root = args
            .get("path")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(".");
        let root = validate(root, PathOperation::Search)?;
        let mut paths: Vec<_> = walkdir::WalkDir::new(root)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_type().is_file())
            .filter(|entry| glob_matches(&entry.file_name().to_string_lossy(), pattern))
            .map(|entry| entry.path().display().to_string())
            .collect();
        paths.sort_unstable();
        let paths: Vec<_> = paths.into_iter().take(100).collect();
        text_result_with_details(
            paths.join("\n"),
            serde_json::json!({"pattern": pattern, "match_count": paths.len()}),
        )
    }
}

#[async_trait::async_trait]
impl AgentTool for BashTool {
    workspace_tool!(
        "bash",
        "Run command",
        "Run a shell command after approval.",
        serde_json::json!({"type":"object","properties":{"command":{"type":"string"},"timeout_ms":{"type":"integer","minimum":1}},"required":["command"]})
    );
    async fn execute(
        &self,
        _id: &str,
        args: serde_json::Value,
        signal: Option<tokio_util::sync::CancellationToken>,
        update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
    ) -> Result<AgentToolResult, String> {
        let command = required_string(&args, "command", "bash")?;
        let timeout = args
            .get("timeout_ms")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(120_000);
        let result = run_shell(command, timeout, signal, update).await?;
        text_result_with_details(
            result.text.clone(),
            serde_json::json!({
                "stdout": result.stdout,
                "stderr": result.stderr,
                "exit_code": result.exit_code,
                "timed_out": false,
                "cancelled": false,
            }),
        )
    }
}

struct ShellResult {
    text: String,
    stdout: String,
    stderr: String,
    exit_code: i32,
}

async fn run_shell(
    command: &str,
    timeout: u64,
    signal: Option<tokio_util::sync::CancellationToken>,
    update: Option<Box<dyn Fn(serde_json::Value) + Send + Sync>>,
) -> Result<ShellResult, String> {
    let mut child = spawn_shell(command)?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "stdout pipe unavailable".to_owned())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "stderr pipe unavailable".to_owned())?;
    let (out, err) = {
        let output = read_output(&mut stdout, &mut stderr, update.as_deref());
        tokio::pin!(output);
        tokio::select! {
            result = &mut output => result?,
            _ = cancellation(signal) => { let _ = child.kill().await; return Err("command cancelled".into()); }
            _ = tokio::time::sleep(std::time::Duration::from_millis(timeout)) => { let _ = child.kill().await; return Err("command timed out".into()); }
        }
    };
    let status = child.wait().await.map_err(|error| error.to_string())?;
    let mut text = String::from_utf8_lossy(&out).into_owned();
    text.push_str(&String::from_utf8_lossy(&err));
    if let Some(update) = update {
        update(serde_json::json!({"text": text, "complete": true}));
    }
    if !status.success() {
        return Err(format!("command exited {status}: {text}"));
    }
    Ok(ShellResult {
        text: text.trim_end().into(),
        stdout: String::from_utf8_lossy(&out).into_owned(),
        stderr: String::from_utf8_lossy(&err).into_owned(),
        exit_code: status.code().unwrap_or_default(),
    })
}

fn spawn_shell(command: &str) -> Result<tokio::process::Child, String> {
    tokio::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())
}

async fn read_output(
    stdout: &mut (impl tokio::io::AsyncRead + Unpin),
    stderr: &mut (impl tokio::io::AsyncRead + Unpin),
    update: Option<&(dyn Fn(serde_json::Value) + Send + Sync)>,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let (out, err) = tokio::join!(
        read_stream(stdout, update, "stdout"),
        read_stream(stderr, update, "stderr")
    );
    let (out, err) = (out?, err?);
    Ok((out, err))
}

async fn read_stream(
    stream: &mut (impl tokio::io::AsyncRead + Unpin),
    update: Option<&(dyn Fn(serde_json::Value) + Send + Sync)>,
    name: &str,
) -> Result<Vec<u8>, String> {
    let mut all = Vec::new();
    let mut chunk = [0_u8; 4096];
    loop {
        let read = stream
            .read(&mut chunk)
            .await
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        all.extend_from_slice(&chunk[..read]);
        if let Some(update) = update {
            update(
                serde_json::json!({"text": String::from_utf8_lossy(&chunk[..read]), "stream": name, "complete": false}),
            );
        }
    }
    Ok(all)
}

async fn cancellation(signal: Option<tokio_util::sync::CancellationToken>) {
    if let Some(signal) = signal {
        signal.cancelled().await
    } else {
        std::future::pending::<()>().await
    }
}

fn required_string<'a>(
    args: &'a serde_json::Value,
    key: &str,
    tool: &str,
) -> Result<&'a str, String> {
    args.get(key)
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| format!("{tool} requires string `{key}`"))
}

fn text_result_with_details(
    text: String,
    details: serde_json::Value,
) -> Result<AgentToolResult, String> {
    Ok(AgentToolResult {
        content: vec![ToolResultContent::Text { text }],
        details,
        ..AgentToolResult::default()
    })
}

fn glob_matches(name: &str, pattern: &str) -> bool {
    match pattern.strip_prefix("*") {
        Some(suffix) => name.ends_with(suffix),
        None => name == pattern,
    }
}

fn bounded_lines(content: &str, start: usize, requested: usize) -> String {
    let mut output = String::new();
    for line in content.lines().skip(start).take(requested) {
        let addition = if output.is_empty() {
            line.to_owned()
        } else {
            format!("\n{line}")
        };
        if output.len() + addition.len() > READ_MAX_BYTES {
            break;
        }
        output.push_str(&addition);
    }
    if content.lines().skip(start).count() > requested || output.len() >= READ_MAX_BYTES {
        output.push_str("\n[output truncated]");
    }
    output
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
