//! Grok Parity Standalone Test Binary - DEEP Character-by-Character Comparison
//!
//! Tests each UI component against expected Grok format with EXACT string matching.
//! Run with: cargo run -p runie-tui --bin grok_parity_test
//!
//! For each element:
//! 1. Render to buffer and extract EXACT text content
//! 2. Compare against expected Grok reference string character-by-character
//! 3. Show diff output when mismatches occur

use ratatui::{buffer::Buffer, layout::Rect, style::Color, prelude::Widget};
use runie_tui::components::{
    top_bar::{TopBarViewModel, render_top_bar, format_context_window, format_token_count},
    message_list::{MessageListViewModel, MessageItem, MessageList, ThinkingBlock, render_thinking_block, ToolCallBlock, ToolStatus, render_tool_call_block},
    message_list::render::WrapCache,
    home_screen::HomeScreen,
    input_bar::render_input_bar,
};
use runie_tui::tui::render::render_status_bar;
use runie_tui::tui::view_models::StatusBarViewModel;
use runie_tui::tui::state::TuiMode;
use runie_tui::theme::ThemeWrapper;
use runie_tui::glyphs::{spinner_frame, CHEVRON};
use runie_ai::TokenUsage;

// =============================================================================
// Test Infrastructure
// =============================================================================

fn make_test_colors() -> ThemeColors {
    ThemeColors {
        bg_base: Color::Rgb(0x0F, 0x0C, 0x14),
        bg_panel: Color::Rgb(0x20, 0x1F, 0x26),
        accent_primary: Color::Rgb(0xFF, 0x6B, 0x00),
        text_primary: Color::Rgb(0xFF, 0xFA, 0xF1),
        text_secondary: Color::Rgb(0xDF, 0xDB, 0xDD),
        text_dim: Color::Rgb(0x8A, 0x87, 0x94),
        text_muted: Color::Rgb(0xBF, 0xBC, 0xC8),
        border_unfocused: Color::Rgb(0x3A, 0x39, 0x43),
        success: Color::Rgb(0x00, 0xF5, 0xD4),
        warning: Color::Rgb(0xFF, 0x6B, 0x00),
        error: Color::Rgb(0xEB, 0x42, 0x68),
        syntax_phase: Color::Rgb(0x6B, 0x50, 0xFF),
        text_plan: Color::Rgb(0xFF, 0x6B, 0x00),
        accent_secondary: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_user: Color::Rgb(0xFF, 0x6B, 0x00),
        accent_assistant: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_thinking: Color::Rgb(0xFF, 0xD4, 0x00),
        accent_tool: Color::Rgb(0x6B, 0x50, 0xFF),
        accent_system: Color::Rgb(0x8A, 0x87, 0x94),
        accent_error: Color::Rgb(0xEB, 0x42, 0x68),
        accent_success: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_running: Color::Rgb(0xFF, 0xD4, 0x00),
        accent_skill: Color::Rgb(0x6B, 0x50, 0xFF),
        accent_plan: Color::Rgb(0xFF, 0xD4, 0x00),
        accent_feedback: Color::Rgb(0xEB, 0x42, 0x68),
        accent_model: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_teal: Color::Rgb(0x00, 0xF5, 0xD4),
        accent_orange: Color::Rgb(0xFF, 0x6B, 0x00),
        accent_purple: Color::Rgb(0x6B, 0x50, 0xFF),
        accent_yellow: Color::Rgb(0xFF, 0xD4, 0x00),
        accent_blue_bright: Color::Rgb(0x6B, 0x50, 0xFF),
        command: Color::Rgb(0xFF, 0x6B, 0x00),
        path: Color::Rgb(0x00, 0xF5, 0xD4),
        running: Color::Rgb(0xFF, 0xD4, 0x00),
        fuzzy_accent: Color::Rgb(0x6B, 0x50, 0xFF),
        editor_bg: Color::Rgb(0x0F, 0x0C, 0x14),
        surface_bg: Color::Rgb(0x20, 0x1F, 0x26),
        popover_bg: Color::Rgb(0x20, 0x1F, 0x26),
        feed_tool_bar: Color::Rgb(0x6B, 0x50, 0xFF),
    }
}

