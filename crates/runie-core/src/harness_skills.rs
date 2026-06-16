//! Harness Skill Framework — Event-bus interceptors that wrap the agent turn.
//!
//! Skills are default-on, configurable, and togglable harness behaviors.
//! They register hooks: `on_turn_start`, `on_tool_call`, `on_turn_end`.
//!
//! See `docs/adr/0022-harness-middleware-plugins.md` for motivation.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A harness skill that intercepts agent turn lifecycle events.
pub trait HarnessSkill: Send + Sync {
    /// Human-readable name for diagnostics.
    fn name(&self) -> &str;

    /// Called before the LLM call.
    fn on_turn_start(&self, _ctx: &TurnStartCtx) -> TurnStartResult {
        TurnStartResult::Continue
    }

    /// Called before and after each tool execution.
    fn on_tool_call(&self, _ctx: &ToolCallCtx) -> ToolCallResult {
        ToolCallResult::Continue
    }

    /// Called after the model declares completion.
    fn on_turn_end(&self, _ctx: &TurnEndCtx) -> TurnEndResult {
        TurnEndResult::Continue
    }
}

// ---------------------------------------------------------------------------
// Hook input/output types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct TurnStartCtx {
    /// The user's message content.
    pub message: String,
    /// System prompt being used.
    pub system_prompt: String,
    /// Configured skills context.
    pub skills_context: String,
}

#[derive(Debug, Clone, Default)]
pub enum TurnStartResult {
    /// Continue with the turn as normal.
    #[default]
    Continue,
    /// Skip the LLM call, use this message instead.
    SkipWithMessage(String),
    /// Abort the turn with an error message.
    Abort(String),
}

#[derive(Debug, Clone)]
pub struct ToolCallCtx {
    /// Tool name (e.g., "bash", "read_file").
    pub tool_name: String,
    /// Tool input arguments as JSON.
    pub tool_input: serde_json::Value,
    /// Phase: before or after execution.
    pub phase: ToolCallPhase,
    /// Tool output (available in `After` phase).
    pub tool_output: Option<String>,
    /// Whether the tool call succeeded.
    pub success: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolCallPhase {
    Before,
    After,
}

#[derive(Debug, Clone, Default)]
pub enum ToolCallResult {
    /// Continue with tool execution (or continue after).
    #[default]
    Continue,
    /// Skip this tool call, return mock output.
    SkipWithOutput(String),
    /// Abort with error.
    Abort(String),
}

#[derive(Debug, Clone)]
pub struct TurnEndCtx {
    /// The final assistant message.
    pub assistant_message: String,
    /// Number of tool calls made.
    pub tool_call_count: usize,
    /// Whether the turn completed successfully.
    pub success: bool,
}

#[derive(Debug, Clone, Default)]
pub enum TurnEndResult {
    /// Turn is complete.
    #[default]
    Continue,
    /// Request another LLM call (e.g., verification loop).
    RequestAnotherPass,
    /// Abort with error.
    Abort(String),
}

// ---------------------------------------------------------------------------
// Skill configuration
// ---------------------------------------------------------------------------

/// Configuration for a single harness skill.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SkillConfig {
    /// Whether the skill is enabled. Defaults to true.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Skill-specific configuration. Free-form for flexibility.
    #[serde(default)]
    pub options: HashMap<String, serde_json::Value>,
}

fn default_true() -> bool {
    true
}

/// Configuration for the entire harness section.
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct HarnessConfig {
    /// Global harness settings.
    #[serde(default)]
    pub skills: HashMap<String, SkillConfig>,
}

// ---------------------------------------------------------------------------
// Skill registry
// ---------------------------------------------------------------------------

