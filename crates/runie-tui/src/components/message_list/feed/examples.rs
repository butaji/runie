//! Declarative Feed API - Builder Pattern & Examples
//!
//! This module demonstrates the ideal developer experience for working with Feed.
//! The API is designed to be:
//! - **Declarative**: Describe what you want, not how to build it
//! - **Chainable**: Fluent builder pattern for readable test/fixture code
//! - **Type-safe**: Compile-time enforcement of valid conversation structure
//!
//! # Core Philosophy
//!
//! - UserMessage always precedes AssistantMessage (enforced by builder)
//! - Thoughts and ToolCalls attach to AssistantMessage (not separate items)
//! - Streaming content uses mutable append operations
//!
//! # Example 1: Simple Conversation
//!
//! ```
//! use std::time::Duration;
//! use runie_tui::components::message_list::feed::{Feed, FeedBuilder};
//!
//! let feed = Feed::builder()
//!     .user_message("Hello!")
//!     .assistant()
//!         .thinking_for(Duration::from_millis(200))
//!         .say("Hi there! How can I help?")
//!         .turn_completed_in(Duration::from_secs(1))
//!     .build();
//! ```
//!
//! # Example 2: With Tool Calls
//!
//! ```
//! use std::time::Duration;
//! use runie_tui::components::message_list::feed::{Feed, FeedBuilder};
//! use serde_json::json;
//!
//! let feed = Feed::builder()
//!     .user_message("List files")
//!     .assistant()
//!         .thinking_for(Duration::from_millis(500))
//!         .tool_call("bash", json!({"command": "ls -la"}))
//!         .thinking_for(Duration::from_millis(200))
//!         .say("Here are the files:")
//!         .code_block("file1.rs\nfile2.rs")
//!         .turn_completed_in(Duration::from_secs(3))
//!     .build();
//! ```
//!
//! # Example 3: Multi-turn Conversation
//!
//! ```
//! use std::time::Duration;
//! use runie_tui::components::message_list::feed::{Feed, FeedBuilder};
//!
//! let feed = Feed::builder()
//!     .user_message("What files are here?")
//!     .assistant()
//!         .thinking_for(Duration::from_millis(300))
//!         .say("Let me check...")
//!         .tool_call("bash", serde_json::json!({"command": "ls"}))
//!         .say("Found 3 files.")
//!         .turn_completed_in(Duration::from_secs(2))
//!     .user_message("Show me the first one")
//!     .assistant()
//!         .thinking_for(Duration::from_millis(100))
//!         .say("Here's file1.rs:")
//!         .code_block("fn main() {}")
//!         .turn_completed_in(Duration::from_secs(1))
//!     .build();
//! ```
//!
//! # Example 4: Streaming (Mutable)
//!
//! ```
//! use std::time::Duration;
//! use runie_tui::components::message_list::feed::Feed;
//!
//! let mut feed = Feed::new();
//!
//! // Start with user message
//! feed.add_user_message("Hello".to_string());
//!
//! // Begin streaming assistant response
//! feed.add_assistant_message();
//! feed.add_thought(1.0);
//!
//! // Stream text incrementally
//! feed.append_to_last("Hello");
//! feed.append_to_last(" world");
//! feed.complete_turn(2.0);
//! ```
//!
//! # Example 5: Code Block Helpers
//!
//! ```
//! use std::time::Duration;
//! use runie_tui::components::message_list::feed::FeedBuilder;
//!
//! let feed = FeedBuilder::new()
//!     .user_message("Show me a Rust example")
//!     .assistant()
//!         .thinking_for(Duration::from_millis(200))
//!         .code_block_with_lang("rust", r#"
//! fn main() {
//!     println!("Hello!");
//! }
//! "#)
//!         .say("Here's a simple Rust program.")
//!         .turn_completed_in(Duration::from_secs(1))
//!     .build();
//! ```
//!
//! # Example 6: Fixture Factory Pattern
//!
//! For tests and fixtures, create reusable builders:
//!
//! ```
//! use std::time::Duration;
//! use runie_tui::components::message_list::feed::FeedBuilder;
//!
//! // Reusable fixture factory
//! fn make_thinking_assistant() -> impl FeedChainable {
//!     FeedBuilder::new()
//!         .user_message("Prompt")
//!         .assistant()
//!             .thinking_for(Duration::from_secs(1))
//!             .say("Thinking response...")
//! }
//! ```

pub mod builder;
pub mod examples_tests;

pub use builder::{AgentEvent, FeedBuilder, FeedChainable};