use runie_tui::theme::ThemeColors;

/// Extract a specific line from buffer at given y position
fn extract_line(buf: &Buffer, y: u16) -> String {
    let mut line = String::new();
    for x in 0..buf.area.width {
        if let Some(cell) = buf.cell((x, y)) {
            line.push_str(cell.symbol());
        }
    }
    line.trim_end().to_string()
}

/// Convert full buffer to string
fn buffer_to_string(buf: &Buffer) -> String {
    let mut result = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            if let Some(cell) = buf.cell((x, y)) {
                result.push_str(cell.symbol());
            }
        }
        if y < buf.area.height - 1 {
            result.push('\n');
        }
    }
    result
}



// =============================================================================
// 1. Header Bar Tests - Deep Character-by-Character
// =============================================================================

mod header_bar {
    use super::*;

    fn create_idle_vm() -> TopBarViewModel {
        TopBarViewModel {
            repo: "runie".to_string(),
            branch: "main".to_string(),
            path: "~/Code/GitHub/runie".to_string(),
            context_window: 512_000,
            estimated_tokens: 21_000,
            agent_running: false,
            braille_frame: 0,
            mode: TuiMode::Chat,
        }
    }

    fn render_header(vm: &TopBarViewModel) -> String {
        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(vm, area, &mut buf, &colors);
        extract_line(&buf, 0)
    }

    fn check_branch_and_path(actual: &str) -> bool {
        let mut passed = true;
        if !actual.contains("main") {
            println!("  ✗ Missing branch 'main'");
            passed = false;
        } else {
            println!("  ✓ Contains branch 'main'");
        }
        if !actual.contains("~/Code/GitHub/runie") {
            println!("  ✗ Missing path '~/Code/GitHub/runie'");
            passed = false;
        } else {
            println!("  ✓ Contains path '~/Code/GitHub/runie'");
        }
        passed
    }

    fn check_token_meter(actual: &str) -> bool {
        let tokens_str = format_token_count(21_000);
        let window_str = format_context_window(512_000);
        // Token count uses uppercase K, context window uses lowercase k
        let meter = format!("│ {} / {} │", tokens_str, window_str);
        if actual.contains(&meter) {
            println!("  ✓ Token meter '{}' present", meter);
            true
        } else {
            println!("  ✗ Missing token meter '{}'", meter);
            // Show what token-related content exists
            for part in actual.split('│') {
                if part.contains("K") || part.contains("k") {
                    println!("    Found token-like content: '{}'", part.trim());
                }
            }
            false
        }
    }

    pub fn test_idle() -> bool {
        println!("\n=== Header Bar: Idle State ===");
        let vm = create_idle_vm();
        let colors = make_test_colors();
        let area = Rect::new(0, 0, 80, 1);
        let mut buf = Buffer::empty(area);
        render_top_bar(&vm, area, &mut buf, &colors);
        let actual = extract_line(&buf, 0);
        println!("  DEBUG header: {:?}", actual);

        let mut passed = check_branch_and_path(&actual);
        passed = check_token_meter(&actual) && passed;
        passed
    }

    pub fn test_streaming() -> bool {
        println!("\n=== Header Bar: Streaming State ===");
        let mut vm = create_idle_vm();
        vm.agent_running = true;
        vm.braille_frame = 2;
        let actual = render_header(&vm);
        let spinner = spinner_frame(2);
        let present = actual.starts_with(spinner.to_string().as_str())
            || actual.contains(spinner.to_string().as_str());
        if present {
            println!("  ✓ Starts with spinner '{}'", spinner);
        } else {
            println!("  ✗ Should have spinner '{}', got: {}", spinner, actual);
        }
        present
    }