/// Registry that manages harness skills and dispatches hooks.
pub struct SkillRegistry {
    skills: Vec<Box<dyn HarnessSkill>>,
    config: HarnessConfig,
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl SkillRegistry {
    /// Create a new empty registry.
    pub fn new() -> Self {
        Self {
            skills: Vec::new(),
            config: HarnessConfig::default(),
        }
    }

    /// Register a skill.
    pub fn register(&mut self, skill: impl HarnessSkill + 'static) {
        self.skills.push(Box::new(skill));
    }

    /// Update configuration for skills.
    pub fn set_config(&mut self, config: HarnessConfig) {
        self.config = config;
    }

    /// Get the current configuration.
    pub fn config(&self) -> &HarnessConfig {
        &self.config
    }

    /// Get names of enabled skills.
    pub fn enabled_skills(&self) -> Vec<&str> {
        self.skills
            .iter()
            .filter(|s| self.is_enabled(s.name()))
            .map(|s| s.name())
            .collect()
    }

    /// Check if a skill is enabled in config.
    fn is_enabled(&self, name: &str) -> bool {
        self.config
            .skills
            .get(name)
            .map(|c| c.enabled)
            .unwrap_or(true) // Default to enabled
    }

    /// Dispatch `on_turn_start` to all enabled skills.
    pub fn on_turn_start(&self, ctx: &TurnStartCtx) -> TurnStartResult {
        let mut result = TurnStartResult::Continue;
        for skill in &self.skills {
            if !self.is_enabled(skill.name()) {
                continue;
            }
            let r = skill.on_turn_start(ctx);
            match &r {
                TurnStartResult::Continue => {}
                TurnStartResult::SkipWithMessage(_) => {
                    result = r;
                    break;
                }
                TurnStartResult::Abort(_) => {
                    result = r;
                    break;
                }
            }
        }
        result
    }

    /// Dispatch `on_tool_call` to all enabled skills.
    pub fn on_tool_call(&self, ctx: &ToolCallCtx) -> ToolCallResult {
        let mut result = ToolCallResult::Continue;
        for skill in &self.skills {
            if !self.is_enabled(skill.name()) {
                continue;
            }
            let r = skill.on_tool_call(ctx);
            match &r {
                ToolCallResult::Continue => {}
                ToolCallResult::SkipWithOutput(_) => {
                    result = r;
                    break;
                }
                ToolCallResult::Abort(_) => {
                    result = r;
                    break;
                }
            }
        }
        result
    }

    /// Dispatch `on_turn_end` to all enabled skills.
    pub fn on_turn_end(&self, ctx: &TurnEndCtx) -> TurnEndResult {
        let mut result = TurnEndResult::Continue;
        for skill in &self.skills {
            if !self.is_enabled(skill.name()) {
                continue;
            }
            let r = skill.on_turn_end(ctx);
            match &r {
                TurnEndResult::Continue => {}
                TurnEndResult::RequestAnotherPass => {
                    result = r;
                    break;
                }
                TurnEndResult::Abort(_) => {
                    result = r;
                    break;
                }
            }
        }
        result
    }
}

// ---------------------------------------------------------------------------
// Default skills (empty implementations for extensibility)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Startup Context Injector Skill
// ---------------------------------------------------------------------------

/// Configuration for the startup context skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartupContextConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_max_output")]
    pub max_output_bytes: usize,
    #[serde(default)]
    pub commands: Vec<String>,
}

impl Default for StartupContextConfig {
    fn default() -> Self {
        Self { enabled: true, max_output_bytes: 2048, commands: vec!["pwd".into(), "ls".into(), "git branch --show-current".into()] }
    }
}
fn default_max_output() -> usize { 2048 }

/// Startup context injector skill.
pub struct StartupContextSkill { config: StartupContextConfig, cache: std::sync::RwLock<Option<String>> }

impl StartupContextSkill {
    pub fn new(config: StartupContextConfig) -> Self { Self { config, cache: std::sync::RwLock::new(None) } }
    
    fn run_cmd(cmd: &str) -> String {
        let parts: Vec<&str> = cmd.split_whitespace().collect();
        if parts.is_empty() { return String::new(); }
        match std::process::Command::new(parts[0]).args(&parts[1..]).output() {
            Ok(o) => String::from_utf8_lossy(&o.stdout).trim().to_string(), Err(_) => String::new()
        }
    }
    
