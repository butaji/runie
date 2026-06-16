use std::fs;
use std::path::{Path, PathBuf};

// Long-term targets from AGENTS.md. The allow-lists below are temporary while
// the R3 simplification pass shrinks the oversized modules/functions.
const MAX_FILE_LINES: usize = 500;
const MAX_FUNCTION_LINES: usize = 40;
const MAX_COMPLEXITY: usize = 10;

/// Files currently over `MAX_FILE_LINES`. Each entry should reference a task
/// that will eventually remove it from this list.
const ALLOWED_FILE_VIOLATIONS: &[&str] = &[
    "crates/runie-core/src/actors/fff_indexer.rs",       // extract-core-monolith
    "crates/runie-core/src/harness_skills.rs",           // extract-core-monolith
    "crates/runie-core/src/keybindings.rs",              // consolidate-keybindings-into-config-toml
    "crates/runie-core/src/markdown.rs",                 // pre-existing: is_break helper
    "crates/runie-core/src/orchestrator.rs",             // r4-orchestrator-actor
    "crates/runie-core/src/skills.rs",                   // adopt-serde-yaml-skills cleanup
    "crates/runie-core/src/state.rs",                    // complete-appstate-refactor
    "crates/runie-core/src/tool/mod.rs",                 // unify-tool-implementations
    "crates/runie-core/src/tool/search.rs",              // fff-unified-search-tool cleanup
    "crates/runie-core/src/update/agent.rs",             // coalesce-update-modules
    "crates/runie-core/src/update/dialog/mod.rs",        // coalesce-update-modules
    "crates/runie-core/src/update/mod.rs",               // coalesce-update-modules
    "crates/runie-agent/src/tools.rs",                   // unify-tool-implementations
    "crates/runie-provider/src/planner.rs",              // extract-core-monolith
    "crates/runie-tui/src/theme.rs",                     // grok-theme-quantization cleanup
    "crates/runie-tui/src/ui/messages.rs",               // unify-rendering-pipeline
];

