use super::*;
use std::io::Write;
use std::path::Path;
use tempfile::tempdir;

#[test]
fn load_skills_from_dir_parses_markdown() {
    let dir = tempdir().unwrap();
    let mut file = std::fs::File::create(dir.path().join("rust.md")).unwrap();
    file.write_all(
        b"# Rust Skill\n\n## Description\n\nBest practices for Rust.\n\n## Context\n\nAlways use clippy.\n\n## Invocation\n\nUser can invoke with /skill rust\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "rust");
    assert_eq!(skills[0].description, "Best practices for Rust.");
    assert_eq!(skills[0].context, "Always use clippy.");
    assert!(skills[0].user_invocable);
}

#[test]
fn skill_not_user_invocable_without_invocation_section() {
    let dir = tempdir().unwrap();
    let mut file = std::fs::File::create(dir.path().join("quiet.md")).unwrap();
    file.write_all(
        b"# Quiet\n\n## Description\n\nBe concise.\n\n## Context\n\nKeep answers short.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert!(!skills[0].user_invocable);
}

#[test]
fn empty_dir_returns_no_skills() {
    let dir = tempdir().unwrap();
    let skills = load_from_dir(dir.path());
    assert!(skills.is_empty());
}

#[test]
fn nonexistent_dir_returns_no_skills() {
    let skills = load_from_dir(Path::new("/does/not/exist"));
    assert!(skills.is_empty());
}

#[test]
fn skill_injects_context() {
    let skills = vec![Skill {
        name: "rust".into(),
        description: "Rust best practices".into(),
        context: "Use clippy.".into(),
        user_invocable: false,
        file_path: PathBuf::from("rust.md"),
    }];
    let ctx = build_skills_context(&skills);
    assert!(ctx.contains("Use clippy."));
    assert!(ctx.contains("Additional context:"));
}

#[test]
fn empty_context_returns_empty_string() {
    let skills = vec![Skill {
        name: "empty".into(),
        description: "Nothing".into(),
        context: "".into(),
        user_invocable: false,
        file_path: PathBuf::from("empty.md"),
    }];
    let ctx = build_skills_context(&skills);
    assert!(ctx.is_empty());
}

#[test]
fn user_invocable_shown_in_summary() {
    let skill = Skill {
        name: "test".into(),
        description: "A test skill".into(),
        context: "".into(),
        user_invocable: true,
        file_path: PathBuf::from("test.md"),
    };
    assert!(skill.summary().contains("(invocable)"));
}

#[test]
fn load_all_merges_user_and_project() {
    let user_dir = tempdir().unwrap();
    let project_dir = tempdir().unwrap();

    let mut file = std::fs::File::create(user_dir.path().join("user_skill.md")).unwrap();
    file.write_all(b"# User\n\n## Description\n\nUser skill.\n")
        .unwrap();

    let mut file = std::fs::File::create(project_dir.path().join("project_skill.md")).unwrap();
    file.write_all(b"# Project\n\n## Description\n\nProject skill.\n")
        .unwrap();

    // load_all uses hardcoded paths, so test merge manually
    let mut skills = load_from_dir(user_dir.path());
    skills.extend(load_from_dir(project_dir.path()));
    assert_eq!(skills.len(), 2);
}

#[test]
fn non_md_files_are_ignored() {
    let dir = tempdir().unwrap();
    let mut file = std::fs::File::create(dir.path().join("readme.txt")).unwrap();
    file.write_all(b"## Description\n\nNot a skill.\n").unwrap();

    let skills = load_from_dir(dir.path());
    assert!(skills.is_empty());
}

#[test]
fn subdirectory_skill_loads() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("rust");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    file.write_all(
        b"# Rust Skill\n\n## Description\n\nBest practices for Rust.\n\n## Context\n\nUse clippy.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "rust");
    assert_eq!(skills[0].description, "Best practices for Rust.");
}

#[test]
fn subdirectory_prefers_over_flat_file() {
    let dir = tempdir().unwrap();

    // Flat file
    let mut flat = std::fs::File::create(dir.path().join("rust.md")).unwrap();
    flat.write_all(b"# Flat Rust\n\n## Description\n\nFlat description.\n")
        .unwrap();

    // Subdirectory version (should win)
    let skill_dir = dir.path().join("rust");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut subdir = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    subdir
        .write_all(b"# Subdir Rust\n\n## Description\n\nSubdir description.\n")
        .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "rust");
    assert_eq!(skills[0].description, "Subdir description.");
}

#[test]
fn yaml_frontmatter_overrides_name_and_description() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("my-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    file.write_all(
        b"---\nname: custom-name\ndescription: From frontmatter\ncontext: Some context.\n---\n\n## Description\n\nFrom section.\n\n## Context\n\nSome section context.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "custom-name");
    assert_eq!(skills[0].description, "From frontmatter");
    assert_eq!(skills[0].context, "Some context.");
}

#[test]
fn yaml_frontmatter_falls_back_to_sections() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("my-skill");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    file.write_all(
        b"---\ndescription: Frontmatter desc\n---\n\n## Description\n\nSection desc.\n\n## Context\n\nSome context.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    // Name from dir, description from frontmatter
    assert_eq!(skills[0].name, "my-skill");
    assert_eq!(skills[0].description, "Frontmatter desc");
    assert_eq!(skills[0].context, "Some context.");
}

#[test]
fn flat_md_file_still_works() {
    let dir = tempdir().unwrap();
    let mut file = std::fs::File::create(dir.path().join("flat.md")).unwrap();
    file.write_all(
        b"# Flat Skill\n\n## Description\n\nA flat skill.\n\n## Context\n\nFlat context.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "flat");
    assert_eq!(skills[0].description, "A flat skill.");
}

#[test]
fn build_skills_context_includes_subdir_skill() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("code-review");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    file.write_all(
        b"# Code Review\n\n## Description\n\nReview code.\n\n## Context\n\nRun clippy before review.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    let ctx = build_skills_context(&skills);
    assert!(ctx.contains("Run clippy before review."));
}

// ── serde_yaml-specific tests ────────────────────────────────────────────────

#[test]
fn serde_yaml_frontmatter_parses_quoted_strings() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("quoted");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    // Quoted strings: double-quoted handles colons, single-quoted preserves literal
    file.write_all(
        b"---\nname: \"quoted-name\"\ndescription: \"Desc with colon: inside\"\ncontext: 'Context with single quotes'\n---\n\n## Description\n\nNot used.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "quoted-name");
    assert_eq!(skills[0].description, "Desc with colon: inside");
    assert_eq!(skills[0].context, "Context with single quotes");
}

#[test]
fn serde_yaml_frontmatter_parses_multiline_context() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("multiline");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    // Multiline using | (literal block scalar)
    file.write_all(
        b"---\nname: multiline-skill\ndescription: A skill\ncontext: |\n  Line one\n  Line two\n  Line three\n---\n\n## Description\n\nNot used.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].context, "Line one\nLine two\nLine three");
}

#[test]
fn serde_yaml_frontmatter_parses_multiline_with_indentation() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("folded");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    // Multiline using > (folded block scalar)
    file.write_all(
        b"---\nname: folded-skill\ndescription: A skill\ncontext: >\n  This is\n  folded into\n  a single line\n---\n\n## Description\n\nNot used.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    // Folded scalars replace newlines with spaces
    assert!(skills[0].context.contains("folded into"));
}

#[test]
fn serde_yaml_frontmatter_ignores_non_string_values() {
    let dir = tempdir().unwrap();
    let skill_dir = dir.path().join("mixed");
    std::fs::create_dir(&skill_dir).unwrap();
    let mut file = std::fs::File::create(skill_dir.join("SKILL.md")).unwrap();
    // A list value (not a string) should be ignored, plain string should be kept
    file.write_all(
        b"---\nname: mixed-skill\ntags:\n  - rust\n  - tool\ndescription: Plain string description\n---\n\n## Description\n\nNot used.\n",
    )
    .unwrap();

    let skills = load_from_dir(dir.path());
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "mixed-skill");
    assert_eq!(skills[0].description, "Plain string description");
}

#[test]
fn serde_yaml_frontmatter_no_frontmatter_returns_empty() {
    let fm = extract_frontmatter("# No frontmatter\n\n## Description\n\nJust text.\n");
    assert!(fm.is_empty());
}

#[test]
fn serde_yaml_frontmatter_empty_frontmatter_returns_empty() {
    let fm = extract_frontmatter("---\n---\n\n## Description\n\nNo keys.\n");
    assert!(fm.is_empty());
}

#[test]
fn serde_yaml_frontmatter_single_quoted_values() {
    let fm = extract_frontmatter("---\nname: 'single quoted'\ndescription: 'also single'\n---\n");
    assert_eq!(fm.get("name"), Some(&"single quoted".to_string()));
    assert_eq!(fm.get("description"), Some(&"also single".to_string()));
}