    fn discover(&self) -> String {
        let lines: Vec<String> = std::iter::once("=== Workspace Context ===".into())
            .chain(self.config.commands.iter().filter_map(|c| { let o = Self::run_cmd(c); if o.is_empty() { None } else { Some(format!("$ {}\n{}", c, o)) } }))
            .collect();
        let ctx = lines.join("\n");
        if ctx.len() > self.config.max_output_bytes { ctx[..self.config.max_output_bytes].to_string() } else { ctx }
    }
    
    pub fn get_context(&self) -> String {
        if let Ok(g) = self.cache.read() { if let Some(c) = g.as_ref() { return c.clone(); } }
        let ctx = self.discover();
        if let Ok(mut g) = self.cache.write() { *g = Some(ctx.clone()); }
        ctx
    }
    
    pub fn clear_cache(&self) { if let Ok(mut g) = self.cache.write() { *g = None; } }
}

impl HarnessSkill for StartupContextSkill {
    fn name(&self) -> &str { "startup_context" }
    fn on_turn_start(&self, ctx: &TurnStartCtx) -> TurnStartResult {
        if !self.config.enabled { return TurnStartResult::Continue; }
        if ctx.system_prompt.contains("=== Workspace Context ===") { return TurnStartResult::Continue; }
        let ctx_str = self.get_context();
        if ctx_str.is_empty() { return TurnStartResult::Continue; }
        TurnStartResult::SkipWithMessage(format!("{}\n\n{}", ctx_str, ctx.message))
    }
}

// ---------------------------------------------------------------------------
// Loop Detector Skill
// ---------------------------------------------------------------------------

/// Configuration for the loop detector skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoopDetectorConfig {
    /// Maximum repeats before triggering loop detection.
    #[serde(default = "default_max_repeats")]
    pub max_repeats: usize,
    /// Whether detection is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

impl Default for LoopDetectorConfig {
    fn default() -> Self {
        Self { max_repeats: 3, enabled: true }
    }
}

fn default_max_repeats() -> usize { 3 }

/// Loop detector skill.
pub struct LoopDetectorSkill {
    config: LoopDetectorConfig,
    recent_calls: std::sync::Mutex<Vec<(String, String, bool)>>,
}

impl LoopDetectorSkill {
    pub fn new(config: LoopDetectorConfig) -> Self {
        Self { config, recent_calls: std::sync::Mutex::new(Vec::new()) }
    }

    /// Record a tool call outcome.
    pub fn record_call(&self, tool_name: &str, input: &serde_json::Value, success: bool) {
        let target = input.get("path").or_else(|| input.get("command"))
            .and_then(|v| v.as_str()).unwrap_or("").to_string();
        let entry = (tool_name.to_string(), target, success);
        if let Ok(mut calls) = self.recent_calls.lock() {
            calls.push(entry);
            if calls.len() > 100 { calls.drain(0..50); }
        }
    }

    /// Reset state at turn start.
    pub fn reset(&self) {
        if let Ok(mut calls) = self.recent_calls.lock() { calls.clear(); }
    }

    /// Check for loop. Returns message if detected.
    pub fn check_loop(&self) -> Option<String> {
        if !self.config.enabled { return None; }
        let calls = self.recent_calls.lock().ok()?;
        let mut counts = std::collections::HashMap::new();
        for (tool, target, success) in calls.iter().rev() {
            if *success { break; }
            let key = format!("{}/{}", tool, target);
            *counts.entry(key).or_insert(0) += 1;
        }
        for (pattern, count) in counts {
            if count >= self.config.max_repeats {
                return Some(format!("Loop detected ({}x): {}. Try a different approach.", count, pattern));
            }
        }
        None
    }
}

impl HarnessSkill for LoopDetectorSkill {
    fn name(&self) -> &str { "loop_detector" }
}

// ---------------------------------------------------------------------------
// Tool Schema Enricher Skill
// ---------------------------------------------------------------------------

/// Configuration for the tool schema enricher skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSchemaEnricherConfig {
    /// Whether enrichment is enabled. Defaults to true.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Tools to skip enrichment for (empty = enrich all).
    #[serde(default)]
    pub skip_tools: Vec<String>,
}