/// Functions currently over `MAX_FUNCTION_LINES` or `MAX_COMPLEXITY`.
/// Format: (file_path, function_signature_or_name_prefix).
const ALLOWED_FUNC_VIOLATIONS: &[(&str, &str)] = &[
    ("crates/runie-core/src/actors/fff_indexer.rs", "async fn init_fff"),
    ("crates/runie-core/src/actors/fff_indexer.rs", "async fn handle_search"),
    ("crates/runie-core/src/actors/fff_indexer.rs", "fn format_git_status_str"),
    ("crates/runie-core/src/actors/fff_indexer.rs", "async fn indexer_initializes_in_temp_dir"),
    ("crates/runie-core/src/actors/fff_indexer.rs", "async fn indexer_answers_file_search"),
    ("crates/runie-core/src/actors/fff_indexer.rs", "async fn search_request_event_returns_results"),
    ("crates/runie-core/src/config_migrate.rs", "fn v2_to_v3"),
    ("crates/runie-core/src/config_reload/watcher.rs", "pub fn spawn_config_watcher"),
    ("crates/runie-core/src/dialog/builders.rs", "fn add_session_section"),
    ("crates/runie-core/src/dialog/builders.rs", "fn session_list_builds_with_sections"),
    ("crates/runie-core/src/event/variants.rs", "pub fn to_durable"),
    ("crates/runie-core/src/event/variants_tests.rs", "fn dispatcher_handles_all_variants"),
    ("crates/runie-core/src/event/dialog_display.rs", "pub fn variant_name"),
    ("crates/runie-core/src/event/dialog_display.rs", "fn fmt"),
    ("crates/runie-core/src/location.rs", "pub fn parse_location"),
    ("crates/runie-core/src/session_actor.rs", "async fn session_actor_replays_to_uactor"),
    ("crates/runie-core/src/session_store.rs", "fn open_db"),
    ("crates/runie-core/src/skills.rs", "pub fn load_from_dir"),
    ("crates/runie-core/src/skills.rs", "fn parse_skill_md"),
    ("crates/runie-core/src/snapshot.rs", "pub fn compute_mouse_target"),
    ("crates/runie-core/src/tool/find_definitions.rs", "fn detect_kind"),
    ("crates/runie-core/src/tool/find_definitions.rs", "async fn call"),
    ("crates/runie-core/src/tool/search.rs", "fn search_impl"),
    ("crates/runie-core/src/tool/search.rs", "fn search_files"),
    ("crates/runie-core/src/tool/search.rs", "fn search_content"),
    ("crates/runie-core/src/tool/search.rs", "fn search_glob"),
    ("crates/runie-core/src/tool/search.rs", "fn format_git_status"),
    ("crates/runie-core/src/trait_resolver.rs", "pub(crate) fn score"),
    ("crates/runie-core/src/update/agent.rs", "pub fn agent_event"),
    ("crates/runie-core/src/update/dialog/mod.rs", "fn query_fff_files"),
    ("crates/runie-core/src/update/dialog/mod.rs", "fn format_fff_git_status"),
    ("crates/runie-core/src/update/input/mod.rs", "pub fn input_event"),
    ("crates/runie-core/src/update/input/nav.rs", "pub(crate) fn try_vim_nav_motion"),
    ("crates/runie-core/src/update/input/text.rs", "fn mode_hints"),
    ("crates/runie-core/src/update/mod.rs", "fn handle_orchestrator_event"),
    ("crates/runie-core/src/update/mod.rs", "pub fn update"),
    ("crates/runie-core/src/update/mod.rs", "fn is_dialog_event"),
    ("crates/runie-core/src/update/mod.rs", "fn dispatch_event"),
    ("crates/runie-core/src/update/dialog/panel.rs", "fn handle_panel_action"),
    ("crates/runie-agent/src/inspector.rs", "pub async fn call"),
    ("crates/runie-agent/src/subagent.rs", "async fn run_subagent_turn"),
    ("crates/runie-agent/src/tools.rs", "pub(crate) fn list_dir"),
    ("crates/runie-agent/src/tools/exec.rs", "fn write_file"),
    ("crates/runie-agent/src/turn.rs", "async fn execute_single_tool"),
    ("crates/runie-agent/src/tests/turn.rs", "async fn test_agent_loop_simple_response"),
    ("crates/runie-agent/src/tests/turn.rs", "async fn test_agent_loop_with_tool_call"),
    ("crates/runie-agent/src/tests/turn.rs", "async fn agent_tool_uses_core_trait"),
    ("crates/runie-agent/src/tests/turn.rs", "async fn tool_call_event_matches_output"),
    ("crates/runie-provider/src/mock.rs", "fn generate"),
    ("crates/runie-provider/src/planner.rs", "fn build_planner_system_prompt"),
    ("crates/runie-provider/src/planner.rs", "fn parse_raw_plan"),
    ("crates/runie-provider/src/planner.rs", "pub async fn plan"),
    ("crates/runie-provider/src/planner.rs", "async fn orchestrator_context_included_in_prompt"),
    ("crates/runie-tui/src/diff.rs", "fn parse_patch_hunks"),
    ("crates/runie-tui/src/diff.rs", "fn parse_line"),
    ("crates/runie-tui/src/main.rs", "async fn main"),
    ("crates/runie-tui/src/main.rs", "fn spawn_background_tasks"),
    ("crates/runie-tui/src/main.rs", "async fn agent_loop"),
    ("crates/runie-tui/src/popups/welcome.rs", "fn build_welcome_content"),
    ("crates/runie-tui/src/status_bar.rs", "pub(crate) fn build_left_text"),
    ("crates/runie-tui/src/terminal_setup.rs", "pub fn enable_mouse"),
    ("crates/runie-tui/src/terminal_setup.rs", "pub fn enable_mouse_grok_style"),
    ("crates/runie-tui/src/terminal_setup.rs", "pub fn disable_mouse_grok_style"),
    ("crates/runie-tui/src/terminal_setup.rs", "fn mouse_init_sequence_includes_all_grok_modes"),
    ("crates/runie-tui/src/terminal_setup.rs", "fn cleanup_sequence_disables_all_modes"),
    ("crates/runie-tui/src/tests/render/flow.rs", "fn test_formatted_labels_short_names"),
    ("crates/runie-tui/src/ui/mouse.rs", "pub fn compute_mouse_target"),
];

fn find_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_rust_files(&path));
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
}

fn relative_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn check_file_length(rel_path: &str, lines: &[&str], errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    if lines.len() > MAX_FILE_LINES {
        let msg = format!(
            "{}: {} lines (max {})",
            rel_path,
            lines.len(),
            MAX_FILE_LINES
        );
        if ALLOWED_FILE_VIOLATIONS.contains(&rel_path) {
            warnings.push(format!("[allowed] {}", msg));
        } else {
            errors.push(msg);
        }
    }
}

fn count_complexity(trimmed: &str) -> usize {
    trimmed.matches("if ").count()
        + trimmed.matches("else if").count()
        + trimmed.matches("match ").count()
        + trimmed.matches("while ").count()
        + trimmed.matches("for ").count()
        + trimmed.matches("&&").count()
        + trimmed.matches("||").count()
        + trimmed.matches('?').count()
}

fn is_function_start(trimmed: &str) -> bool {
    if trimmed.ends_with(';') {
        return false;
    }
    let mut tokens = trimmed.split_whitespace().peekable();
    loop {
        match tokens.peek().copied() {
            Some("pub") | Some("pub(crate)") | Some("pub(super)") | Some("crate") => {
                tokens.next();
            }
            Some("async") | Some("const") | Some("unsafe") | Some("static") => {
                tokens.next();
            }
            Some("fn") => {
                tokens.next();
                return tokens
                    .next()
                    .map(|name| !name.starts_with('('))
                    .unwrap_or(false);
            }
            _ => return false,
        }
    }
}

