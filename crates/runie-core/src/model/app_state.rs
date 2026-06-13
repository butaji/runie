//! AppState implementation methods.

use crate::model::AppState;
use crate::snapshot::Snapshot;
use crate::ui::elements::Element;
use std::sync::Arc;

impl AppState {
    /// Seconds elapsed since thinking started.
    pub fn thinking_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .thinking_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    /// Seconds elapsed since turn started.
    pub fn turn_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .turn_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    /// Seconds elapsed since current tool started.
    pub fn tool_elapsed_secs(&self) -> Option<f64> {
        self.agent
            .tool_started_at
            .map(|t| t.elapsed().as_secs_f64())
    }

    /// Braille spinner frame (12-frame cycle).
    pub fn spinner_frame(&self) -> char {
        use crate::model::{SPINNER_CHARS, SPINNER_FRAMES};
        SPINNER_CHARS[(self.view.animation_frame % SPINNER_FRAMES) as usize]
    }

    /// Generate and increment next request ID.
    pub fn next_id(&mut self) -> String {
        let id = format!("req.{}", self.agent.next_id);
        self.agent.next_id += 1;
        id
    }

    /// Mark the view as needing a redraw.
    pub(crate) fn mark_dirty(&mut self) {
        self.view.dirty = true;
    }

    /// Signal that messages have changed (increments generations).
    pub fn messages_changed(&mut self) {
        self.view.message_gen = self.view.message_gen.wrapping_add(1);
        self.session.session_updated_at = crate::message::now();
        self.view.dirty = true;
    }

    /// Get filtered command palette items for the current dialog.
    fn palette_items(&mut self) -> Vec<(String, String, String)> {
        let filter = match &self.open_dialog {
            Some(d) => d
                .panel_stack()
                .current()
                .map(|p| p.filter.clone())
                .unwrap_or_default(),
            _ => {
                self.view.cached_palette_filter = None;
                self.view.cached_palette_items.clear();
                return Vec::new();
            }
        };
        if Some(&filter) != self.view.cached_palette_filter.as_ref() {
            self.view.cached_palette_filter = Some(filter.clone());
            let mut items: Vec<_> = crate::commands::filter_commands(&self.registry, &filter)
                .into_iter()
                .map(|cmd| {
                    (
                        cmd.name.clone(),
                        cmd.desc.clone(),
                        cmd.category.as_str().to_string(),
                    )
                })
                .collect();
            self.add_matching_skills(&mut items, &filter);
            self.view.cached_palette_items = items;
        }
        self.view.cached_palette_items.clone()
    }

    /// Add skills matching the filter to the palette items list.
    fn add_matching_skills(&self, items: &mut Vec<(String, String, String)>, filter: &str) {
        let f = filter.to_lowercase();
        for skill in &self.skills {
            if skill.user_invocable
                && (f.is_empty()
                    || skill.name.to_lowercase().contains(&f)
                    || skill.description.to_lowercase().contains(&f))
            {
                items.push((
                    skill.name.clone(),
                    skill.description.clone(),
                    "Skill".to_string(),
                ));
            }
        }
    }

    /// Get session tree items for the dialog.
    fn session_tree_items(&self) -> Vec<(usize, String)> {
        let filter = match &self.open_dialog {
            Some(crate::commands::DialogState::SessionTree(_)) => {
                crate::session_tree::SessionTreeFilter::All
            }
            _ => return Vec::new(),
        };
        match self.session.session_tree.as_ref() {
            Some(tree) => tree
                .filtered_walk(filter)
                .into_iter()
                .map(|(depth, node)| {
                    let preview = format!(
                        "[{}] {}",
                        node.message.role.as_str(),
                        node.message.content.chars().take(60).collect::<String>()
                    );
                    (depth, preview)
                })
                .collect(),
            None => Vec::new(),
        }
    }

    /// Get filtered model selector items.
    fn model_selector_items(&mut self) -> Vec<(String, String, String, bool, bool)> {
        let filter = match &self.open_dialog {
            Some(d) => d
                .panel_stack()
                .current()
                .map(|p| p.filter.clone())
                .unwrap_or_default(),
            _ => {
                self.view.cached_model_filter = None;
                self.view.cached_model_items.clear();
                return Vec::new();
            }
        };
        if Some(&filter) != self.view.cached_model_filter.as_ref() {
            self.view.cached_model_filter = Some(filter.clone());
            self.view.cached_model_items = crate::model_catalog::build_model_selector_items(
                &crate::model_catalog::model_catalog(),
                &self.config.recent_models,
                &filter,
                &self.config.current_provider,
                &self.config.current_model,
            );
        }
        self.view.cached_model_items.clone()
    }