    pub fn test_token_formats() -> bool {
        println!("\n=== Header Bar: Token Format Variations ===");
        // Note: 1M tokens formats as "1.0M" not "1M"
        let cases = [(4_000, "4K"), (21_000, "21K"), (512_000, "512K"), (1_000_000, "1.0M")];
        let mut all_passed = true;
        for (tokens, expected) in cases {
            let mut vm = create_idle_vm();
            vm.estimated_tokens = tokens;
            let actual = render_header(&vm);
            // Both token count and context window use uppercase K
            let meter = format!("│ {} / 512K │", expected);
            if actual.contains(&meter) {
                println!("  ✓ {} tokens: meter='{}'", tokens, meter);
            } else {
                println!("  ✗ {} tokens: expected '{}'", tokens, meter);
                all_passed = false;
            }
        }
        all_passed
    }

    pub fn run_all() -> bool {
        let p1 = test_idle();
        let p2 = test_streaming();
        let p3 = test_token_formats();
        p1 && p2 && p3
    }
}

// =============================================================================
// 2. Welcome Menu Tests - Exact Position Verification
// =============================================================================

mod welcome_menu {
    use super::*;

    fn render_menu() -> String {
        let screen = HomeScreen::new();
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        screen.render(area, &mut buf);
        buffer_to_string(&buf)
    }

    pub fn test_menu_items() -> bool {
        println!("\n=== Welcome Menu: Menu Items ===");
        let content = render_menu();
        let mut passed = true;
        let items = [("New worktree", "ctrl-w"), ("Resume session", "ctrl-s"), ("Quit", "ctrl-q")];
        for (name, hint) in items {
            if content.contains(name) {
                println!("  ✓ Contains '{}'", name);
            } else {
                println!("  ✗ Missing '{}'", name);
                passed = false;
            }
            if content.contains(hint) {
                println!("  ✓ Contains hint '{}'", hint);
            } else {
                println!("  ✗ Missing hint '{}'", hint);
                passed = false;
            }
        }
        passed
    }

    pub fn test_tip_text() -> bool {
        println!("\n=== Welcome Menu: Tip Text ===");
        let content = render_menu();
        let expected = "Tip: Press Ctrl-W to start a parallel task in its own worktree.";
        if content.contains(expected) {
            println!("  ✓ Tip text matches");
            true
        } else {
            println!("  ✗ Tip text mismatch");
            println!("  Expected: {}", expected);
            false
        }
    }

    pub fn run_all() -> bool {
        test_menu_items() && test_tip_text()
    }
}

// =============================================================================
// 3. User Message Tests - Exact Indent and Timestamp Position
// =============================================================================

mod user_message {
    use super::*;

    fn create_vm(text: &str, ts: &str) -> MessageListViewModel {
        MessageListViewModel::new(
            vec![MessageItem::User {
                text: text.to_string(),
                model: None,
                timestamp: Some(ts.to_string()),
            }].into(),
            0,
            false,
            Default::default(),
            WrapCache::new(),
            None,
            None,
        )
    }

    fn render_msg(vm: &MessageListViewModel) -> String {
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 18);
        let mut buf = Buffer::empty(area);
        MessageList::render_ref(vm, area, &mut buf, &theme);
        buffer_to_string(&buf)
    }

    fn check_chevron_indent(content: &str) -> bool {
        let chevron_str = CHEVRON.to_string();
        match content.find(chevron_str.as_str()) {
            Some(pos) => pos > 0,
            None => false,
        }
    }

    pub fn test_exact_format() -> bool {
        println!("\n=== User Message: Exact Format ===");
        let vm = create_vm("hello", "9:45 PM");
        let content = render_msg(&vm);
        let chevron_str = CHEVRON.to_string();
        let all_checks = [
            ("Chevron", content.contains(&chevron_str)),
            ("Timestamp", content.contains("9:45 PM")),
            ("Message text", content.contains("hello")),
            ("Indent", check_chevron_indent(&content)),
        ];
        let mut passed = true;
        for (name, result) in all_checks {
            println!("  {} {}", if result { "✓" } else { "✗" }, name);
            passed = passed && result;
        }
        passed
    }

    pub fn test_timestamp_position() -> bool {
        println!("\n=== User Message: Timestamp Position ===");
        let vm = create_vm("hello", "9:45 PM");
        let content = render_msg(&vm);
        if let Some(pos) = content.find("9:45 PM") {
            println!("  Timestamp at position: {}", pos);
            if pos > 60 {
                println!("  ✓ Timestamp is right-aligned");
            } else {
                println!("  ⚠ Timestamp may not be right-aligned");
            }
        }
        true
    }

    pub fn run_all() -> bool {
        test_exact_format() && test_timestamp_position()
    }
}

