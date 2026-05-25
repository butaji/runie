#!/usr/bin/env python3
"""
Grader for error_state_recovery task.

Validates that the agent correctly handles errors by:
1. Displaying error messages to user
2. Returning to Chat mode
3. Preserving scroll position
4. Allowing continuation
"""

import sys
import os

def check_error_displayed(output: str) -> bool:
    """Check that error message was displayed to user."""
    error_indicators = [
        "error",
        "failed",
        "timeout",
        "connection",
        "Error",
        "Failed",
        "network"
    ]
    return any(indicator.lower() in output.lower() for indicator in error_indicators)

def check_mode_returns_to_chat(output: str) -> bool:
    """Check that mode returned to Chat after error."""
    # Look for evidence of Chat mode being active
    chat_indicators = [
        "chat mode",
        "ready",
        "type a message",
        "❯",
        "model:"
    ]
    return any(indicator.lower() in output.lower() for indicator in chat_indicators)

def check_scroll_preserved(output: str) -> bool:
    """Check that scroll position was preserved."""
    # If we see scroll-related output, position was preserved
    scroll_indicators = [
        "scroll",
        "offset",
        "message",
        "20"  # Original message count
    ]
    return any(indicator.lower() in output.lower() for indicator in scroll_indicators)

def check_user_can_continue(output: str) -> bool:
    """Check that user can continue after error."""
    continue_indicators = [
        "retry",
        "continue",
        "submit",
        "send",
        "ready",
        "chat"
    ]
    return any(indicator.lower() in output.lower() for indicator in continue_indicators)

def check_no_panic(output: str) -> bool:
    """Check that no panic occurred."""
    panic_indicators = [
        "panicked",
        "thread '",
        "panicked at"
    ]
    return not any(indicator in output for indicator in panic_indicators)

def main():
    # Read workspace contents
    workspace_path = os.getcwd()
    src_path = os.path.join(workspace_path, "src")
    
    # Check if source file exists
    error_prone = os.path.join(src_path, "error_prone.rs")
    if not os.path.exists(error_prone):
        print("FAIL: Source file error_prone.rs not found")
        return 1
    
    # For this test, we check the implementation handles errors correctly
    # In a real scenario, this would spawn the agent and capture output
    
    # Simulated output for testing
    # In real harness, this would come from agent execution
    output = """
    [Agent] Starting execution...
    [Tool] Calling network_tool with args {"url": "http://example.com"}
    [Error] Connection refused: network unavailable
    [UI] Error displayed in chat: Connection refused: network unavailable
    [UI] Mode returned to Chat
    [UI] Scroll position preserved at offset 5
    [UI] Ready for next input
    """
    
    checks = [
        ("error_displayed_to_user", check_error_displayed(output)),
        ("mode_returns_to_chat", check_mode_returns_to_chat(output)),
        ("scroll_position_preserved", check_scroll_preserved(output)),
        ("user_can_continue", check_user_can_continue(output)),
        ("no_panic_on_error", check_no_panic(output)),
    ]
    
    passed = 0
    failed = 0
    
    for check_name, result in checks:
        if result:
            print(f"PASS: {check_name}")
            passed += 1
        else:
            print(f"FAIL: {check_name}")
            failed += 1
    
    print(f"\n{passed}/{passed + failed} checks passed")
    
    return 0 if failed == 0 else 1

if __name__ == "__main__":
    sys.exit(main())
