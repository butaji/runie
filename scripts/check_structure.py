#!/usr/bin/env python3
"""Workspace structural linter.

Enforces the advertised file/function/complexity limits on production Rust code:
- File length <= 500 lines
- Function length <= 40 lines
- Approximate complexity <= 10

Tests, benches, and build.rs are exempt. Existing legacy violations are listed in
exemption sets below; new code must stay within the limits.
"""

import os
import re
import sys
from pathlib import Path

MAX_FILE_LINES = 500
MAX_FUNCTION_LINES = 40
MAX_COMPLEXITY = 10

# Files exempt from the file-length limit.
FILE_LENGTH_EXEMPTS = {
    "crates/runie-testing/src/keystroke_dsl.rs",
    "crates/runie-core/build.rs",
    "crates/runie-core/src/shell.rs",
    "crates/runie-core/src/subagents/mod.rs",
    "crates/runie-core/src/update/dispatch.rs",
    "crates/runie-core/src/model_catalog/mod.rs",
    "crates/runie-core/src/config/config_impl.rs",
    "crates/runie-core/src/actors/session/session_handlers.rs",
    "crates/runie-core/src/event/generated.rs",
    "crates/runie-core/src/event/mod.rs",
    "crates/runie-core/src/session/store.rs",
    "crates/runie-core/src/session/tree.rs",
    "crates/runie-core/src/session/replay.rs",
    "crates/runie-agent/src/stream_response.rs",
    "crates/runie-agent/src/tool/find_definitions.rs",
    "crates/runie-tui/src/bootstrap.rs",
    "crates/runie-tui/src/ui_actor/mod.rs",
    "crates/runie-tui/src/popups/panel/form.rs",
    "crates/runie-provider/src/mock.rs",
    "crates/runie-provider/src/openai/protocol.rs",
    "crates/runie-cli/src/inspect/mod.rs",
}