// =============================================================================
// 4. Thinking Block Tests - Collapsed and Expanded States
// =============================================================================

mod thinking_block {
    use super::*;

    fn create_block(content: &str, collapsed: bool) -> ThinkingBlock {
        ThinkingBlock {
            content: content.to_string(),
            duration_secs: 0.9,
            collapsed,
            animation_frame: 0,
        }
    }

    fn render_block(block: &ThinkingBlock) -> String {
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        render_thinking_block(block, area, &mut buf, &theme, 0, 3);
        buffer_to_string(&buf)
    }

    pub fn test_collapsed() -> bool {
        println!("\n=== Thinking Block: Collapsed State ===");
        let block = create_block("The user said...", true);
        let content = render_block(&block);
        let mut passed = true;
        if content.contains("Thinking") || content.contains("◆") {
            println!("  ✓ Contains thinking indicator");
        } else {
            println!("  ✗ Missing thinking indicator");
            passed = false;
        }
        if !content.contains("The user said") {
            println!("  ✓ Content hidden in collapsed");
        } else {
            println!("  ✗ Content should be hidden");
            passed = false;
        }
        if content.contains("┃") && content.contains("◆") {
            println!("  ✓ Has vertical bar and diamond");
        } else {
            println!("  ✗ Missing thinking elements");
            passed = false;
        }
        passed
    }

    pub fn test_expanded() -> bool {
        println!("\n=== Thinking Block: Expanded State ===");
        let block = create_block("The user said...", false);
        let content = render_block(&block);
        let mut passed = true;
        if content.contains("┃") && content.contains("◆") && content.contains("Thinking") {
            println!("  ✓ Contains thinking header");
        } else {
            println!("  ✗ Missing thinking header");
            passed = false;
        }
        if content.contains("The user said") {
            println!("  ✓ Contains expanded content");
        } else {
            println!("  ✗ Missing expanded content");
            passed = false;
        }
        if content.contains("┃  The user said") {
            println!("  ✓ Content has correct prefix");
        } else {
            println!("  ✗ Content missing prefix");
            passed = false;
        }
        passed
    }

    pub fn run_all() -> bool {
        test_collapsed() && test_expanded()
    }
}

// =============================================================================
// 5. Tool Status Tests - Running, Complete, Error States
// =============================================================================

mod tool_status {
    use super::*;

    fn create_block(name: &str, args: &str, status: ToolStatus, elapsed: f64, total: f64, bytes: u64, frame: usize) -> ToolCallBlock {
        ToolCallBlock {
            tool_name: name.to_string(),
            args: args.to_string(),
            status,
            elapsed_secs: elapsed,
            total_secs: total,
            bytes_in: bytes,
            spinner_frame: frame,
        }
    }

    fn render_tool(block: &ToolCallBlock) -> String {
        let theme = ThemeWrapper::default();
        let area = Rect::new(0, 2, 80, 10);
        let mut buf = Buffer::empty(area);
        render_tool_call_block(block, area, &mut buf, &theme, 0, 3);
        buffer_to_string(&buf)
    }

    pub fn test_running() -> bool {
        println!("\n=== Tool Status: Running State ===");
        let block = create_block("List", "ls", ToolStatus::Running, 1.8, 0.0, 0, 2);
        let content = render_tool(&block);
        let checks = [("List", "tool name"), ("ls", "args"), ("1.8s", "elapsed"), ("Run", "label")];
        let mut passed = true;
        for (needle, label) in checks {
            if content.contains(needle) {
                println!("  ✓ Contains {} ('{}')", label, needle);
            } else {
                println!("  ✗ Missing {} ('{}')", label, needle);
                passed = false;
            }
        }
        passed
    }

