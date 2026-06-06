//! Tests for runie-agent
use crate::{AgentCommand, MockProvider, Provider, needs_tool_execution, get_fake_file_list};

#[test]
fn test_mock_provider_generates_chunks() {
    let provider = MockProvider;
    let messages = vec![crate::Message::User { content: "Hello World".to_string() }];
    let chunks = provider.generate(messages);
    
    assert_eq!(chunks.len(), 2);
    assert_eq!(chunks[0].content, "Hello ");
    assert_eq!(chunks[1].content, "World ");
}

#[test]
fn test_mock_provider_empty_input() {
    let provider = MockProvider;
    let messages = vec![];
    let chunks = provider.generate(messages);
    
    assert!(chunks.is_empty());
}

#[test]
fn test_mock_provider_single_word() {
    let provider = MockProvider;
    let messages = vec![crate::Message::User { content: "Hello".to_string() }];
    let chunks = provider.generate(messages);
    
    assert_eq!(chunks.len(), 1);
    assert_eq!(chunks[0].content, "Hello ");
}

#[test]
fn test_agent_command_structure() {
    let cmd = AgentCommand {
        content: "test".to_string(),
        id: "req.0".to_string(),
    };
    
    assert_eq!(cmd.content, "test");
    assert_eq!(cmd.id, "req.0");
}

#[test]
fn test_needs_tool_execution_list_files() {
    assert!(needs_tool_execution("list files"), "Should trigger for 'list files'");
    assert!(needs_tool_execution("List Files"), "Should trigger for 'List Files'");
    assert!(needs_tool_execution("LIST FILES"), "Should trigger for 'LIST FILES'");
    assert!(needs_tool_execution("please list files"), "Should trigger with prefix");
    assert!(needs_tool_execution("list files now"), "Should trigger with suffix");
}

#[test]
fn test_needs_tool_execution_other() {
    assert!(!needs_tool_execution("hello"), "Should NOT trigger for 'hello'");
    assert!(!needs_tool_execution("list"), "Should NOT trigger for 'list' only");
    assert!(!needs_tool_execution("files"), "Should NOT trigger for 'files' only");
}

#[test]
fn test_get_fake_file_list() {
    let files = get_fake_file_list();
    assert!(!files.is_empty(), "Should return file list");
    assert!(files.contains("src/"), "Should contain src/ directory");
    assert!(files.contains("main.rs"), "Should contain main.rs");
    assert!(files.contains("Cargo.toml"), "Should contain Cargo.toml");
    assert!(files.contains("tests/"), "Should contain tests/ directory");
}

#[test]
fn test_list_files_triggers_tool_flow() {
    // This test verifies the "list files" command triggers tool execution
    let content = "list files";
    assert!(needs_tool_execution(content), "'list files' should trigger tool flow");
    
    // Verify file list is returned
    let files = get_fake_file_list();
    assert!(files.contains("src/"));
    assert!(files.contains("main.rs"));
}
