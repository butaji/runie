//! Declarative workflow DSL for Team mode orchestration.
//!
//! Syntax:
//!   /workflow "Task description" as alias
//!   /workflow ["Task A" as a, "Task B" as b]
//!   /workflow <tasks> --synthesize "prompt"
//!   /workflow <tasks> --template "template"

use std::fmt;

use crate::orchestrator::SynthesisConfig;

/// A single task in a workflow definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowTask {
    pub description: String,
    pub alias: String,
}

impl WorkflowTask {
    pub fn new(description: impl Into<String>, alias: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            alias: alias.into(),
        }
    }
}

/// Parsed `/workflow` command definition.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkflowDefinition {
    pub tasks: Vec<WorkflowTask>,
    pub synthesis: SynthesisConfig,
}

/// Parse the argument string of a `/workflow` command.
pub fn parse_workflow_args(input: &str) -> Result<WorkflowDefinition, String> {
    let tokens = tokenize(input)?;
    parse_tokens(&tokens)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    String(String),
    Word(String),
    As,
    Comma,
    LBracket,
    RBracket,
    Synthesize,
    Template,
}

fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if c.is_whitespace() {
            continue;
        }
        match c {
            '[' => tokens.push(Token::LBracket),
            ']' => tokens.push(Token::RBracket),
            ',' => tokens.push(Token::Comma),
            '"' | '\'' => tokens.push(read_string(&mut chars, c)?),
            _ => tokens.push(read_word(&mut chars, c)?),
        }
    }
    Ok(tokens)
}

fn read_string(
    chars: &mut std::iter::Peekable<std::str::CharIndices>,
    quote: char,
) -> Result<Token, String> {
    let mut value = String::new();
    loop {
        match chars.next() {
            Some((_, ch)) if ch == quote => return Ok(Token::String(value)),
            Some((_, ch)) => value.push(ch),
            None => return Err(format!("unclosed string literal: missing {}", quote)),
        }
    }
}

fn read_word(
    chars: &mut std::iter::Peekable<std::str::CharIndices>,
    first: char,
) -> Result<Token, String> {
    let mut word = String::new();
    word.push(first);
    while let Some(&(_, ch)) = chars.peek() {
        if ch.is_whitespace() || is_delimiter(ch) {
            break;
        }
        word.push(ch);
        chars.next();
    }
    Ok(classify_word(&word))
}

fn is_delimiter(ch: char) -> bool {
    matches!(ch, '[' | ']' | ',' | '"' | '\'')
}

fn classify_word(word: &str) -> Token {
    match word {
        "as" => Token::As,
        "--synthesize" => Token::Synthesize,
        "--template" => Token::Template,
        _ => Token::Word(word.to_string()),
    }
}

fn parse_tokens(tokens: &[Token]) -> Result<WorkflowDefinition, String> {
    let mut tasks: Vec<WorkflowTask> = Vec::new();
    let mut synthesis = SynthesisConfig::default();
    let mut i = 0;
    while i < tokens.len() {
        i = match &tokens[i] {
            Token::LBracket => parse_list(tokens, i, &mut tasks)?,
            Token::String(_) => parse_single(tokens, i, &mut tasks)?,
            Token::Synthesize => parse_option(tokens, i, &mut synthesis, SynthesisConfig::Prompt)?,
            Token::Template => parse_option(tokens, i, &mut synthesis, SynthesisConfig::Template)?,
            other => return Err(format!("unexpected token: {}", token_name(other))),
        };
    }
    if tasks.is_empty() {
        return Err("workflow must define at least one task".into());
    }
    Ok(WorkflowDefinition { tasks, synthesis })
}

fn parse_list(
    tokens: &[Token],
    start: usize,
    tasks: &mut Vec<WorkflowTask>,
) -> Result<usize, String> {
    let end = find_matching_bracket(tokens, start)?;
    let mut i = start + 1;
    while i < end {
        match &tokens[i] {
            Token::String(_) => i = parse_single(tokens, i, tasks)?,
            Token::Comma => i += 1,
            other => {
                return Err(format!(
                    "expected task in list, found {}",
                    token_name(other)
                ))
            }
        }
    }
    Ok(end + 1)
}

