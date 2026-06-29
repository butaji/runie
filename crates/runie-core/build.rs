use std::fs;
use std::path::{Path, PathBuf};

// ── AppState field-access guardrail ──────────────────────────────────────────
//
// Private AppState fields must be accessed through accessors, not directly.
// Uses a trailing dot to match field access (state.session.xxx) but NOT
// method calls (state.session(), state.session_mut()).
const APPSTATE_PATTERNS: &[(&str, &str)] = &[
    ("state.session.", "state.session()"),
    ("state.input.", "state.input()"),
    ("state.agent.", "state.agent_state()"),
    ("state.view.", "state.view()"),
    ("state.config.", "state.config()"),
    ("state.completion.", "state.completion()"),
    ("state.should_quit ", "state.should_quit_mut()"),
    ("state.should_quit\n", "state.should_quit_mut()"),
    ("state.should_quit{", "state.should_quit_mut()"),
    ("state.open_dialog ", "state.open_dialog_mut()"),
    ("state.open_dialog.", "state.open_dialog_mut()"),
    ("state.dialog_back_stack.", "state.dialog_back_stack_mut()"),
    ("state.login_flow ", "state.login_flow_mut()"),
    ("state.login_flow.", "state.login_flow_mut()"),
    ("state.transient_message ", "state.transient_message_mut()"),
    ("state.transient_until ", "state.transient_until_mut()"),
    ("state.transient_level ", "state.transient_level_mut()"),
    ("state.fff_file_results.", "state.fff_file_results()"),
    ("state.fff_debounce ", "state.fff_debounce_mut()"),
    ("state.perm_req ", "state.permission_request_opt()"),
    ("state.perm_req.", "state.permission_request_opt()."),
    ("state.cwd_name ", "state.cwd_name_mut()"),
    ("state.git_info ", "state.git_info_mut()"),
    ("state.git_info.", "state.git_info_mut()"),
    ("state.skills ", "state.skills_mut()"),
    ("state.prompts ", "state.prompts_mut()"),
    ("state.trust_decisions ", "state.trust_decisions_mut()"),
    ("state.trust_decisions.", "state.trust_decisions_mut()"),
    ("state.actor_handles ", "state.actor_handles_mut()"),
    ("state.registry ", "state.registry_mut()"),
    ("state.registry.", "state.registry_mut()"),
    // self.xxx patterns (same replacement, different prefix)
    ("self.session.", "self.session()"),
    ("self.input.", "self.input()"),
    ("self.agent.", "self.agent_state()"),
    ("self.view.", "self.view()"),
    ("self.config.", "self.config()"),
    ("self.completion.", "self.completion()"),
    ("self.should_quit ", "self.should_quit_mut()"),
    ("self.should_quit\n", "self.should_quit_mut()"),
    ("self.should_quit{", "self.should_quit_mut()"),
    ("self.open_dialog ", "self.open_dialog_mut()"),
    ("self.open_dialog.", "self.open_dialog_mut()"),
    ("self.dialog_back_stack.", "self.dialog_back_stack_mut()"),
    ("self.login_flow ", "self.login_flow_mut()"),
    ("self.login_flow.", "self.login_flow_mut()"),
    ("self.transient_message ", "self.transient_message_mut()"),
    ("self.transient_until ", "self.transient_until_mut()"),
    ("self.transient_level ", "self.transient_level_mut()"),
    ("self.fff_file_results.", "self.fff_file_results_mut()"),
    ("self.fff_debounce ", "self.fff_debounce_mut()"),
    ("self.permission_request ", "self.permission_request_mut()"),
    ("self.cwd_name ", "self.cwd_name_mut()"),
    ("self.git_info ", "self.git_info_mut()"),
    ("self.git_info.", "self.git_info_mut()"),
    ("self.skills ", "self.skills_mut()"),
    ("self.prompts ", "self.prompts_mut()"),
    ("self.trust_decisions ", "self.trust_decisions_mut()"),
    ("self.trust_decisions.", "self.trust_decisions_mut()"),
    ("self.actor_handles ", "self.actor_handles_mut()"),
    ("self.registry ", "self.registry_mut()"),
    ("self.registry.", "self.registry_mut()"),
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

fn is_test_file(rel_path: &str) -> bool {
    rel_path.contains("/tests/")
        || rel_path.ends_with("/tests.rs")
        || rel_path.ends_with("_tests.rs")
        || rel_path.ends_with("_test.rs")
        || rel_path.contains("_tests.")
        || rel_path.contains("_test.")
}

/// Files exempt from the AppState field-access check.
fn needs_appstate_lint(rel_path: &str) -> bool {
    let exemptions = [
        "build.rs",
        "accessors.rs",
        "domain_ops.rs",
        "actors/config/actor.rs",
        "actors/config/ractor_config.rs",
        "actors/permission/actor.rs",
        "actors/permission/ractor_permission.rs",
        "actors/input/actor.rs",
        "actors/input/messages.rs",
        "actors/ui_control/actor.rs",
        "actors/handles.rs",
        "actors/leader/actor.rs",
        "update/input/text.rs",
        "update/input/submit.rs",
        "retry.rs",
        "session/replay.rs",
        "login_flow/validation.rs",
        "model/state/input.rs",
    ];
    !is_test_file(rel_path)
        && !rel_path.contains("/benches/")
        && !rel_path.contains("/harness_skills/")
        && !exemptions.iter().any(|e| rel_path.ends_with(e))
}

fn check_appstate_field_access(
    rel_path: &str,
    lines: &[&str],
    errors: &mut Vec<String>,
) {
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            continue;
        }
        for (pattern, suggestion) in APPSTATE_PATTERNS {
            if line.contains(pattern) {
                errors.push(format!(
                    "{}:{}: direct AppState field access `{pattern}` — use {suggestion}",
                    rel_path,
                    i + 1
                ));
                break;
            }
        }
    }
}

