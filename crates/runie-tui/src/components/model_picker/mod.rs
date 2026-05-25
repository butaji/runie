mod render;

use ratatui::{
    buffer::Buffer,
    layout::Rect,
};
use crate::theme::ThemeWrapper;

/// Model information for display
#[derive(Debug, Clone)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub is_recommended: bool,
}

/// Provider group containing models
#[derive(Debug, Clone)]
pub struct ProviderGroup {
    pub provider_id: String,
    pub provider_name: String,
    pub models: Vec<ModelInfo>,
    pub is_configured: bool,
}

/// Provider-grouped model picker
#[derive(Clone)]
pub struct ModelPicker {
    pub providers: Vec<ProviderGroup>,
    pub selected: (usize, usize), // (provider_idx, model_idx)
    pub filter: String,
    pub show_details: bool,
    pub current_model_id: Option<String>,
}

impl Default for ModelPicker {
    fn default() -> Self {
        Self::new()
    }
}

impl ModelPicker {
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
            selected: (0, 0),
            filter: String::new(),
            show_details: false,
            current_model_id: None,
        }
    }

    pub fn with_default_models() -> Self {
        Self {
            providers: default_providers(),
            selected: (0, 0),
            filter: String::new(),
            show_details: false,
            current_model_id: None,
        }
    }

    fn total_items(&self) -> usize {
        self.visible_providers().iter().map(|p| p.models.len()).sum()
    }

    fn visible_providers(&self) -> Vec<&ProviderGroup> {
        if self.filter.is_empty() {
            return self.providers.iter().collect();
        }
        self.providers.iter().filter(|p| {
            p.provider_name.to_lowercase().contains(&self.filter.to_lowercase())
                || p.models.iter().any(|m| {
                    m.name.to_lowercase().contains(&self.filter.to_lowercase())
                        || m.id.to_lowercase().contains(&self.filter.to_lowercase())
                })
        }).collect()
    }

    pub fn prev(&mut self) {
        let (p_idx, m_idx) = self.selected;
        let visible = self.visible_providers();
        if visible.is_empty() { return; }
        let mut flat_pos = 0;
        for (i, p) in visible.iter().enumerate() {
            for (j, _) in p.models.iter().enumerate() {
                if i == p_idx && j == m_idx {
                    if flat_pos > 0 {
                        flat_pos -= 1;
                        self.selected = Self::pos_to_indices(flat_pos, &visible);
                        return;
                    }
                    self.selected = Self::pos_to_indices(self.total_items().saturating_sub(1), &visible);
                    return;
                }
                flat_pos += 1;
            }
        }
        self.selected = Self::pos_to_indices(self.total_items().saturating_sub(1), &visible);
    }

    pub fn next(&mut self) {
        let visible = self.visible_providers();
        if visible.is_empty() { return; }
        let total = self.total_items();
        let (p_idx, m_idx) = self.selected;
        let mut flat_pos = 0;
        for (i, p) in visible.iter().enumerate() {
            for (j, _) in p.models.iter().enumerate() {
                if i == p_idx && j == m_idx {
                    if flat_pos + 1 < total {
                        flat_pos += 1;
                        self.selected = Self::pos_to_indices(flat_pos, &visible);
                        return;
                    }
                    self.selected = Self::pos_to_indices(0, &visible);
                    return;
                }
                flat_pos += 1;
            }
        }
        self.selected = Self::pos_to_indices(0, &visible);
    }

    fn pos_to_indices(pos: usize, visible: &[&ProviderGroup]) -> (usize, usize) {
        let mut flat_pos = 0;
        for (p_idx, p) in visible.iter().enumerate() {
            for (m_idx, _) in p.models.iter().enumerate() {
                if flat_pos == pos { return (p_idx, m_idx); }
                flat_pos += 1;
            }
        }
        (0, 0)
    }

    pub fn next_provider(&mut self) {
        let visible = self.visible_providers();
        if visible.is_empty() { return; }
        let (p_idx, _) = self.selected;
        let new_p_idx = (p_idx + 1) % visible.len();
        self.selected = (new_p_idx, 0);
    }

    pub fn prev_provider(&mut self) {
        let visible = self.visible_providers();
        if visible.is_empty() { return; }
        let (p_idx, _) = self.selected;
        let new_p_idx = p_idx.saturating_sub(1);
        self.selected = (new_p_idx, 0);
    }

    pub fn toggle_details(&mut self) {
        self.show_details = !self.show_details;
    }

    pub fn selected_model(&self) -> Option<(&str, &str)> {
        let visible = self.visible_providers();
        let (p_idx, m_idx) = self.selected;
        visible.get(p_idx).and_then(|p| {
            p.models.get(m_idx).map(|m| (p.provider_id.as_str(), m.id.as_str()))
        })
    }

    pub fn set_filter(&mut self, filter: &str) {
        self.filter = filter.to_string();
        self.selected = (0, 0);
    }

    pub fn is_current(&self, provider_id: &str, model_id: &str) -> bool {
        self.current_model_id.as_ref().map(|id| id.as_str()) == Some(model_id)
    }

    pub fn render_ref(&self, area: Rect, buf: &mut Buffer, theme: &ThemeWrapper) {
        render::render(self, area, buf, theme)
    }
}

fn default_providers() -> Vec<ProviderGroup> {
    vec![
        anthropic_provider(),
        openai_provider(),
        google_provider(),
    ]
}

fn anthropic_provider() -> ProviderGroup {
    ProviderGroup {
        provider_id: "anthropic".to_string(),
        provider_name: "Anthropic".to_string(),
        is_configured: true,
        models: vec![
            ModelInfo { id: "claude-3-5-sonnet-20241022".to_string(), name: "Claude 3.5 Sonnet".to_string(), description: "Most capable model for complex tasks".to_string(), is_recommended: true },
            ModelInfo { id: "claude-3-opus-20240229".to_string(), name: "Claude 3 Opus".to_string(), description: "Highest intelligence for nuanced tasks".to_string(), is_recommended: false },
            ModelInfo { id: "claude-3-haiku-20240307".to_string(), name: "Claude 3 Haiku".to_string(), description: "Fast, lightweight for simple tasks".to_string(), is_recommended: false },
        ],
    }
}

fn openai_provider() -> ProviderGroup {
    ProviderGroup {
        provider_id: "openai".to_string(),
        provider_name: "OpenAI".to_string(),
        is_configured: true,
        models: vec![
            ModelInfo { id: "gpt-4o".to_string(), name: "GPT-4o".to_string(), description: "Most capable multimodal model".to_string(), is_recommended: false },
            ModelInfo { id: "gpt-4o-mini".to_string(), name: "GPT-4o Mini".to_string(), description: "Fast, affordable small model".to_string(), is_recommended: false },
            ModelInfo { id: "o1-mini".to_string(), name: "O1 Mini".to_string(), description: "Reasoning model optimized for code".to_string(), is_recommended: false },
        ],
    }
}

fn google_provider() -> ProviderGroup {
    ProviderGroup {
        provider_id: "google".to_string(),
        provider_name: "Google".to_string(),
        is_configured: true,
        models: vec![
            ModelInfo { id: "gemini-1.5-pro".to_string(), name: "Gemini 1.5 Pro".to_string(), description: "Balanced multimodal model".to_string(), is_recommended: false },
            ModelInfo { id: "gemini-1.5-flash".to_string(), name: "Gemini 1.5 Flash".to_string(), description: "Fast, efficient for high-volume tasks".to_string(), is_recommended: false },
        ],
    }
}