    /// Record a model selection in recent history (max 5, no duplicates).
    pub fn record_model_usage(&mut self, provider: &str, model: &str) {
        let full = format!("{}/{}", provider, model);
        self.config.recent_models.retain(|m| m != &full);
        self.config.recent_models.push(full);
        if self.config.recent_models.len() > 5 {
            self.config.recent_models.remove(0);
        }
    }

    /// Current cache generation number.
    pub fn cache_generation(&self) -> u64 {
        self.view.message_gen
    }

    /// Rebuild element cache if dirty.
    pub fn ensure_fresh(&mut self) {
        if self.view.dirty && self.view.message_gen != self.view.cached_gen {
            let elements = crate::ui::LazyCache::rebuild(self);
            self.view.element_count = elements.len();
            let line_counts: Vec<usize> = elements.iter().map(|e| e.line_count()).collect();
            self.view.total_lines = line_counts.iter().sum();
            self.view.line_counts = line_counts.into();
            self.view.elements_cache = elements.into();
            self.view.cached_gen = self.view.message_gen;
        }
        self.view.dirty = false;
    }

    /// Visible elements slice — O(1), zero allocation.
    pub fn visible(&self, skip: usize, take: usize) -> &[Element] {
        if self.view.elements_cache.is_empty() {
            return &[];
        }
        let start = skip
            .min(self.view.element_count)
            .min(self.view.elements_cache.len());
        let end = (start + take)
            .min(self.view.element_count)
            .min(self.view.elements_cache.len());
        &self.view.elements_cache[start..end]
    }

    /// Total visible element count.
    pub fn count(&self) -> usize {
        self.view.element_count.max(self.view.elements_cache.len())
    }

    /// Visible element count.
    pub fn element_count(&self) -> usize {
        self.view.element_count
    }

    /// Total rendered line count.
    pub fn total_lines(&self) -> usize {
        self.view.total_lines
    }

    /// Element cache reference.
    pub fn elements_cache(&self) -> &[Element] {
        self.view.elements_cache.as_ref()
    }

    /// Advance animation state. Called every tick.
    pub fn tick_animation(&mut self) {
        let mut changed = false;
        if self.agent.turn_active {
            self.view.animation_frame = self.view.animation_frame.wrapping_add(1);
            self.update_speed();
            changed = true;
        }
        if self.input.input_flash > 0 {
            self.input.input_flash -= 1;
            changed = true;
        }
        if self.clear_expired_transient() {
            changed = true;
        }
        if self.animate_tokens() {
            changed = true;
        }
        if changed {
            self.view.dirty = true;
        }
    }

    /// Animate token display values toward actual values.
    fn animate_tokens(&mut self) -> bool {
        if self.agent.tokens_in != self.agent.tokens_in_prev {
            self.agent.tokens_in_prev = self.agent.tokens_in;
        }
        if self.agent.tokens_out != self.agent.tokens_out_prev {
            self.agent.tokens_out_prev = self.agent.tokens_out;
        }
        let t_in = self.agent.tokens_in as f64;
        let t_out = self.agent.tokens_out as f64;
        let d_in = t_in - self.agent.tokens_in_display;
        let d_out = t_out - self.agent.tokens_out_display;
        let c1 = if d_in.abs() < 0.5 {
            let n = self.agent.tokens_in_display.round() as usize != t_in as usize;
            if n {
                self.agent.tokens_in_display = t_in;
            }
            n
        } else {
            self.agent.tokens_in_display += d_in * 0.15;
            true
        };
        let c2 = if d_out.abs() < 0.5 {
            let n = self.agent.tokens_out_display.round() as usize != t_out as usize;
            if n {
                self.agent.tokens_out_display = t_out;
            }
            n
        } else {
            self.agent.tokens_out_display += d_out * 0.15;
            true
        };
        c1 || c2
    }

    /// Update streaming speed from rolling token window.
    pub fn update_speed(&mut self) {
        use std::time::Instant;
        let now = Instant::now();
        let last = self.agent.last_speed_update.get_or_insert(now);
        let elapsed = now.duration_since(*last).as_secs_f64();
        if elapsed < 0.05 {
            return;
        }
        let prev_tokens = self.agent.tokens_at_last_speed;
        let delta_tokens = self.agent.tokens_out.saturating_sub(prev_tokens);
        if delta_tokens > 0 {
            self.agent.speed_window.record(self.agent.tokens_out);
            self.agent.tokens_at_last_speed = self.agent.tokens_out;
            self.agent.speed_tps = self.agent.speed_window.speed();
            *last = now;
        } else if elapsed > 1.0 {
            self.agent.speed_tps *= 0.5;
            if self.agent.speed_tps < 0.1 {
                self.agent.speed_tps = 0.0;
            }
        }
    }