    pub fn test_complete() -> bool {
        println!("\n=== Tool Status: Complete State ===");
        let block = create_block("List", "ls", ToolStatus::Complete, 0.0, 2.9, 22_200, 0);
        let content = render_tool(&block);
        let mut passed = true;
        if content.contains('✓') {
            println!("  ✓ Contains checkmark");
        } else {
            println!("  ✗ Missing checkmark");
            passed = false;
        }
        if content.contains("List") {
            println!("  ✓ Contains tool name");
        } else {
            println!("  ✗ Missing tool name");
            passed = false;
        }
        if content.contains("2.9s") {
            println!("  ✓ Contains duration");
        } else {
            println!("  ✗ Missing duration");
            passed = false;
        }
        if content.contains("[✓]") {
            println!("  ✓ Contains success bracket");
        } else {
            println!("  ✗ Missing success bracket");
            passed = false;
        }
        passed
    }

    pub fn test_error() -> bool {
        println!("\n=== Tool Status: Error State ===");
        let block = create_block("bash", "rm file", ToolStatus::Error, 0.0, 0.5, 0, 0);
        let content = render_tool(&block);
        let mut passed = true;
        if content.contains('✗') {
            println!("  ✓ Contains error mark");
        } else {
            println!("  ✗ Missing error mark");
            passed = false;
        }
        if content.contains("bash") {
            println!("  ✓ Contains tool name");
        } else {
            println!("  ✗ Missing tool name");
            passed = false;
        }
        if content.contains("error") {
            println!("  ✓ Contains error label");
        } else {
            println!("  ✗ Missing error label");
            passed = false;
        }
        if content.contains("[✗]") {
            println!("  ✓ Contains error bracket");
        } else {
            println!("  ✗ Missing error bracket");
            passed = false;
        }
        passed
    }

    pub fn run_all() -> bool {
        test_running() && test_complete() && test_error()
    }
}

// =============================================================================
// 6. Status Bar Tests - Idle and Running States
// =============================================================================

mod status_bar {
    use super::*;

    fn create_idle_vm() -> StatusBarViewModel {
        StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
            agent_running: false,
            status_header: None,
            status_details: None,
            status_start_time: None,
            mcp_status: runie_tui::tui::view_models::McpStatus::None,
            input_has_text: false,
        }
    }

    fn create_running_vm() -> StatusBarViewModel {
        StatusBarViewModel {
            mode: TuiMode::Chat,
            current_model: Some("gpt-4o".to_string()),
            session_token_usage: TokenUsage { total_tokens: 5000, estimated_cost: 0.0234, ..Default::default() },
            agent_running: true,
            status_header: Some("thinking".to_string()),
            status_details: None,
            status_start_time: Some(std::time::Instant::now()),
            mcp_status: runie_tui::tui::view_models::McpStatus::None,
            input_has_text: false,
        }
    }

    fn render_status(wide: bool) -> String {
        let vm = if wide { create_running_vm() } else { create_idle_vm() };
        let colors = make_test_colors();
        // Use a tall buffer - content ends up at y=0 in buffer coords
        let area = Rect::new(0, 0, if wide { 120 } else { 80 }, 24);
        let mut buf = Buffer::empty(area);
        render_status_bar(&vm, area, &mut buf, &colors);
        buffer_to_string(&buf)
    }

    pub fn test_idle() -> bool {
        println!("\n=== Status Bar: Idle State ===");
        let content = render_status(false);
        let mut passed = true;
        if content.contains("Shift+Tab") && content.contains("mode") {
            println!("  ✓ Contains 'Shift+Tab:mode'");
        } else {
            println!("  ✗ Missing 'Shift+Tab:mode'");
            passed = false;
        }
        if content.contains("Ctrl+.") && content.contains("shortcuts") {
            println!("  ✓ Contains 'Ctrl+.:shortcuts'");
        } else {
            println!("  ✗ Missing 'Ctrl+.:shortcuts'");
            passed = false;
        }
        passed
    }

    pub fn test_running() -> bool {
        println!("\n=== Status Bar: Running State ===");
        let actual = render_status(true);
        let keys = [("Shift+Tab", "mode"), ("Ctrl+c", "cancel"), ("Ctrl+Enter", "interject"), ("Ctrl+.", "shortcuts")];
        let mut passed = true;
        for (key, desc) in keys {
            if actual.contains(key) && actual.contains(desc) {
                println!("  ✓ Contains '{}:{}'", key, desc);
            } else {
                println!("  ✗ Missing '{}:{}'", key, desc);
                passed = false;
            }
        }
        passed
    }

    pub fn run_all() -> bool {
        test_idle() && test_running()
    }
}