# Functions exempt from the function-length limit.
FUNCTION_LENGTH_EXEMPTS = {
    ("crates/runie-testing/src/tool_aliases.rs", "transform_args"),
    ("crates/runie-testing/src/keystroke_dsl.rs", "parse_keystroke"),
    ("crates/runie-testing/src/keystroke_dsl.rs", "parse_sequence"),
    ("crates/runie-testing/src/keystroke_dsl.rs", "key_event_to_dsl"),
    ("crates/runie-core/build.rs", "needs_magic_number_lint"),
    ("crates/runie-core/build.rs", "check_magic_numbers"),
    ("crates/runie-core/build.rs", "check_orphan_spawns"),
    ("crates/runie-core/build.rs", "main"),
    ("crates/runie-core/src/layout.rs", "line_count_matches_textwrap"),
    ("crates/runie-core/src/shell.rs", "run_command"),
    ("crates/runie-core/src/bash_safety.rs", "has_recursive_rm"),
    ("crates/runie-core/src/location.rs", "extract_location"),
    ("crates/runie-core/src/sandbox.rs", "sandbox_available"),
    ("crates/runie-core/src/tracing_init.rs", "init_with_mode"),
    ("crates/runie-core/src/proto/message/validation.rs", "validate_messages"),
    ("crates/runie-core/src/update/dispatch.rs", "handle_turn_events"),
    ("crates/runie-core/src/update/dispatch.rs", "handle_persistence_events"),
    ("crates/runie-core/src/update/dispatch.rs", "handle_session_store_events"),
    ("crates/runie-core/src/update/system.rs", "handle_new_session"),
    ("crates/runie-core/src/update/input/mod.rs", "plan_mode_input_event"),
    ("crates/runie-core/src/update/input/mod.rs", "permission_input_event"),
    ("crates/runie-core/src/update/agent/model_config.rs", "handle_main_events"),
    ("crates/runie-core/src/update/dialog/router.rs", "process_command_result"),
    ("crates/runie-core/src/update/dialog/open.rs", "open_command_palette_with_filter"),
    ("crates/runie-core/src/update/dialog/form/mod.rs", "handle_form_submit"),
    ("crates/runie-core/src/config/config_impl.rs", "save_to"),
    ("crates/runie-core/src/config/config_impl.rs", "save_nonblocking_to"),
    ("crates/runie-core/src/config/config_impl.rs", "save_handles_corrupt_existing_file"),
    ("crates/runie-core/src/config/migrate.rs", "v3_to_v4"),
    ("crates/runie-core/src/config/migrate.rs", "migrate_v3_to_v4_removes_plaintext_api_keys"),
    ("crates/runie-core/src/config/layers.rs", "load_layers_from_paths"),
    ("crates/runie-core/src/config/layers.rs", "figment_env_overrides_take_precedence"),
    ("crates/runie-core/src/markdown/parsing.rs", "inlines_to_text"),
    ("crates/runie-core/src/markdown/parsing.rs", "start_tag"),
    ("crates/runie-core/src/markdown/parsing.rs", "end_tag"),
    ("crates/runie-core/src/markdown/heal.rs", "scan_raw_openers"),
    ("crates/runie-core/src/io/atomic_write.rs", "atomic_write"),
    ("crates/runie-core/src/io/atomic_write.rs", "atomic_write_concurrent_stress"),
    ("crates/runie-core/src/provider/config.rs", "save_provider_config_persists_under_runtime"),
    ("crates/runie-core/src/provider/config.rs", "concurrent_provider_saves_do_not_corrupt_config"),
    ("crates/runie-core/src/diff/mod.rs", "diffy_to_canonical"),
    ("crates/runie-core/src/diff/mod.rs", "fallback_parse_diff"),
    ("crates/runie-core/src/mcp/spike_client.rs", "rmcp_client_connects_to_echo_server"),
    ("crates/runie-core/src/mcp/connection.rs", "start_server"),
    ("crates/runie-core/src/mcp/connection.rs", "start_server_creates_handle"),
    ("crates/runie-core/src/mcp/connection.rs", "stop_server_updates_state"),
    ("crates/runie-core/src/login_flow/state/mod.rs", "validate_step"),
    ("crates/runie-core/src/actors/fff_indexer/search_index.rs", "build"),
    ("crates/runie-core/src/actors/fff_indexer/search_index.rs", "fuzzy_search"),
    ("crates/runie-core/src/actors/config/ractor_config.rs", "load_layers_returns_effective_config"),
    ("crates/runie-core/src/actors/config/ractor_config.rs", "mcp_server_roundtrip"),
    ("crates/runie-core/src/actors/config/ractor_config.rs", "config_actor_emits_error_on_invalid_config"),
    ("crates/runie-core/src/actors/config/messages.rs", "clone"),
    ("crates/runie-core/src/actors/input/messages.rs", "apply_to"),
    ("crates/runie-core/src/actors/provider/ractor_provider.rs", "provider_actor_mailbox_not_blocked_by_validate"),
    ("crates/runie-core/src/actors/leader/handle.rs", "shutdown"),
    ("crates/runie-core/src/actors/leader/test_helpers.rs", "test_leader_handle"),
    ("crates/runie-core/src/actors/turn/messages.rs", "clone"),
    ("crates/runie-core/src/model/compaction.rs", "compact"),
    ("crates/runie-core/src/model/compaction.rs", "truncate_fenced_code_blocks"),
    ("crates/runie-core/src/model/compaction.rs", "truncate_details_blocks"),
    ("crates/runie-core/src/model/cache/mod.rs", "snapshot_feed"),
    ("crates/runie-core/src/commands/dsl/embedded_commands.rs", "command_names_are_valid"),
    ("crates/runie-core/src/event/headless.rs", "try_from_event"),
    ("crates/runie-core/src/event/generated.rs", "kind"),
    ("crates/runie-core/src/event/generated.rs", "category"),
    ("crates/runie-core/src/event/generated.rs", "into_intent"),
    ("crates/runie-core/src/event/generated.rs", "is_fact_variant"),
    ("crates/runie-core/src/event/durable.rs", "try_from_event"),
    ("crates/runie-core/src/tool/shim/minimax.rs", "parse_invoke_blocks"),
    ("crates/runie-core/src/session/tree.rs", "to_snapshot"),
    ("crates/runie-core/src/session/tree.rs", "from_snapshot"),
    ("crates/runie-core/src/session/replay.rs", "session_save_preserves_message_parts"),
    ("crates/runie-core/src/session/replay.rs", "save_after_completed_turn_creates_session_file"),
    ("crates/runie-core/src/session/replay.rs", "replay_tree_snapshot_restore"),
    ("crates/runie-agent/src/tool_runner.rs", "tool_cache_not_used_for_write_tools"),
    ("crates/runie-agent/src/headless/mod.rs", "run_round"),
    ("crates/runie-agent/src/tool/bash.rs", "execute"),
    ("crates/runie-agent/src/tool/find_definitions.rs", "execute"),
    ("crates/runie-agent/src/tool/find.rs", "run_find"),
    ("crates/runie-agent/src/tool/find.rs", "run_find_simple"),
    ("crates/runie-tui/src/input_mapping.rs", "route_to_input_actor"),
    ("crates/runie-tui/src/input_mapping.rs", "route_to_input_actor_returns_true_for_routable_events"),
    ("crates/runie-tui/src/bootstrap.rs", "to_crossterm_event"),
    ("crates/runie-tui/src/app_init.rs", "bootstrap_emits_load_intents"),
    ("crates/runie-tui/src/ui/messages/mod.rs", "blockquote_renders_inline_styles"),
    ("crates/runie-tui/src/ui_actor/mod.rs", "handle_input_event"),
    ("crates/runie-tui/src/message/support.rs", "wrap_styled_spans_for_blockquote"),
    ("crates/runie-tui/src/popups/permission.rs", "format_tool_input"),
    ("crates/runie-tui/src/popups/permission.rs", "test_fetch_docs_formatting"),
    ("crates/runie-provider/src/retry.rs", "from_sse_error_uses_shared_classifier"),
    ("crates/runie-cli/src/login.rs", "run"),
    ("crates/runie-cli/src/print.rs", "print_mode_emits_jsonl_events"),
    ("crates/runie-cli/src/inspect/mod.rs", "validate_config"),
    ("crates/runie-cli/src/inspect/mod.rs", "generate_setup_hints"),
}