    /// Clear expired transient notification.
    fn clear_expired_transient(&mut self) -> bool {
        if let Some(until) = self.transient_until {
            if std::time::Instant::now() > until {
                self.transient_message = None;
                self.transient_until = None;
                self.transient_level = None;
                return true;
            }
        }
        false
    }

    /// Build an immutable Snapshot for the render actor.
    pub fn snapshot(&mut self) -> Snapshot {
        Snapshot {
            elements: Arc::clone(&self.view.elements_cache),
            line_counts: Arc::clone(&self.view.line_counts),
            total_lines: self.view.total_lines,
            input: self.input.input.clone(),
            cursor_pos: self.input.cursor_pos,
            hint_text: self.hint_text(),
            path_suggestions: self.completion.path_suggestions.clone(),
            path_selected: self.completion.path_selected,
            turn_active: self.agent.turn_active,
            input_flash: self.input.input_flash,
            placeholder: self.input.placeholder.clone(),
            ghost_completion: self.input.ghost_completion.clone(),
            spinner_frame: self.spinner_frame(),
            scroll: self.view.scroll,
            turn_elapsed_secs: self.turn_elapsed_secs(),
            provider: self.config.current_provider.clone(),
            model: self.config.current_model.clone(),
            theme_name: self.config.theme_name.clone(),
            thinking_level: self.config.thinking_level,
            read_only: self.config.read_only,
            queue_count: self.agent.message_queue.len() + self.agent.request_queue.len(),
            dialog: self.open_dialog.clone(),
            palette_items: self.palette_items(),
            model_selector_items: self.model_selector_items(),
            pending_edits: self.session.pending_edits.clone(),
            scoped_models: self.config.scoped_models.clone(),
            settings_items: crate::update::settings_dialog::build_setting_items(self),
            session_tree_items: self.session_tree_items(),
            image_attachments: self.session.image_attachments.clone(),
            auth_providers: crate::auth::AuthStorage::load()
                .tokens
                .keys()
                .cloned()
                .collect(),
            transient_message: self.transient_message.clone(),
            transient_level: self.transient_level,
            tokens_in: self.agent.tokens_in,
            tokens_out: self.agent.tokens_out,
            speed_tps: self.agent.speed_tps,
            tokens_in_display: self.agent.tokens_in_display,
            tokens_out_display: self.agent.tokens_out_display,
            git_info: self.git_info.clone(),
            cwd_name: self.cwd_name.clone(),
            input_scroll: self.input.input_scroll,
        }
    }

    /// Check if view needs redraw.
    pub fn is_dirty(&self) -> bool {
        self.view.dirty
    }

    /// Total tokens across all messages.
    pub fn total_tokens(&self) -> usize {
        self.session
            .messages
            .iter()
            .map(|m| crate::tokens::estimate_tokens(&m.content))
            .sum()
    }

    /// Compact session to keep roughly `keep_recent_tokens`.
    pub fn compact(&mut self, keep_recent_tokens: usize) -> String {
        use crate::message::{now, ChatMessage, Role};
        let total = self.total_tokens();
        if total <= keep_recent_tokens {
            return format!("Session has {} tokens, no compaction needed", total);
        }
        let mut accumulated = 0usize;
        let mut cut_idx = 0usize;
        for (i, msg) in self.session.messages.iter().enumerate().rev() {
            accumulated += crate::tokens::estimate_tokens(&msg.content);
            if accumulated >= keep_recent_tokens {
                cut_idx = i;
                break;
            }
        }
        while cut_idx < self.session.messages.len() {
            match self.session.messages[cut_idx].role {
                Role::User | Role::Assistant => break,
                _ => cut_idx += 1,
            }
        }
        if cut_idx == 0 {
            return "Cannot compact: all messages are recent".to_string();
        }
        let removed_count = cut_idx;
        self.session.messages.drain(..cut_idx);
        let summary = format!(
            "[Compacted: {} earlier messages removed, keeping ~{} tokens]",
            removed_count, keep_recent_tokens
        );
        self.session.messages.insert(
            0,
            ChatMessage {
                role: Role::System,
                content: summary.clone(),
                timestamp: now(),
                id: "compaction".to_string(),
                ..Default::default()
            },
        );
        self.messages_changed();
        summary
    }
}
