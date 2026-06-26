use serde::{Deserialize, Serialize};
use similar::TextDiff;

use super::{HarnessSkill, ToolCallCtx, ToolCallPhase, ToolCallResult};

/// Configuration for the hashline edit skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HashlineEditConfig {
    /// Whether hashline editing is enabled. Defaults to true.
    #[serde(default = "super::default_true")]
    pub enabled: bool,
    /// Number of characters to use for the hash (4-8 recommended).
    #[serde(default = "default_hash_length")]
    pub hash_length: usize,
}

impl Default for HashlineEditConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            hash_length: 6,
        }
    }
}

fn default_hash_length() -> usize {
    6
}

/// Hashline edit skill: line-addressed edits with content hashes.
pub struct HashlineEditSkill {
    config: HashlineEditConfig,
}

impl HashlineEditSkill {
    pub fn new(config: HashlineEditConfig) -> Self {
        Self { config }
    }

    /// Compute a short hash using FNV-1a.
    pub(crate) fn compute_hash(content: &str, length: usize) -> String {
        let trimmed = content.trim_end();
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in trimmed.bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x100000001b3);
        }
        format!("{:x}", hash)[..length.min(16)].to_string()
    }

    /// Get the hashline schema for edit_file tool.
    pub fn hashline_schema() -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "path": {"type": "string", "description": "File to edit"},
                "edits": {
                    "type": "array",
                    "description": "Line edits to apply",
                    "items": {
                        "type": "object",
                        "properties": {
                            "line": {"type": "integer", "description": "Line number (1-indexed)"},
                            "hash": {"type": "string", "description": "Expected hash of current line content"},
                            "content": {"type": "string", "description": "New content (empty to delete)"}
                        },
                        "required": ["line", "hash", "content"]
                    }
                }
            },
            "required": ["path", "edits"]
        })
    }

    /// Validate that the hashes in the edit request match the current file content.
    pub fn validate_hashes(path: &std::path::Path, edits: &[HashlineEdit]) -> Result<(), String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Error reading {}: {}", path.display(), e))?;
        let lines: Vec<&str> = content.lines().collect();
        for edit in edits {
            let idx = edit.line.saturating_sub(1);
            if idx >= lines.len() {
                return Err(format!("Line {} out of bounds", edit.line));
            }
            let hash = Self::compute_hash(lines[idx], 6);
            if hash != edit.hash {
                return Err(format!(
                    "Hash mismatch on line {}: expected {:6}, got {:6}",
                    edit.line, hash, edit.hash
                ));
            }
        }
        Ok(())
    }

    /// Apply hashline edits to a file.
    pub fn apply_edits(path: &std::path::Path, edits: &[HashlineEdit]) -> Result<String, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Error reading {}: {}", path.display(), e))?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_owned()).collect();
        // Apply from bottom to top to avoid line number shifting
        let mut sorted = edits.to_vec();
        sorted.sort_by_key(|b| std::cmp::Reverse(b.line));
        for edit in sorted {
            let idx = edit.line.saturating_sub(1);
            if idx >= lines.len() {
                return Err(format!("Line {} out of bounds", edit.line));
            }
            if edit.content.is_empty() {
                lines.remove(idx);
            } else {
                lines[idx] = edit.content.clone();
            }
        }

        let new_content = lines.join("\n");
        std::fs::write(path, &new_content)
            .map_err(|e| format!("Error writing {}: {}", path.display(), e))?;

        Ok(new_content)
    }
}

/// A single hashline edit operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HashlineEdit {
    /// Line number (1-indexed).
    pub line: usize,
    /// Expected hash of the current line content.
    pub hash: String,
    /// New content for the line (empty to delete).
    pub content: String,
}

fn format_diff(old: &str, new: &str) -> String {
    let diff = TextDiff::from_lines(old, new);
    let mut out = String::from("Applied hashline edits. Diff:\n");
    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            similar::ChangeTag::Delete => '-',
            similar::ChangeTag::Insert => '+',
            similar::ChangeTag::Equal => ' ',
        };
        out.push(sign);
        out.push_str(change.value());
    }
    out
}

fn try_apply_hashline(ctx: &ToolCallCtx) -> Result<ToolCallResult, String> {
    let edits = ctx.tool_input.get("edits").cloned().unwrap_or_default();
    let edits: Vec<HashlineEdit> = serde_json::from_value(edits)
        .map_err(|e| format!("Invalid hashline edit format: {}", e))?;
    let path = ctx
        .tool_input
        .get("path")
        .and_then(|p| p.as_str())
        .ok_or_else(|| "path is required for hashline edit".to_owned())?;
    let path = std::path::PathBuf::from(path);

    HashlineEditSkill::validate_hashes(&path, &edits)?;
    let old_content = std::fs::read_to_string(&path)
        .map_err(|e| format!("Error reading {}: {}", path.display(), e))?;
    let new_content = HashlineEditSkill::apply_edits(&path, &edits)?;
    Ok(ToolCallResult::SkipWithOutput(format_diff(
        &old_content,
        &new_content,
    )))
}

impl HarnessSkill for HashlineEditSkill {
    fn name(&self) -> &str {
        "hashline_edit"
    }

    fn on_tool_call(&self, ctx: &ToolCallCtx) -> ToolCallResult {
        if ctx.tool_name != "edit_file" || !self.config.enabled {
            return ToolCallResult::Continue;
        }
        if ctx.phase != ToolCallPhase::Before {
            return ToolCallResult::Continue;
        }
        if ctx.tool_input.get("edits").is_none() {
            return ToolCallResult::Continue;
        }

        match tokio::task::block_in_place(|| try_apply_hashline(ctx)) {
            Ok(result) => result,
            Err(e) => ToolCallResult::Abort(format!("Hashline edit failed: {}", e)),
        }
    }
}
