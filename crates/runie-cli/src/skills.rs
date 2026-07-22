//! `runie skill` — Manage skills (list, create, delete, show, install).
//!
//! Skills are stored in `~/.runie/skills/` directory.

use anyhow::Result;

/// Get the skills directory path.
fn skills_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".runie")
        .join("skills")
}

/// Ensure the skills directory exists.
fn ensure_skills_dir() -> Result<std::path::PathBuf> {
    let dir = skills_dir();
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

/// Run the `skill list` subcommand.
pub async fn list() -> Result<()> {
    let skills = runie_core::skills::load_all();

    if skills.is_empty() {
        println!("No skills found.");
        println!("\nSkills are loaded from:");
        println!("  - ~/.agents/skills/");
        println!("  - ~/.runie/skills/");
        println!("  - ./.runie/skills/");
        println!("\nUse `runie skill create <name>` to create a new skill.");
        return Ok(());
    }

    println!("Installed Skills ({}):\n", skills.len());
    for skill in &skills {
        println!("  {}", skill.summary());
    }

    Ok(())
}

/// Run the `skill show` subcommand.
pub async fn show(name: &str) -> Result<()> {
    let skills = runie_core::skills::load_all();

    let skill = skills.iter().find(|s| s.name == name);
    match skill {
        Some(s) => {
            println!("# Skill: {}\n", s.name);
            println!("{}", s.description);
            if !s.context.is_empty() {
                println!("\n## Context\n\n{}", s.context);
            }
            if s.user_invocable {
                println!("\n(User invocable)");
            }
            println!("\nFile: {}", s.file_path);
        }
        None => {
            anyhow::bail!(
                "Skill '{}' not found. Use `runie skill list` to see available skills.",
                name
            );
        }
    }
    Ok(())
}

/// Run the `skill create` subcommand.
pub async fn create(name: &str) -> Result<()> {
    let dir = ensure_skills_dir()?;

    let skill_path = dir.join(name).with_extension("md");
    let skill_dir = dir.join(name);

    if skill_path.exists() || skill_dir.join("SKILL.md").exists() {
        anyhow::bail!("Skill '{}' already exists.", name);
    }

    let template = format!(
        r#"---
name: {name}
description: Description of the {name} skill
context: |
  Describe how this skill should be used and what context it provides.
---

# {name}

Describe what this skill does and how it helps.

## Usage

Describe how to use this skill.

## Examples

Provide examples of how this skill is used.
"#,
        name = name
    );

    std::fs::write(&skill_path, &template)?;
    println!("Created skill '{}' at: {}", name, skill_path.display());
    println!("\nEdit the file to customize your skill.");
    Ok(())
}

/// Run the `skill delete` subcommand.
pub async fn delete(name: &str) -> Result<()> {
    let dir = skills_dir();

    // Check both flat (.md) and nested (/SKILL.md) formats
    let skill_path = dir.join(name).with_extension("md");
    let skill_dir = dir.join(name);
    let skill_md = skill_dir.join("SKILL.md");

    let path_to_delete = if skill_path.exists() {
        skill_path
    } else if skill_md.exists() {
        skill_md
    } else {
        anyhow::bail!("Skill '{}' not found.", name);
    };

    use std::io::{self, Write};
    print!("Delete skill '{}'? (y/N): ", name);
    io::stdout().flush()?;
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;
    let confirm = confirm.trim().to_lowercase();

    if confirm != "y" && confirm != "yes" {
        println!("Aborted.");
        return Ok(());
    }

    // If it's a file, delete it; if it's a directory, delete recursively
    if path_to_delete.is_dir() {
        std::fs::remove_dir_all(&path_to_delete)?;
    } else {
        std::fs::remove_file(&path_to_delete)?;
    }
    println!("Deleted skill '{}'.", name);
    Ok(())
}

/// Run the `skill install` subcommand.
pub async fn install(url: &str, name: Option<&str>) -> Result<()> {
    let dir = ensure_skills_dir()?;

    // Fetch the skill file
    println!("Fetching skill from: {}", url);
    let response = reqwest::get(url).await?;
    if !response.status().is_success() {
        anyhow::bail!("Failed to fetch skill: HTTP {}", response.status());
    }
    let content = response.text().await?;

    // Parse frontmatter to get skill name, or use provided/derived name
    let skill_name = if let Some(n) = name {
        n.to_string()
    } else {
        // Try to extract name from frontmatter
        if let Some(start) = content.find("name:") {
            let after_name = &content[start + 5..];
            if let Some(end) = after_name.find('\n') {
                let name_line = after_name[..end].trim().to_string();
                if !name_line.is_empty() {
                    name_line
                } else {
                    // Derive from URL
                    derive_name_from_url(url)
                }
            } else {
                derive_name_from_url(url)
            }
        } else {
            derive_name_from_url(url)
        }
    };

    let skill_path = dir.join(&skill_name).with_extension("md");
    if skill_path.exists() {
        anyhow::bail!(
            "Skill '{}' already exists at {}. Use `runie skill delete {}` first.",
            skill_name,
            skill_path.display(),
            skill_name
        );
    }

    std::fs::write(&skill_path, &content)?;
    println!(
        "Installed skill '{}' to: {}",
        skill_name,
        skill_path.display()
    );
    Ok(())
}

/// Derive a skill name from a URL.
fn derive_name_from_url(url: &str) -> String {
    // Get the filename from URL
    let url_path = url.split('?').next().unwrap_or(url);
    let filename = url_path
        .split('/')
        .next_back()
        .unwrap_or("skill")
        .trim_end_matches(".md")
        .trim_end_matches("/SKILL.md")
        .to_string();

    // Clean up common patterns
    let name = filename
        .replace("-skill", "")
        .replace("_skill", "")
        .replace("-SKILL", "")
        .replace("_SKILL", "");

    if name.is_empty() || name == "SKILL.md" {
        "skill".to_string()
    } else {
        name
    }
}

/// Run the `skill disable` subcommand.
pub async fn disable(name: &str) -> Result<()> {
    let mut config = runie_core::config::Config::load(None);
    if config.skills.disabled.contains(&name.to_string()) {
        println!("Skill '{}' is already disabled.", name);
        return Ok(());
    }
    config.skills.disabled.push(name.to_string());
    config.save()?;
    println!("Disabled skill '{}'.", name);
    Ok(())
}

/// Run the `skill enable` subcommand.
pub async fn enable(name: &str) -> Result<()> {
    let mut config = runie_core::config::Config::load(None);
    let original_len = config.skills.disabled.len();
    config.skills.disabled.retain(|s| s != name);
    if config.skills.disabled.len() == original_len {
        println!("Skill '{}' is not in the disabled list.", name);
    } else {
        config.save()?;
        println!("Enabled skill '{}'.", name);
    }
    Ok(())
}

/// Run the `skill toggle` subcommand.
pub async fn toggle(name: &str) -> Result<()> {
    let mut config = runie_core::config::Config::load(None);
    if config.skills.disabled.contains(&name.to_string()) {
        config.skills.disabled.retain(|s| s != name);
        config.save()?;
        println!("Enabled skill '{}'.", name);
    } else {
        config.skills.disabled.push(name.to_string());
        config.save()?;
        println!("Disabled skill '{}'.", name);
    }
    Ok(())
}

/// Run the `skill reset` subcommand.
pub async fn reset() -> Result<()> {
    let mut config = runie_core::config::Config::load(None);
    let count = config.skills.disabled.len();
    config.skills.disabled.clear();
    config.save()?;
    if count > 0 {
        println!("Reset {} disabled skill(s).", count);
    } else {
        println!("No disabled skills to reset.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_name_from_url_github() {
        let url = "https://github.com/user/repo/raw/main/SKILL.md";
        assert_eq!(derive_name_from_url(url), "SKILL".to_string());
    }

    #[test]
    fn derive_name_from_url_with_name() {
        let url = "https://example.com/path/my-custom-skill.md";
        let name = derive_name_from_url(url);
        assert!(name.contains("my-custom") || name.contains("custom"));
    }
}