# Functions exempt from the complexity limit.
COMPLEXITY_EXEMPTS = {
    ("crates/runie-testing/src/keystroke_dsl.rs", "parse_keystroke"),
    ("crates/runie-testing/src/keystroke_dsl.rs", "parse_sequence"),
    ("crates/runie-testing/src/keystroke_dsl.rs", "key_event_to_dsl"),
    ("crates/runie-testing/src/keystroke_dsl.rs", "test_named_keys"),
    ("crates/runie-core/build.rs", "check_magic_numbers"),
    ("crates/runie-core/build.rs", "check_orphan_spawns"),
    ("crates/runie-core/src/dry_run.rs", "resolve_provider_model"),
    ("crates/runie-core/src/input_history.rs", "load_history"),
    ("crates/runie-core/src/input_history.rs", "save_history"),
    ("crates/runie-core/src/input_history.rs", "append_history"),
    ("crates/runie-core/src/bash_safety.rs", "has_recursive_rm"),
    ("crates/runie-core/src/location.rs", "parse_search_query"),
    ("crates/runie-core/src/tool_markers/strip.rs", "find_object_end"),
    ("crates/runie-core/src/proto/message/validation.rs", "validate_message"),
    ("crates/runie-core/src/update/command.rs", "run_fork_command"),
    ("crates/runie-core/src/update/mod.rs", "strip_with_unclosed"),
    ("crates/runie-core/src/update/mod.rs", "update"),
    ("crates/runie-core/src/update/input/text.rs", "mode_hints"),
    ("crates/runie-core/src/update/dialog/router.rs", "process_command_result"),
    ("crates/runie-core/src/update/dialog/form_handler.rs", "handle_form_dialog"),
    ("crates/runie-core/src/update/dialog/form_handler.rs", "compute_before_text"),
    ("crates/runie-core/src/update/dialog/form/mod.rs", "form_panel_edit_action"),
    ("crates/runie-core/src/update/dialog/form/mod.rs", "handle_form_input"),
    ("crates/runie-core/src/update/dialog/form/mod.rs", "handle_form_submit"),
    ("crates/runie-core/src/config/config_impl.rs", "save_to"),
    ("crates/runie-core/src/config/config_impl.rs", "save_nonblocking_to"),
    ("crates/runie-core/src/config/migrate.rs", "v3_to_v4"),
    ("crates/runie-core/src/auth/keyring.rs", "migrate_legacy_auth"),
    ("crates/runie-core/src/markdown/heal.rs", "scan_raw_openers"),
    ("crates/runie-core/src/provider/registry.rs", "openrouter_model_matches_canonical"),
    ("crates/runie-core/src/diff/mod.rs", "to_unified_string"),
    ("crates/runie-core/src/diff/mod.rs", "parse"),
    ("crates/runie-core/src/diff/mod.rs", "diffy_to_canonical"),
    ("crates/runie-core/src/diff/mod.rs", "fallback_parse_diff"),
    ("crates/runie-core/src/mcp/spike_client.rs", "rmcp_client_connects_to_echo_server"),
    ("crates/runie-core/src/mcp/connection.rs", "start_server"),
    ("crates/runie-core/src/permissions/rules.rs", "matches"),
    ("crates/runie-core/src/dialog/score.rs", "command_match_score"),
    ("crates/runie-core/src/login_flow/state/mod.rs", "validate_step"),
    ("crates/runie-core/src/harness_skills/hashline_edit.rs", "try_apply_hashline"),
    ("crates/runie-core/src/harness_skills/hashline_edit.rs", "on_tool_call"),
    ("crates/runie-core/src/harness_skills/loop_detector.rs", "check_loop"),
    ("crates/runie-core/src/harness_skills/verification_loop.rs", "on_turn_end"),
    ("crates/runie-core/src/actors/fff_indexer/search_index.rs", "fuzzy_search"),
    ("crates/runie-core/src/actors/fff_indexer/ractor_fff_indexer.rs", "handle_search"),
    ("crates/runie-core/src/actors/fff_indexer/git_status.rs", "format_git_status"),
    ("crates/runie-core/src/actors/input/messages.rs", "apply_to"),
    ("crates/runie-core/src/model/compaction.rs", "truncate_structural"),
    ("crates/runie-core/src/view/transform.rs", "should_skip_msg"),
    ("crates/runie-core/src/tool/format.rs", "format_bytes"),
    ("crates/runie-core/src/session/store.rs", "load_events_internal"),
    ("crates/runie-core/src/session/tree.rs", "fork_at"),
    ("crates/runie-core/src/session/persistence/header.rs", "read_header"),
    ("crates/runie-agent/src/tool/find.rs", "run_find"),
    ("crates/runie-agent/src/tool/find.rs", "run_find_simple"),
    ("crates/runie-agent/src/tool/search/types.rs", "format_git_status"),
    ("crates/runie-tui/src/keymap.rs", "convert_key_event"),
    ("crates/runie-tui/src/keymap.rs", "map_key_event"),
    ("crates/runie-tui/src/bootstrap.rs", "to_crossterm_event"),
    ("crates/runie-tui/src/terminal_setup.rs", "mouse_init_sequence_includes_all_grok_modes"),
    ("crates/runie-tui/src/terminal_setup.rs", "cleanup_sequence_disables_all_modes"),
    ("crates/runie-tui/src/ui_actor/mod.rs", "handle_input_event"),
    ("crates/runie-tui/src/message/support.rs", "wrap_styled_spans_for_blockquote"),
    ("crates/runie-tui/src/popups/permission.rs", "format_tool_input"),
    ("crates/runie-tui/src/popups/permission.rs", "test_fetch_docs_formatting"),
    ("crates/runie-tui/src/popups/panel/mod.rs", "panel_dialog"),
    ("crates/runie-provider/src/mock.rs", "detect_fixture"),
    ("crates/runie-provider/src/retry.rs", "classify_http_status"),
    ("crates/runie-provider/src/retry.rs", "from_sse_error_uses_shared_classifier"),
    ("crates/runie-provider/src/openai/stream.rs", "parse_error_value"),
    ("crates/runie-cli/src/inspect/mod.rs", "validate_config"),
}

