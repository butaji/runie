#!/usr/bin/env python3
"""
Grader for stream_error_partial_response task.

Validates that stream errors are handled gracefully:
1. Error event is sent to UI
2. Partial content is displayed
3. Message is marked as error
4. User can continue
"""
import sys
import re
from pathlib import Path

# Project root is the working directory when this script runs
PROJECT_ROOT = Path.cwd()

def check_file_content(relative_path):
    """Check if a file exists and return its content."""
    full_path = PROJECT_ROOT / relative_path
    if full_path.exists():
        return full_path.read_text()
    return ""

def check_stream_error_handling():
    checks = {
        "error_event_sent": False,
        "partial_content_displayed": False,
        "message_marked_error": False,
        "user_can_continue": False,
    }

    # Check rig_loop.rs for stream error handling
    rig_loop_content = check_file_content("crates/runie-agent/src/rig_loop.rs")
    if rig_loop_content:
        # Check for error event on stream failure
        if re.search(r'event_tx.*Error|AgentEvent::Error', rig_loop_content):
            checks["error_event_sent"] = True

        # Check for partial content preservation
        if "text_content" in rig_loop_content or "partial" in rig_loop_content.lower():
            checks["partial_content_displayed"] = True

    # Also check loop_engine.rs
    loop_engine_content = check_file_content("crates/runie-agent/src/loop_engine.rs")
    if loop_engine_content:
        if "Error" in loop_engine_content and "event_tx" in loop_engine_content:
            checks["error_event_sent"] = True
        if "partial" in loop_engine_content.lower() or "text_content" in loop_engine_content:
            checks["partial_content_displayed"] = True

    # Check agent.rs for error message handling
    agent_update_content = check_file_content("crates/runie-tui/src/tui/update/agent.rs")
    if agent_update_content:
        # Check for error state handling
        if re.search(r'on_agent_error|error_message', agent_update_content):
            checks["message_marked_error"] = True

        # Check for continuation
        if "agent_running" in agent_update_content and "false" in agent_update_content:
            checks["user_can_continue"] = True

    # Check message types for error variant
    message_types_content = check_file_content("crates/runie-tui/src/components/message_list/types.rs")
    if message_types_content:
        if "Error" in message_types_content or "error" in message_types_content:
            checks["message_marked_error"] = True

    # Check events.rs for error event definition
    events_content = check_file_content("crates/runie-agent/src/events.rs")
    if events_content:
        if "Error" in events_content and "enum AgentEvent" in events_content:
            checks["error_event_sent"] = True

    return checks


def main():
    print("Checking stream error partial response handling...\n")

    checks = check_stream_error_handling()

    passed = 0
    total = len(checks)

    for check_name, result in checks.items():
        status = "PASS" if result else "FAIL"
        print(f"{status}: {check_name}")
        if result:
            passed += 1

    print(f"\n{passed}/{total} checks passed")

    if passed >= 3:
        print("RESULT: pass")
        sys.exit(0)
    else:
        print("RESULT: fail")
        sys.exit(1)


if __name__ == "__main__":
    main()