fn lint_file(path: &Path, workspace_root: &Path, errors: &mut Vec<String>) {
    let rel_path = relative_path(path, workspace_root);
    if needs_appstate_lint(&rel_path) {
        let content = fs::read_to_string(path).unwrap();
        let lines: Vec<_> = content.lines().collect();
        check_appstate_field_access(&rel_path, &lines, errors);
    }
}

fn main() {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default();
    let workspace_root =
        Path::new(&manifest_dir).parent().unwrap().parent().unwrap();

    // Validate bundled subagent type checksums.
    if let Err(msg) =
        validate_agent_manifest(PathBuf::from(&manifest_dir).join("resources").join("agents"))
    {
        eprintln!("\n=== AGENT MANIFEST VALIDATION FAILED ===\n  {}\n\n", msg);
        std::process::exit(1);
    }

    let mut errors = Vec::new();
    let crates_path = workspace_root.join("crates");

    for path in find_rust_files(&crates_path) {
        if !path.to_string_lossy().contains("target/") {
            lint_file(&path, workspace_root, &mut errors);
        }
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

/// Validate that all files in `resources/agents/manifest.json` match their
/// stored SHA-256 checksums.
fn validate_agent_manifest(agents_dir: PathBuf) -> Result<(), String> {
    let manifest_path = agents_dir.join("manifest.json");
    let manifest_json =
        fs::read_to_string(&manifest_path).map_err(|e| format!("failed to read manifest.json: {}", e))?;

    #[derive(serde::Deserialize)]
    struct Manifest {
        files: std::collections::HashMap<String, String>,
    }
    let manifest: Manifest = serde_json::from_str(&manifest_json)
        .map_err(|e| format!("failed to parse manifest.json: {}", e))?;

    use sha2::{Digest, Sha256};
    for (filename, expected_hash) in &manifest.files {
        let file_path = agents_dir.join(filename);
        let content =
            fs::read(&file_path).map_err(|e| format!("failed to read {}: {}", filename, e))?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let actual = hex::encode(hasher.finalize());
        if &actual != expected_hash {
            return Err(format!(
                "checksum mismatch for {}: expected {}, got {}",
                filename, expected_hash, actual
            ));
        }
    }
    Ok(())
}