# Whole files that are exempt from per-function structural checks.
FUNCTION_LINT_EXEMPT_FILES = {
    "crates/runie-core/build.rs",
    "crates/runie-core/src/event/generated.rs",
}

FN_START_RE = re.compile(r"^\s*(?:pub\s+|crate\s+|async\s+|unsafe\s+|const\s+)*fn\s+(\w+)")
COMPLEXITY_TOKEN_RES = [
    re.compile(re.escape(t)) for t in [
        "if", "else if", "match", "while", "for", "loop",
        "break", "continue", "return", "&&", "||", "?",
    ]
]


def is_test_file(rel: str) -> bool:
    return (
        "/tests/" in rel
        or rel.endswith("/tests.rs")
        or rel.endswith("_tests.rs")
        or rel.endswith("_test.rs")
        or "_tests." in rel
        or "_test." in rel
        or "/benches/" in rel
    )


def find_rust_files(workspace: Path):
    for path in workspace.rglob("*.rs"):
        rel = path.relative_to(workspace).as_posix()
        if "target" in path.parts or is_test_file(rel):
            continue
        yield path, rel


def count_braces(line: str) -> int:
    # Strip line comments to avoid counting braces inside them.
    if "//" in line:
        line = line.split("//")[0]
    return line.count("{") - line.count("}")


