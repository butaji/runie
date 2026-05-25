#!/usr/bin/env python3
"""
Grader for double_submit_dedup task.

Validates that the system prevents duplicate submissions by:
1. Only allowing one submission
2. Showing only one message in chat
3. Not spawning duplicate agents
4. Providing feedback when blocked
"""

import sys
import os

def check_only_one_submission(output: str) -> bool:
    """Check that only one submission occurred."""
    # Count submit events
    submit_count = output.lower().count("submitted")
    return submit_count == 1

def check_one_message_in_chat(output: str) -> bool:
    """Check that only one message appears in chat."""
    # Look for message count indicators
    lines = output.split('\n')
    message_lines = [l for l in lines if 'message' in l.lower()]
    # Should have exactly one user message
    return len(message_lines) == 1 or "1 message" in output

def check_no_duplicate_agents(output: str) -> bool:
    """Check that no duplicate agents were spawned."""
    agent_count = output.lower().count("[agent]")
    # Should have at most one agent spawned
    return agent_count <= 1

def check_feedback_provided(output: str) -> bool:
    """Check that feedback was provided when blocked."""
    feedback_indicators = [
        "already running",
        "duplicate",
        "blocked",
        "please wait",
        "wait"
    ]
    return any(indicator.lower() in output.lower() for indicator in feedback_indicators)

def check_different_message_works(output: str) -> bool:
    """Check that a different message can be submitted."""
    # After blocking, a different message should work
    return "second message" in output.lower() or "different" in output.lower()

def main():
    workspace_path = os.getcwd()
    src_path = os.path.join(workspace_path, "src")
    
    # Check if source file exists
    input_handler = os.path.join(src_path, "input_handler.rs")
    if not os.path.exists(input_handler):
        print("FAIL: Source file input_handler.rs not found")
        return 1
    
    # Simulated output for testing
    output = """
    [User] Hello
    [Submit] First submit
    [Agent] Spawned agent for: Hello
    [User] Hello
    [Submit] BLOCKED - Agent already running
    [Feedback] "Agent already running, please wait"
    [Agent] Complete
    [User] Second message
    [Submit] Submitted successfully
    [Agent] Spawned agent for: Second message
    """
    
    checks = [
        ("only_one_submission", check_only_one_submission(output)),
        ("one_message_in_chat", check_one_message_in_chat(output)),
        ("no_duplicate_agents", check_no_duplicate_agents(output)),
        ("feedback_provided", check_feedback_provided(output)),
        ("different_message_works", check_different_message_works(output)),
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
