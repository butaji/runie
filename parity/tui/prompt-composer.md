# Prompt composer

## Anatomy

Border, model caption, input body, cursor, multiline indicator, history/file
search affordances, and mode hints.

## States

Normal, alternate, plan, multiline, history, file-search, command completion,
and empty/typed.

## Reference

Grok: `src/views/prompt_widget/` and `src/app/agent_view/prompt.rs`.
Runie: `widgets/prompt.rs` and the prompt actor.