impl Default for ToolSchemaEnricherConfig {
    fn default() -> Self {
        Self { enabled: true, skip_tools: Vec::new() }
    }
}

/// Tool schema enricher skill: adds examples to tool schemas.
pub struct ToolSchemaEnricherSkill { config: ToolSchemaEnricherConfig }

impl ToolSchemaEnricherSkill {
    pub fn new(config: ToolSchemaEnricherConfig) -> Self { Self { config } }

    /// Get example inputs for a tool.
    pub(crate) fn get_examples(tool_name: &str) -> Vec<serde_json::Value> {
        match tool_name {
            "bash" => vec![serde_json::json!({"command": "ls"}), serde_json::json!({"command": "cargo test"}), serde_json::json!({"command": "git status"})],
            "read_file" => vec![serde_json::json!({"path": "src/main.rs"}), serde_json::json!({"path": "README.md"})],
            "write_file" => vec![serde_json::json!({"path": "f.txt", "content": "hi"})],
            "edit_file" => vec![serde_json::json!({"path": "f.rs", "search": "a", "replace": "b"})],
            "list_dir" => vec![serde_json::json!({"path": "."})],
            "grep" => vec![serde_json::json!({"pattern": "TODO", "path": "."})],
            "find" => vec![serde_json::json!({"pattern": "*.rs", "path": "."})],
            "fetch_docs" => vec![serde_json::json!({"library": "serde"})],
            _ => Vec::new(),
        }
    }

    /// Check if a tool should be enriched (not in skip list).
    pub(crate) fn should_enrich(&self, tool_name: &str) -> bool {
        !self.config.skip_tools.contains(&tool_name.to_string())
    }

    /// Enrich a tool schema with examples.
    pub(crate) fn enrich_schema(&self, schema: &serde_json::Value) -> serde_json::Value {
        let name = schema.get("name").and_then(|v| v.as_str()).unwrap_or("");
        let examples = Self::get_examples(name);
        if examples.is_empty() || !self.should_enrich(name) { return schema.clone(); }
        let mut enriched = schema.clone();
        if let Some(obj) = enriched.get_mut("input_schema").and_then(|v| v.as_object_mut()) {
            obj.insert("examples".to_string(), serde_json::json!(examples));
        }
        enriched
    }

    /// Enrich a list of tool schemas with examples.
    pub fn enrich_schemas(&self, schemas: Vec<serde_json::Value>) -> Vec<serde_json::Value> {
        if !self.config.enabled { return schemas; }
        schemas.into_iter().map(|s| self.enrich_schema(&s)).collect()
    }
}

impl HarnessSkill for ToolSchemaEnricherSkill {
    fn name(&self) -> &str { "tool_schema_enricher" }
}

// ---------------------------------------------------------------------------
// Hashline Edit Skill
// ---------------------------------------------------------------------------

/// Configuration for the hashline edit skill.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct HashlineEditConfig {
    /// Whether hashline editing is enabled. Defaults to true.
    #[serde(default = "default_true")]
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

fn default_hash_length() -> usize { 6 }

/// Hashline edit skill: line-addressed edits with content hashes.
pub struct HashlineEditSkill { config: HashlineEditConfig }

impl HashlineEditSkill {
    pub fn new(config: HashlineEditConfig) -> Self { Self { config } }

    /// Compute a short hash using FNV-1a.
    pub(crate) fn compute_hash(content: &str, length: usize) -> String {
        let trimmed = content.trim_end();
        let mut hash: u64 = 0xcbf29ce484222325;
        for byte in trimmed.bytes() { hash ^= byte as u64; hash = hash.wrapping_mul(0x100000001b3); }
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
                        "properties": {"line": {"type": "integer", "description": "Line number (1-indexed)"}, "content": {"type": "string", "description": "New content (empty to delete)"}},
                        "required": ["line", "content"]
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
            if idx >= lines.len() { return Err(format!("Line {} out of bounds", edit.line)); }
            let hash = Self::compute_hash(lines[idx], 6);
            if hash != edit.hash {
                return Err(format!("Hash mismatch on line {}: expected {:6}, got {:6}", edit.line, hash, edit.hash));
            }
        }
        Ok(())
    }

    /// Apply hashline edits to a file.
    pub fn apply_edits(path: &std::path::Path, edits: &[HashlineEdit]) -> Result<String, String> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| format!("Error reading {}: {}", path.display(), e))?;
        let mut lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
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