fn find_matching_bracket(tokens: &[Token], start: usize) -> Result<usize, String> {
    let mut depth = 1;
    for (i, token) in tokens.iter().enumerate().skip(start + 1) {
        match token {
            Token::LBracket => depth += 1,
            Token::RBracket => {
                depth -= 1;
                if depth == 0 {
                    return Ok(i);
                }
            }
            _ => {}
        }
    }
    Err("unclosed '[' in workflow list".into())
}

fn parse_single(
    tokens: &[Token],
    start: usize,
    tasks: &mut Vec<WorkflowTask>,
) -> Result<usize, String> {
    let description = expect_string(&tokens[start])?;
    let alias = parse_alias(tokens, start + 1)?;
    tasks.push(WorkflowTask::new(description, alias));
    Ok(next_index(tokens, start + 1))
}

fn parse_alias(tokens: &[Token], idx: usize) -> Result<String, String> {
    if idx >= tokens.len() {
        return Ok("agent".into());
    }
    if tokens[idx] != Token::As {
        return Ok("agent".into());
    }
    let alias_idx = idx + 1;
    if alias_idx >= tokens.len() {
        return Err("expected alias after 'as'".into());
    }
    match &tokens[alias_idx] {
        Token::Word(a) | Token::String(a) => Ok(a.clone()),
        other => Err(format!("expected alias, found {}", token_name(other))),
    }
}

fn next_index(tokens: &[Token], after_description: usize) -> usize {
    if after_description < tokens.len() && tokens[after_description] == Token::As {
        return after_description + 2;
    }
    after_description
}

fn parse_option<F>(
    tokens: &[Token],
    start: usize,
    synthesis: &mut SynthesisConfig,
    ctor: F,
) -> Result<usize, String>
where
    F: FnOnce(String) -> SynthesisConfig,
{
    let value_idx = start + 1;
    if value_idx >= tokens.len() {
        return Err("expected value after flag".into());
    }
    let value = expect_string(&tokens[value_idx])?;
    *synthesis = ctor(value);
    Ok(value_idx + 1)
}

fn expect_string(token: &Token) -> Result<String, String> {
    match token {
        Token::String(s) => Ok(s.clone()),
        other => Err(format!(
            "expected quoted string, found {}",
            token_name(other)
        )),
    }
}

fn token_name(token: &Token) -> String {
    match token {
        Token::String(_) => "string".into(),
        Token::Word(w) => format!("'{}'", w),
        Token::As => "'as'".into(),
        Token::Comma => "','".into(),
        Token::LBracket => "'['".into(),
        Token::RBracket => "']'".into(),
        Token::Synthesize => "'--synthesize'".into(),
        Token::Template => "'--template'".into(),
    }
}

impl fmt::Display for Token {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", token_name(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_task_with_alias() {
        let def = parse_workflow_args("\"Research Rust\" as researcher").unwrap();
        assert_eq!(def.tasks.len(), 1);
        assert_eq!(def.tasks[0].description, "Research Rust");
        assert_eq!(def.tasks[0].alias, "researcher");
        assert!(matches!(def.synthesis, SynthesisConfig::Llm));
    }

    #[test]
    fn parse_parallel_list() {
        let def = parse_workflow_args("[\"Task A\" as a, \"Task B\" as b]").unwrap();
        assert_eq!(def.tasks.len(), 2);
        assert_eq!(def.tasks[0].description, "Task A");
        assert_eq!(def.tasks[0].alias, "a");
        assert_eq!(def.tasks[1].description, "Task B");
        assert_eq!(def.tasks[1].alias, "b");
    }

    #[test]
    fn parse_synthesize_option() {
        let def =
            parse_workflow_args("\"Research\" as r --synthesize \"Combine findings\"").unwrap();
        assert!(matches!(def.synthesis, SynthesisConfig::Prompt(ref s) if s == "Combine findings"));
    }

    #[test]
    fn parse_template_option() {
        let def = parse_workflow_args("[\"A\" as a] --template \"Results:\\n{tasks}\"").unwrap();
        assert!(
            matches!(def.synthesis, SynthesisConfig::Template(ref s) if s == "Results:\\n{tasks}")
        );
    }

    #[test]
    fn parse_rejects_empty_input() {
        assert!(parse_workflow_args("").is_err());
    }
}
