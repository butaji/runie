//! Tests for runie-agent
use crate::{AgentCommand, MockProvider, Provider};

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