impl HarnessSkill for HashlineEditSkill {
    fn name(&self) -> &str {
        "hashline_edit"
    }

    fn on_tool_call(&self, ctx: &ToolCallCtx) -> ToolCallResult {
        // Check if this is a hashline edit call and skill is enabled
        if ctx.tool_name != "edit_file" {
            return ToolCallResult::Continue;
        }
        
        if !self.config.enabled {
            return ToolCallResult::Continue;
        }
        
        // Check if this is a hashline format call (has "edits" field)
        if let Some(edits) = ctx.tool_input.get("edits") {
            // This is a hashline format call
            let edits: Vec<HashlineEdit> = match serde_json::from_value(edits.clone()) {
                Ok(e) => e,
                Err(e) => {
                    return ToolCallResult::Abort(format!("Invalid hashline edit format: {}", e));
                }
            };
            
            let path = match ctx.tool_input.get("path").and_then(|p| p.as_str()) {
                Some(p) => std::path::PathBuf::from(p),
                None => {
                    return ToolCallResult::Abort("path is required for hashline edit".to_string());
                }
            };
            
            // Validate hashes
            if let Err(e) = HashlineEditSkill::validate_hashes(&path, &edits) {
                return ToolCallResult::Abort(format!("Hashline validation failed: {}", e));
            }
        }
        
        ToolCallResult::Continue
    }
}

// ---------------------------------------------------------------------------
// Verification Loop Skill
// ---------------------------------------------------------------------------

/// Configuration for the verification loop skill.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VerificationConfig {
    /// Command to run for verification (e.g., "cargo test", "npm test").
    #[serde(default)]
    pub command: Option<String>,
    /// Maximum number of fix attempts after verification failure.
    #[serde(default = "default_max_fix_passes")]
    pub max_fix_passes: usize,
    /// Whether verification is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,
}

fn default_max_fix_passes() -> usize {
    3
}

/// Verification loop skill: runs command after turn to verify results.
pub struct VerificationLoopSkill {
    config: VerificationConfig,
    fix_pass_count: std::sync::atomic::AtomicUsize,
}

impl VerificationLoopSkill {
    pub fn new(config: VerificationConfig) -> Self { Self { config, fix_pass_count: std::sync::atomic::AtomicUsize::new(0) } }
    pub(crate) fn needs_verification(message: &str) -> bool { message.contains("```") || message.contains("file") || message.contains("fn ") || message.contains("class") || message.contains("const ") || message.contains("let ") }
    fn run_verification(command: &str) -> std::process::Output {
        let parts: Vec<&str> = command.split_whitespace().collect();
        if parts.is_empty() { return std::process::Command::new("true").output().unwrap(); }
        std::process::Command::new(parts[0]).args(&parts[1..]).output().unwrap_or_else(|_| std::process::Command::new("true").output().unwrap())
    }
}

impl HarnessSkill for VerificationLoopSkill {
    fn name(&self) -> &str { "verification_loop" }
    fn on_turn_end(&self, ctx: &TurnEndCtx) -> TurnEndResult {
        if !self.config.enabled { return TurnEndResult::Continue; }
        let command = match &self.config.command { Some(cmd) if !cmd.is_empty() => cmd, _ => return TurnEndResult::Continue };
        if !Self::needs_verification(&ctx.assistant_message) { return TurnEndResult::Continue; }
        let passes = self.fix_pass_count.load(std::sync::atomic::Ordering::Relaxed);
        if passes >= self.config.max_fix_passes { return TurnEndResult::Continue; }
        let output = Self::run_verification(command);
        if output.status.success() { self.fix_pass_count.store(0, std::sync::atomic::Ordering::Relaxed); TurnEndResult::Continue } 
        else { self.fix_pass_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed); TurnEndResult::RequestAnotherPass }
    }
}