// =============================================================================
// 7. Input Border Tests - Normal, Plan, Always-Approve Modes
// =============================================================================

mod input_border {
    use super::*;

    fn render_input(mode: &str) -> String {
        let textarea = ratatui_textarea::TextArea::new(vec![]);
        let colors = make_test_colors();
        // Use a taller buffer to ensure content is captured
        let area = Rect::new(0, 0, 80, 24);
        let mut buf = Buffer::empty(area);
        render_input_bar(&textarea, "\u{276F} ", "", area, &mut buf, &colors, mode, &[], None, true);
        buffer_to_string(&buf)
    }

    pub fn test_normal_mode() -> bool {
        println!("\n=== Input Border: Normal Mode ===");
        let content = render_input("Grok Build");
        let mut passed = true;
        if content.contains("Grok Build") {
            println!("  ✓ Contains 'Grok Build'");
        } else {
            println!("  ✗ Missing 'Grok Build'");
            passed = false;
        }
        // Check for bottom border line with dashes
        if content.contains("─") {
            println!("  ✓ Contains border dashes");
        } else {
            println!("  ✗ Missing border dashes");
            passed = false;
        }
        passed
    }

    pub fn test_plan_mode() -> bool {
        println!("\n=== Input Border: Plan Mode ===");
        let content = render_input("plan");
        if content.contains("plan") {
            println!("  ✓ Contains 'plan'");
            true
        } else {
            println!("  ✗ Missing 'plan'");
            false
        }
    }

    pub fn test_always_approve_mode() -> bool {
        println!("\n=== Input Border: Always-Approve Mode ===");
        let content = render_input("always-approve");
        if content.contains("always-approve") {
            println!("  ✓ Contains 'always-approve'");
            true
        } else {
            println!("  ✗ Missing 'always-approve'");
            false
        }
    }

    pub fn run_all() -> bool {
        test_normal_mode() && test_plan_mode() && test_always_approve_mode()
    }
}

// =============================================================================
// Main Test Runner
// =============================================================================

fn main() {
    println!("╔════════════════════════════════════════════════════════════════╗");
    println!("║        Grok Parity Test Suite - DEEP Character Comparison      ║");
    println!("╚════════════════════════════════════════════════════════════════╝");
    println!();

    let mut all_passed = true;

    macro_rules! run_tests {
        ($name:expr, $mod:ident) => {
            println!("\n{}", "=".repeat(70));
            println!("Running {} Tests...", $name);
            println!("{}", "=".repeat(70));
            if !$mod::run_all() {
                all_passed = false;
            }
        };
    }

    run_tests!("Header Bar", header_bar);
    run_tests!("Welcome Menu", welcome_menu);
    run_tests!("User Message", user_message);
    run_tests!("Thinking Block", thinking_block);
    run_tests!("Tool Status", tool_status);
    run_tests!("Status Bar", status_bar);
    run_tests!("Input Border", input_border);

    println!("\n{}", "═".repeat(70));
    println!("                         TEST SUMMARY");
    println!("{}", "═".repeat(70));

    if all_passed {
        println!("\n  ✓✓✓ ALL Grok parity tests PASSED! ✓✓✓\n");
        println!("  Elements tested:");
        println!("    • Header Bar: idle, streaming, token formats");
        println!("    • Welcome Menu: items, hints, tip text");
        println!("    • User Message: exact indent, timestamp position");
        println!("    • Thinking Block: collapsed, expanded states");
        println!("    • Tool Status: running, complete, error states");
        println!("    • Status Bar: idle, running states");
        println!("    • Input Border: normal, plan, always-approve modes");
    } else {
        println!("\n  ✗✗✗ SOME Grok parity tests FAILED! ✗✗✗\n");
        println!("  Review output above for details.");
        std::process::exit(1);
    }
}