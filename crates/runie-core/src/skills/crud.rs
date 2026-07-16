//! Skills CRUD Operations
//!
//! Provides create, update, delete operations for skills.
//! Skills are stored as SKILL.md files in standard locations.

use anyhow::{Context, Result};

use super::{load_from_dir, Skill};

/// Create a new skill with the given name and content.
/// Returns the created skill.
pub fn create_skill(name: &str, description: &str, content: &str) -> Result<Skill> {
    let skill_dir = get_user_skills_dir()?.join(name);
    
    // Create skill directory
    std::fs::create_dir_all(&skill_dir)
        .with_context(|| format!("Failed to create skill directory: {}", skill_dir.display()))?;
    
    // Generate SKILL.md content
    let skill_md = skill_dir.join("SKILL.md");
    let frontmatter = format!(
        "---\nname: {}\ndescription: {}\n---\n\n",
        name, description
    );
    let full_content = format!("{}{}", frontmatter, content);
    
    // Write the file
    std::fs::write(&skill_md, &full_content)
        .with_context(|| format!("Failed to write skill file: {}", skill_md.display()))?;
    
    // Return the created skill
    Ok(Skill {
        name: name.to_string(),
        description: description.to_string(),
        context: String::new(),
        user_invocable: true,
        file_path: skill_md.into(),
    })
}

/// Update an existing skill's content.
/// Returns the updated skill.
pub fn update_skill(name: &str, description: Option<&str>, content: Option<&str>) -> Result<Skill> {
    let skill = find_skill(name)?;
    
    // Read existing content
    let existing = std::fs::read_to_string(&skill.file_path)
        .with_context(|| format!("Failed to read skill file: {}", skill.file_path))?;
    
    // Parse and update
    let new_description = description.unwrap_or(&skill.description);
    let new_content = content.unwrap_or(&skill.context);
    
    // Generate updated content
    let frontmatter = format!(
        "---\nname: {}\ndescription: {}\n---\n\n",
        name, new_description
    );
    let full_content = format!("{}{}", frontmatter, new_content);
    
    // Write the file
    std::fs::write(&skill.file_path, &full_content)
        .with_context(|| format!("Failed to update skill file: {}", skill.file_path))?;
    
    // Return the updated skill
    Ok(Skill {
        name: name.to_string(),
        description: new_description.to_string(),
        context: new_content.to_string(),
        user_invocable: skill.user_invocable,
        file_path: skill.file_path.clone(),
    })
}

/// Delete a skill by name.
/// Returns Ok if deleted, or error if skill doesn't exist.
pub fn delete_skill(name: &str) -> Result<()> {
    let skill = find_skill(name)?;
    
    // Remove the skill directory
    if let Some(parent) = skill.file_path.parent() {
        std::fs::remove_dir_all(parent)
            .with_context(|| format!("Failed to delete skill directory: {}", parent.display()))?;
    }
    
    Ok(())
}

/// Get the directory for user-level skills (~/.runie/skills/).
fn get_user_skills_dir() -> Result<PathBuf> {
    let home = dirs::home_dir()
        .context("Could not find home directory")?;
    Ok(home.join(".runie").join("skills"))
}

/// Find a skill by name across all skill directories.
/// Returns the skill if found, or error if not found.
pub fn find_skill(name: &str) -> Result<Skill> {
    // Search in user skills
    if let Some(home) = dirs::home_dir() {
        let user_skills = home.join(".runie").join("skills");
        let skills = load_from_dir(&user_skills);
        if let Some(skill) = skills.iter().find(|s| s.name == name) {
            return Ok(skill.clone());
        }
    }
    
    // Search in project skills
    let project_skills = PathBuf::from(".runie").join("skills");
    let skills = load_from_dir(&project_skills);
    if let Some(skill) = skills.iter().find(|s| s.name == name) {
        return Ok(skill.clone());
    }
    
    anyhow::bail!("Skill '{}' not found", name)
}

/// Check if a skill exists.
pub fn skill_exists(name: &str) -> bool {
    find_skill(name).is_ok()
}

/// Get all skill directories for hot-reload watching.
pub fn get_skill_directories() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    
    // User skills directory
    if let Some(home) = dirs::home_dir() {
        let user_skills = home.join(".runie").join("skills");
        if user_skills.exists() {
            dirs.push(user_skills);
        }
    }
    
    // Project skills directory
    let project_skills = PathBuf::from(".runie").join("skills");
    if project_skills.exists() {
        dirs.push(project_skills);
    }
    
    dirs
}