def check_file(path: Path, rel: str) -> list[str]:
    errors = []
    lines = path.read_text(encoding="utf-8").splitlines()
    if rel not in FILE_LENGTH_EXEMPTS and len(lines) > MAX_FILE_LINES:
        errors.append(
            f"{rel}: file has {len(lines)} lines (max {MAX_FILE_LINES})"
        )

    if rel in FUNCTION_LINT_EXEMPT_FILES:
        return errors

    depth = 0
    in_fn = False
    fn_start_depth = 0
    fn_start_line = 0
    fn_name: str | None = None
    fn_lines: list[str] = []

    for i, raw_line in enumerate(lines):
        line = raw_line
        if not in_fn:
            m = FN_START_RE.match(line)
            if m:
                # Trait method declaration without a body.
                if line.strip().endswith(";"):
                    continue
                in_fn = True
                fn_start_depth = depth
                fn_start_line = i + 1
                fn_name = m.group(1)
                fn_lines = []

        if in_fn:
            fn_lines.append(line)
            depth += count_braces(line)
            if depth <= fn_start_depth:
                length = len(fn_lines)
                key = (rel, fn_name)
                if length > MAX_FUNCTION_LINES and key not in FUNCTION_LENGTH_EXEMPTS:
                    errors.append(
                        f"{rel}:{fn_start_line}: `{fn_name}` has {length} lines "
                        f"(max {MAX_FUNCTION_LINES})"
                    )

                complexity = 0
                for fl in fn_lines:
                    code = fl.split("//")[0]
                    for r in COMPLEXITY_TOKEN_RES:
                        complexity += len(r.findall(code))
                if complexity > MAX_COMPLEXITY and key not in COMPLEXITY_EXEMPTS:
                    errors.append(
                        f"{rel}:{fn_start_line}: `{fn_name}` has complexity {complexity} "
                        f"(max {MAX_COMPLEXITY})"
                    )

                in_fn = False
                fn_lines = []
                fn_name = None
        else:
            depth += count_braces(line)

    return errors


def main() -> int:
    workspace = Path(__file__).resolve().parent.parent
    all_errors: list[str] = []
    for path, rel in find_rust_files(workspace):
        all_errors.extend(check_file(path, rel))

    if all_errors:
        print("=== STRUCTURAL LINT VIOLATIONS ===")
        for err in all_errors:
            print(f"  {err}")
        print(f"\n{len(all_errors)} violation(s) found")
        return 1

    print(
        f"=== All production .rs files within {MAX_FILE_LINES}/"
        f"{MAX_FUNCTION_LINES}/{MAX_COMPLEXITY} limits ==="
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
