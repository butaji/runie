//! Intent parser - converts natural language to typed Intent
//! Uses pattern matching and heuristics for v1

use crate::router::ModelDatabase;

/// Task types that the router uses for model selection
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TaskType {
    /// Simple refactor, free tier eligible
    Refactor,
    /// Architecture or design decisions
    Architecture,
    /// Test generation
    TestGeneration,
    /// Context-heavy analysis
    Analysis,
    /// Emergency fix
    EmergencyFix,
    /// General coding task
    General,
    /// Unknown/ambiguous
    Unknown,
}

/// Parsed intent from natural language input
#[derive(Debug, Clone)]
pub struct Intent {
    /// The original text
    pub text: String,
    /// Classified task type
    pub task_type: TaskType,
    /// Estimated context size in tokens
    pub estimated_tokens: usize,
    /// Severity/criticality
    pub severity: Severity,
    /// Target files (if detected)
    pub target_files: Vec<String>,
    /// Detected language/framework
    pub language: Option<String>,
    /// Whether this is a multi-file task
    pub multi_file: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Severity {
    Low,
    Medium,
    High,
    Critical,
}

impl Intent {
    /// Create a new intent from text
    pub fn from_text(text: &str) -> Self {
        let text = text.trim();
        let lower = text.to_lowercase();

        // Detect task type
        let task_type = Self::detect_task_type(&lower);

        // Estimate context size
        let estimated_tokens = Self::estimate_tokens(text);

        // Detect severity
        let severity = Self::detect_severity(&lower);

        // Extract target files
        let target_files = Self::extract_files(text);

        // Detect language
        let language = Self::detect_language(&lower, &target_files);

        // Multi-file detection
        let multi_file = target_files.len() > 1 || lower.contains("across");

        Self {
            text: text.to_string(),
            task_type,
            estimated_tokens,
            severity,
            target_files,
            language,
            multi_file,
        }
    }

    fn detect_task_type(lower: &str) -> TaskType {
        if lower.contains("architecture") || lower.contains("design") || lower.contains("pattern") {
            TaskType::Architecture
        } else if lower.contains("refactor") || lower.contains("rename") || lower.contains("extract") {
            TaskType::Refactor
        } else if lower.contains("test") || lower.contains("spec") || lower.contains("coverage") {
            TaskType::TestGeneration
        } else if lower.contains("analyze") || lower.contains("audit") || lower.contains("review") {
            TaskType::Analysis
        } else if lower.contains("fix") || lower.contains("bug") || lower.contains("hotfix") || lower.contains("urgent") {
            TaskType::EmergencyFix
        } else {
            TaskType::General
        }
    }

    fn estimate_tokens(text: &str) -> usize {
        // Rough estimate: ~4 chars per token
        (text.len() / 4).max(100)
    }

    fn detect_severity(lower: &str) -> Severity {
        if lower.contains("critical") || lower.contains("breaking") || lower.contains("security") {
            Severity::Critical
        } else if lower.contains("important") || lower.contains("priority") || lower.contains("urgent") {
            Severity::High
        } else if lower.contains("nice") || lower.contains("should") || lower.contains("optional") {
            Severity::Low
        } else {
            Severity::Medium
        }
    }

    fn extract_files(text: &str) -> Vec<String> {
        let mut files = Vec::new();

        // Common patterns: src/foo.rs, lib/bar.ts, components/Baz.tsx
        let patterns = [
            r"[\w\-\./]+\.(rs|ts|tsx|js|jsx|py|go|java|cpp|c|h)",
            r"src/[\w\-\./]+",
            r"lib/[\w\-\./]+",
            r"components/[\w\-\./]+",
        ];

        for pattern in patterns {
            if let Ok(re) = regex::Regex::new(pattern) {
                for cap in re.find_iter(text) {
                    let file = cap.as_str().to_string();
                    if !files.contains(&file) {
                        files.push(file);
                    }
                }
            }
        }

        files
    }