fn is_allowed_func(rel_path: &str, fn_name: &str) -> bool {
    ALLOWED_FUNC_VIOLATIONS
        .iter()
        .any(|(file, prefix)| rel_path == *file && fn_name.starts_with(*prefix))
}

fn report_fn_violation(
    rel_path: &str,
    fn_start: usize,
    fn_name: &str,
    fn_len: usize,
    complexity: usize,
    errors: &mut Vec<String>,
    warnings: &mut Vec<String>,
) {
    if fn_len > MAX_FUNCTION_LINES {
        let msg = format!(
            "{}:{}: function {} lines (max {})",
            rel_path,
            fn_start + 1,
            fn_len,
            MAX_FUNCTION_LINES
        );
        if is_allowed_func(rel_path, fn_name) {
            warnings.push(format!("[allowed] {}", msg));
        } else {
            errors.push(msg);
        }
    }
    if complexity > MAX_COMPLEXITY {
        let msg = format!(
            "{}:{}: {} complexity {} (max {})",
            rel_path,
            fn_start + 1,
            fn_name,
            complexity,
            MAX_COMPLEXITY
        );
        if is_allowed_func(rel_path, fn_name) {
            warnings.push(format!("[allowed] {}", msg));
        } else {
            errors.push(msg);
        }
    }
}

#[derive(Default)]
struct FnTracker {
    in_fn: bool,
    in_fn_body: bool,
    fn_start: usize,
    brace_depth: usize,
    fn_complexity: usize,
    fn_name: String,
}

impl FnTracker {
    fn start(&mut self, i: usize, trimmed: &str) {
        self.in_fn = true;
        self.in_fn_body = false;
        self.fn_start = i;
        self.fn_complexity = 1;
        self.fn_name = trimmed.lines().next().unwrap_or("").to_string();
    }

    fn update_braces(&mut self, trimmed: &str) {
        let opens = trimmed.matches('{').count();
        let closes = trimmed.matches('}').count();
        self.brace_depth = self.brace_depth.saturating_add(opens);
        self.brace_depth = self.brace_depth.saturating_sub(closes);
        if opens > 0 {
            self.in_fn_body = true;
        }
    }

    fn ended(&self, trimmed: &str) -> bool {
        self.in_fn_body && self.brace_depth == 0 && trimmed.contains('}')
    }

    fn report_and_reset(&mut self, path: &str, i: usize, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
        let fn_len = i - self.fn_start + 1;
        report_fn_violation(
            path,
            self.fn_start,
            &self.fn_name,
            fn_len,
            self.fn_complexity,
            errors,
            warnings,
        );
        self.in_fn = false;
        self.in_fn_body = false;
        self.fn_complexity = 0;
        self.fn_name.clear();
    }
}

fn check_function_violations(rel_path: &str, lines: &[&str], errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let mut tracker = FnTracker::default();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();

        if !tracker.in_fn && is_function_start(trimmed) {
            tracker.start(i, trimmed);
        }

        if tracker.in_fn {
            tracker.update_braces(trimmed);
            tracker.fn_complexity += count_complexity(trimmed);

            if tracker.ended(trimmed) {
                tracker.report_and_reset(rel_path, i, errors, warnings);
            }
        }
    }
}

fn lint_file(path: &Path, workspace_root: &Path, errors: &mut Vec<String>, warnings: &mut Vec<String>) {
    let rel_path = relative_path(path, workspace_root);
    let content = fs::read_to_string(path).unwrap();
    let lines: Vec<_> = content.lines().collect();
    check_file_length(&rel_path, &lines, errors, warnings);
    check_function_violations(&rel_path, &lines, errors, warnings);
}

fn main() {
    if std::env::var("RUNIE_SKIP_BUILD_CHECKS").is_ok() {
        return;
    }

    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let workspace_root = Path::new(&manifest_dir).parent().unwrap().parent().unwrap();
    let crates_path = workspace_root.join("crates");

    for path in find_rust_files(&crates_path) {
        if !path.to_string_lossy().contains("target/") {
            lint_file(&path, workspace_root, &mut errors, &mut warnings);
        }
    }

    if !warnings.is_empty() {
        eprintln!("\n=== RUNIE LINT ALLOWED VIOLATIONS (temporary) ===\n");
        for warn in &warnings {
            eprintln!("  {}", warn);
        }
        eprintln!("\n{} allowed violations\n", warnings.len());
    }

    if !errors.is_empty() {
        eprintln!("\n=== RUNIE LINT VIOLATIONS ===\n");
        for err in &errors {
            eprintln!("  {}", err);
        }
        eprintln!("\n{} violations found\n", errors.len());
        std::process::exit(1);
    }
}
