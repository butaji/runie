//! Integration tests for doom loop detection (Task 28).
//!
//! These tests verify the DoomLoopDetector correctly identifies repeating
//! tool call patterns and signals doom loops.

use runie_patterns::doom_loop::{DoomLoopDetector, DEFAULT_DOOM_LOOP_THRESHOLD};

/// Test: Detector triggers at default threshold
#[test]
fn triggers_at_default_threshold() {
    let mut detector = DoomLoopDetector::with_default_threshold();
    
    // Should not trigger before threshold
    for _ in 0..(DEFAULT_DOOM_LOOP_THRESHOLD - 1) {
        let signal = detector.check("read_file");
        assert!(signal.is_none(), "Should not trigger before threshold");
    }
    
    // Should trigger at threshold
    let signal = detector.check("read_file");
    assert!(signal.is_some());
    assert_eq!(signal.as_ref().unwrap().repetition_count, DEFAULT_DOOM_LOOP_THRESHOLD);
}

/// Test: Different tool resets the streak
#[test]
fn different_tool_resets_streak() {
    let mut detector = DoomLoopDetector::new(3);
    
    detector.check("read_file");
    detector.check("read_file");
    
    // Switch to different tool — streak resets for bash
    let signal = detector.check("bash");
    assert!(signal.is_none(), "Should reset streak for different tool");
    
    // Need 3 bash calls total to trigger after reset
    detector.check("bash");                          // streak=2, no signal
    let signal = detector.check("bash");              // streak=3, triggers
    assert!(signal.is_some());
    assert_eq!(signal.as_ref().unwrap().tool_name, "bash");
}

/// Test: Signal contains correct information
#[test]
fn signal_contains_correct_info() {
    let mut detector = DoomLoopDetector::new(2);
    
    detector.check("grep");
    let signal = detector.check("grep");
    
    let signal = signal.unwrap();
    assert_eq!(signal.tool_name, "grep");
    assert_eq!(signal.repetition_count, 2);
    assert!(signal.message.contains("grep"));
    assert!(signal.message.contains("2"));
}

/// Test: Reset clears state
#[test]
fn reset_clears_state() {
    let mut detector = DoomLoopDetector::new(2);
    
    // Build up to near threshold
    detector.check("read_file");
    assert_eq!(detector.streak(), 1);
    
    // Reset
    detector.reset();
    
    assert_eq!(detector.streak(), 0);
    assert!(!detector.is_looping());
    assert!(detector.check("read_file").is_none());
}

/// Test: is_looping helper
#[test]
fn is_looping_helper() {
    let mut detector = DoomLoopDetector::new(3);
    
    assert!(!detector.is_looping());
    
    detector.check("read_file");
    assert!(!detector.is_looping());
    
    detector.check("read_file");
    assert!(!detector.is_looping());
    
    detector.check("read_file");
    assert!(detector.is_looping());
    
    // Different tool should reset
    detector.check("bash");
    assert!(!detector.is_looping());
}

/// Test: History tracks recent calls
#[test]
fn history_tracks_recent_calls() {
    let mut detector = DoomLoopDetector::new(5);
    
    detector.check("read_file");
    detector.check("read_file");
    detector.check("bash");
    detector.check("grep");
    
    let history = detector.history();
    assert_eq!(history.len(), 4);
    assert_eq!(history[0], "read_file");
    assert_eq!(history[1], "read_file");
    assert_eq!(history[2], "bash");
    assert_eq!(history[3], "grep");
}

/// Test: Custom threshold works
#[test]
fn custom_threshold_works() {
    let mut detector = DoomLoopDetector::new(1);
    
    // With threshold 1, should trigger immediately
    let signal = detector.check("any_tool");
    assert!(signal.is_some());
    assert_eq!(signal.unwrap().repetition_count, 1);
}

/// Test: Zero threshold defaults to 1
#[test]
fn zero_threshold_defaults_to_one() {
    let detector = DoomLoopDetector::new(0);
    assert_eq!(detector.threshold(), 1);
}

/// Test: Integration with PatternConfig doom_loop_threshold
#[test]
fn doom_loop_threshold_integration() {
    use runie_patterns::PatternConfig;
    
    let config = PatternConfig::default();
    assert_eq!(config.doom_loop_threshold, DEFAULT_DOOM_LOOP_THRESHOLD);
    
    // Custom config
    let config = PatternConfig {
        doom_loop_threshold: 10,
        ..Default::default()
    };
    assert_eq!(config.doom_loop_threshold, 10);
}