    fn detect_language(lower: &str, files: &[String]) -> Option<String> {
        // From file extensions
        for file in files {
            if file.ends_with(".rs") {
                return Some("rust".to_string());
            }
            if file.ends_with(".ts") || file.ends_with(".tsx") {
                return Some("typescript".to_string());
            }
            if file.ends_with(".js") || file.ends_with(".jsx") {
                return Some("javascript".to_string());
            }
            if file.ends_with(".py") {
                return Some("python".to_string());
            }
            if file.ends_with(".go") {
                return Some("go".to_string());
            }
        }

        // From keywords
        if lower.contains("fn ") || lower.contains("impl ") || lower.contains("struct ") {
            Some("rust".to_string())
        } else if lower.contains("function ") || lower.contains("const ") || lower.contains("=>") {
            Some("javascript".to_string())
        } else {
            None
        }
    }

    /// Route to the best model for this intent
    pub fn route(&self, models: &ModelDatabase) -> Option<(String, crate::router::Model)> {
        use crate::router::HealthLevel;

        let candidates: Vec<_> = models.models.iter()
            .filter(|(_id, model)| {
                // Check context fits
                self.estimated_tokens <= model.context_length
            })
            .filter(|(id, _)| {
                // Check health
                models.statuses.get(*id)
                    .map(|s| s.health != HealthLevel::Critical)
                    .unwrap_or(false)
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Score based on task type
        let mut scored: Vec<_> = candidates.iter()
            .map(|(id, model)| {
                let mut score = 100.0;

                match self.task_type {
                    TaskType::Refactor => {
                        // Prefer free models
                        if model.input_cost == 0.0 { score += 50.0; }
                        if id.contains("llama") { score += 30.0; }
                    }
                    TaskType::Architecture => {
                        // Prefer best models
                        if id.contains("claude") { score += 50.0; }
                        if model.context_length >= 100_000 { score += 30.0; }
                    }
                    TaskType::TestGeneration => {
                        // GPT-4o-mini for pattern matching
                        if id.contains("gpt-4o-mini") { score += 40.0; }
                        if model.input_cost < 1.0 { score += 20.0; }
                    }
                    TaskType::Analysis => {
                        // Gemini for context
                        if id.contains("gemini") { score += 50.0; }
                        if model.context_length >= 500_000 { score += 40.0; }
                    }
                    TaskType::EmergencyFix => {
                        // DeepSeek for cost
                        if id.contains("deepseek") { score += 40.0; }
                        if model.input_cost < 0.5 { score += 30.0; }
                    }
                    TaskType::General => {
                        // Default priority
                        if id.contains("claude") || id.contains("gpt") { score += 20.0; }
                    }
                    TaskType::Unknown => {}
                }

                // Penalize by cost
                score -= model.input_cost * 5.0;

                // Penalize by latency
                if let Some(status) = models.statuses.get(*id) {
                    if status.latency_ms > 500 { score -= 30.0; }
                    else if status.latency_ms > 200 { score -= 10.0; }
                }

                ((*id).clone(), *model, score)
            })
            .collect();

        // Sort by score descending
        scored.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

        scored.first().map(|(id, model, _)| (id.clone(), (*model).clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_refactor_detection() {
        let intent = Intent::from_text("refactor the auth module to use OAuth2");
        assert_eq!(intent.task_type, TaskType::Refactor);
    }

    #[test]
    fn test_architecture_detection() {
        let intent = Intent::from_text("design a new architecture for the service layer");
        assert_eq!(intent.task_type, TaskType::Architecture);
    }

    #[test]
    fn test_file_extraction() {
        let intent = Intent::from_text("fix the bug in src/auth.rs and lib/token.rs");
        assert_eq!(intent.target_files.len(), 2);
        assert!(intent.target_files.contains(&"src/auth.rs".to_string()));
    }

    #[test]
    fn test_language_detection() {
        let intent = Intent::from_text("add a function in main.rs");
        assert_eq!(intent.language, Some("rust".to_string()));
    }

    #[test]
    fn test_severity_critical() {
        let intent = Intent::from_text("critical security fix in login");
        assert_eq!(intent.severity, Severity::Critical);
    }

    #[test]
    fn test_multi_file_detection() {
        let intent = Intent::from_text("refactor across all components");
        assert!(intent.multi_file);
    }
}
