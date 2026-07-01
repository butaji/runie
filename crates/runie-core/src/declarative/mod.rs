//! Declarative Configuration Loader
//!
//! Loads configuration from markdown and YAML files with YAML frontmatter.
//! This module provides a generic loader that handles:
//!
//! - **Skills**: Markdown files (`.md`) with frontmatter declaring name, description, triggers.
//! - **Commands**: YAML files (`.yaml`) with frontmatter declaring name, description, intent.
//!
//! # File Formats
//!
//! ## Skill (`.md` with frontmatter)
//! ```markdown
//! ---
//! name: check-work
//! description: Verify changes with a subagent.
//! triggers:
//!   - command: /check-work
//!   - command: /verify
//! ---
//!
//! ## Usage
//!
//! `/check-work [focus area]`
//! ```
//!
//! ## Command (`.yaml`)
//! ```yaml
//! name: bookmark
//! description: Bookmark the current assistant message.
//! intent: BookmarkMessage
//! shortcut: Ctrl+b
//! category: Session
//! ```

pub mod loader;
#[cfg(test)]
mod tests;
pub mod types;

pub use loader::{load_commands_from_dir, load_skills_from_dir, DeclarativeLoader};
pub use types::{CommandDef, SkillDef, Trigger};
