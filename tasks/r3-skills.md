# Skills System

**Status**: todo
**Milestone**: R3
**Category**: Core Architecture

## Description

Load SKILL.md files from user/project directories. Skills are self-describing interceptors that can inject context, modify tool calls, or preprocess input.

## Architecture

```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub file_path: PathBuf,
    pub user_invocable: bool,
    pub interceptors: Vec<Interceptor>,
}

pub enum Interceptor {
    PreProcessInput,    // Modify user input before sending
    PostProcessOutput,  // Modify agent output before display
    InjectContext,      // Add context to system prompt
    ModifyToolCall,     // Transform tool calls
}

pub struct SkillLoader {
    user_dir: PathBuf,    // ~/.runie/skills/
    project_dir: PathBuf, // ./.runie/skills/
}

impl SkillLoader {
    pub fn load_all() -> Vec<Skill>;
    pub fn load_from_dir(dir: &Path) -> Vec<Skill>;
}
```

### Skill File Format (SKILL.md)

```markdown
# My Skill

## Description

Brief description of what this skill does.

## Invocation

User can invoke with `/skill my-skill` or it auto-triggers on file patterns.

## Context

Additional system prompt context injected when active.
```

## Acceptance Criteria

- [ ] Load skills from `~/.runie/skills/*.md`
- [ ] Load skills from `./.runie/skills/*.md`
- [ ] Skills inject context into system prompt
- [ ] User-invocable skills available in command palette
- [ ] `/skills` lists loaded skills
- [ ] Skills reload with `/reload`

## Tests

### Layer 1
- [ ] `load_skills_from_dir` — parses SKILL.md files
- [ ] `skill_injects_context` — context added to system prompt
- [ ] `user_invocable_shown_in_palette` — appears in commands
