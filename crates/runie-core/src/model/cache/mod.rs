//! Cached view data and animation state.

use std::sync::Arc;

use crate::model::state::AppState;
use crate::model::{build_model_selector_items, ModelSelectorItem};
use crate::model_catalog::configured_models_catalog;
use crate::snapshot::{
    compute_current_top_element, compute_hovered_element, compute_mouse_target, Snapshot,
};

impl AppState {
    fn palette_filter(&self) -> Option<String> {
        match self.open_dialog() {
            Some(d) => d
                .panel_stack()
                .and_then(|s| s.current())
                .map(|p| p.filter.clone()),
            _ => None,
        }
    }

    fn palette_items(&self) -> Arc<[(String, String, String)]> {
        Arc::clone(&self.view().cached_palette_items)
    }

    fn refresh_palette_items(&mut self) {
        let filter = match self.palette_filter() {
            Some(f) => f,
            _ => {
                self.view_mut().cached_palette_filter = None;
                if self.view_mut().cached_palette_items.is_empty() {
                    return;
                }
                self.view_mut().cached_palette_items = Arc::new([]);
                return;
            }
        };

        if Some(&filter) != self.view_mut().cached_palette_filter.as_ref() {
            self.view_mut().cached_palette_filter = Some(filter.clone());
            let mut items = self.command_palette_items(&filter);
            self.add_skill_palette_items(&filter, &mut items);
            self.view_mut().cached_palette_items = items.into();
        }
    }

    fn command_palette_items(&self, filter: &str) -> Vec<(String, String, String)> {
        crate::commands::filter_commands(&self.registry, filter)
            .into_iter()
            .map(|cmd| {
                (
                    cmd.name.clone(),
                    cmd.desc.clone(),
                    cmd.category.as_str().to_string(),
                )
            })
            .collect()
    }

    fn add_skill_palette_items(&self, filter: &str, items: &mut Vec<(String, String, String)>) {
        let f = filter.to_lowercase();
        for skill in self.skills().iter() {
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

    fn session_tree_items(&self) -> Arc<[(usize, String)]> {
        Arc::clone(&self.view().cached_session_tree_items)
    }

    fn refresh_session_tree_items(&mut self) {
        let filter = match self.open_dialog() {
            Some(crate::commands::DialogState::SessionTree(_)) => {
                crate::session::tree::SessionTreeFilter::All
            }
            _ => {
                self.view_mut().cached_session_tree_valid = false;
                if self.view_mut().cached_session_tree_items.is_empty() {
                    return;
                }
                self.view_mut().cached_session_tree_items = Arc::new([]);
                return;
            }
        };
        if !self.view_mut().cached_session_tree_valid {
            self.view_mut().cached_session_tree_items =
                match self.session_mut().session_tree.as_ref() {
                    Some(tree) => tree
                        .filtered_walk(filter)
                        .into_iter()
                        .map(|(depth, node)| {
                            let preview = format!(
                                "[{}] {}",
                                node.message.role.as_str(),
                                node.message.content().chars().take(60).collect::<String>()
                            );
                            (depth, preview)
                        })
                        .collect::<Vec<_>>()
                        .into(),
                    None => Arc::new([]),
                };
            self.view_mut().cached_session_tree_valid = true;
        }
    }

    fn model_selector_items(&self) -> Arc<[ModelSelectorItem]> {
        Arc::clone(&self.view().cached_model_items)
    }

    fn refresh_model_selector_items(&mut self) {
        let filter = match self.open_dialog() {
            Some(d) => d
                .panel_stack()
                .and_then(|s| s.current())
                .map(|p| p.filter.clone())
                .unwrap_or_default(),
            _ => {
                self.view_mut().cached_model_filter = None;
                if self.view_mut().cached_model_items.is_empty() {
                    return;
                }
                self.view_mut().cached_model_items = Arc::new([]);
                return;
            }
        };
        if Some(&filter) != self.view_mut().cached_model_filter.as_ref() {
            self.view_mut().cached_model_filter = Some(filter.clone());
            let configured = self.configured_providers();
            let models = configured_models_catalog(&configured);
            let config = self.config();
            self.view_mut().cached_model_items = build_model_selector_items(
                &models,
                &config.recent_models,
                &filter,
                &config.current_provider,
                &config.current_model,
            )
            .into();
        }
    }

    fn settings_items(&self) -> Arc<[crate::settings::SettingItem]> {
        Arc::clone(&self.view().cached_settings_items)
    }

    fn refresh_settings_items(&mut self) {
        if !self.view_mut().cached_settings_valid {
            self.view_mut().cached_settings_items =
                crate::update::settings_dialog::build_setting_items(self).into();
            self.view_mut().cached_settings_valid = true;
        }
    }

    fn auth_providers(&self) -> Arc<[String]> {
        Arc::clone(&self.view().cached_auth_providers)
    }

    pub fn set_auth_providers(&mut self, providers: Vec<String>) {
        self.view_mut().cached_auth_providers = providers.into();
    }

    /// Rebuild caches when inputs changed — O(n) but gated.
    pub fn ensure_fresh(&mut self) {
        let prev_total_lines = self.view_mut().total_lines;
        let was_streaming = self.agent_state_mut().streaming;
        let prev_scroll = self.view_mut().scroll;

        self.refresh_palette_items();
        self.refresh_model_selector_items();
        self.refresh_session_tree_items();
        self.refresh_settings_items();

        if self.view_mut().dirty && self.view_mut().message_gen != self.view_mut().cached_gen {
            let feed = crate::view::LazyCache::feed(self);
            self.view_mut().element_count = feed.elements.len();
            let width = self.view_mut().last_content_width.max(1);
            let line_counts: Vec<usize> = feed
                .elements
                .iter()
                .map(|e| crate::layout::element_line_count(e, width))
                .collect();
            self.view_mut().total_lines = line_counts.iter().sum();
            self.view_mut().line_counts = line_counts.into();
            self.view_mut().elements_cache = feed.elements.into();
            self.view_mut().posts = feed.posts.into();
            self.view_mut().cached_gen = self.view_mut().message_gen;
        }
        // Keep the nav-mode selection valid after the feed changes.
        if let Some(sel) = self.view_mut().selected_post {
            let max = self.view_mut().posts.len().saturating_sub(1);
            self.view_mut().selected_post = Some(sel.min(max));
        }
        // While streaming, preserve the user's scroll position so new content
        // does not auto-scroll the viewport when the user has scrolled up.
        if was_streaming && prev_scroll > 0 {
            let delta = self.view_mut().total_lines.saturating_sub(prev_total_lines);
            self.view_mut().scroll = self.view_mut().scroll.saturating_add(delta);
        }
        self.view_mut().dirty = false;
    }

    pub fn tick_animation(&mut self) {
        let mut changed = false;
        if self.agent_state_mut().turn_active {
            self.view_mut().animation_frame = self.view_mut().animation_frame.wrapping_add(1);
            self.update_speed();
            changed = true;
        }
        if self.input_mut().input_flash > 0 {
            self.input_mut().input_flash -= 1;
            changed = true;
        }
        if self.clear_expired_transient() {
            changed = true;
        }
        if self.animate_tokens() {
            changed = true;
        }
        if changed {
            self.view_mut().dirty = true;
        }
    }

    /// Animate token display values toward their actual values.
    /// Returns true if the display values changed.
    fn animate_tokens(&mut self) -> bool {
        if self.agent_state_mut().tokens_in != self.agent_state_mut().tokens_in_prev {
            self.agent_state_mut().tokens_in_prev = self.agent_state_mut().tokens_in;
        }
        if self.agent_state_mut().tokens_out != self.agent_state_mut().tokens_out_prev {
            self.agent_state_mut().tokens_out_prev = self.agent_state_mut().tokens_out;
        }

        let c1 = Self::animate_token_value(
            self.agent_state_mut().tokens_in,
            &mut self.agent_state_mut().tokens_in_display,
        );
        let c2 = Self::animate_token_value(
            self.agent_state_mut().tokens_out,
            &mut self.agent_state_mut().tokens_out_display,
        );
        c1 || c2
    }

    fn animate_token_value(target: usize, display: &mut f64) -> bool {
        let t = target as f64;
        let d = t - *display;
        if d.abs() < 0.5 {
            let changed = display.round() as usize != target;
            if changed {
                *display = t;
            }
            changed
        } else {
            *display += d * 0.15;
            true
        }
    }

    /// Update streaming speed using rolling window of last 1000 tokens.
    /// Called every animation tick (~200ms).
    pub fn update_speed(&mut self) {
        let now = std::time::Instant::now();

        let (elapsed, tokens_out, tokens_at_last_speed) = {
            let agent = self.agent_state_mut();
            let last = agent.last_speed_update.get_or_insert(now);
            (
                now.duration_since(*last).as_secs_f64(),
                agent.tokens_out,
                agent.tokens_at_last_speed,
            )
        };

        if elapsed < 0.05 {
            return; // Too soon, wait for next tick
        }

        let delta_tokens = tokens_out.saturating_sub(tokens_at_last_speed);

        if delta_tokens > 0 {
            self.agent_state_mut().speed_window.record(tokens_out);
            self.agent_state_mut().tokens_at_last_speed = tokens_out;
            self.agent_state_mut().speed_tps = self.agent_state_mut().speed_window.speed();
            if let Some(last) = self.agent_state_mut().last_speed_update.as_mut() {
                *last = now;
            }
        } else if elapsed > 1.0 {
            self.agent_state_mut().speed_tps *= 0.5;
            if self.agent_state_mut().speed_tps < 0.1 {
                self.agent_state_mut().speed_tps = 0.0;
            }
        }
    }

    fn clear_expired_transient(&mut self) -> bool {
        if let Some(until) = *self.transient_until_mut() {
            if std::time::Instant::now() > until {
                *self.transient_message_mut() = None;
                *self.transient_until_mut() = None;
                *self.transient_level_mut() = None;
                return true;
            }
        }
        false
    }

    fn snapshot_mouse(&self) -> (crate::snapshot::MouseTarget, Option<usize>) {
        let has_models = self.has_models();
        let view = self.view();
        let input = self.input();
        let target = compute_mouse_target(
            view.mouse_position,
            view.last_content_width,
            view.last_visible_height,
            &input.input,
            has_models,
        );
        let hovered = compute_hovered_element(
            view.mouse_position,
            view.last_content_width,
            view.last_visible_height,
            &input.input,
            &view.elements_cache,
            &view.line_counts,
            view.total_lines,
            has_models,
        );
        (target, hovered)
    }

    /// Build an immutable Snapshot for the render actor.
    pub fn snapshot(&mut self) -> Snapshot {
        self.ensure_fresh();
        let mut s = self.snapshot_base();
        self.fill_snapshot_config(&mut s);
        self.fill_snapshot_dialog(&mut s);
        self.fill_snapshot_meta(&mut s);
        s
    }

    fn snapshot_base(&self) -> Snapshot {
        let mut s = self.snapshot_feed();
        self.fill_snapshot_input(&mut s);
        self.fill_snapshot_agent(&mut s);
        s
    }

    fn snapshot_feed(&self) -> Snapshot {
        let (mouse_target, hovered_element) = self.snapshot_mouse();
        let view = self.view();
        Snapshot {
            elements: Arc::clone(&view.elements_cache),
            line_counts: Arc::clone(&view.line_counts),
            total_lines: view.total_lines,
            scroll: view.scroll,
            content_width: view.last_content_width,
            current_top_element: compute_current_top_element(
                &view.elements_cache,
                &view.line_counts,
                view.total_lines,
                view.scroll,
                view.last_visible_height,
            ),
            posts: Arc::clone(&view.posts),
            selected_post: view.selected_post,
            last_visible_height: view.last_visible_height,
            mouse_target,
            hovered_element,
            mouse_position: view.mouse_position,
            ..Default::default()
        }
    }

    fn fill_snapshot_input(&self, s: &mut Snapshot) {
        let input = self.input();
        let completion = self.completion();
        s.input = input.input.clone();
        s.cursor_pos = input.cursor_pos;
        s.hint_text = self.hint_text();
        s.placeholder = input.placeholder.clone();
        s.ghost_completion = input.ghost_completion.clone();
        s.input_scroll = input.input_scroll;
        s.path_suggestions = completion.path_suggestions.clone();
        s.path_selected = completion.path_selected;
    }

    fn fill_snapshot_agent(&self, s: &mut Snapshot) {
        let agent = self.agent_state();
        let input = self.input();
        let view = self.view();
        s.turn_active = agent.turn_active;
        s.input_flash = input.input_flash;
        s.vim_nav_mode = view.vim_nav_mode;
        s.spinner_frame = self.spinner_frame();
        s.turn_elapsed_secs = self.turn_elapsed_secs();
        s.queue_count = agent.message_queue.len() + agent.request_queue.len();
        s.tokens_in = agent.tokens_in;
        s.tokens_out = agent.tokens_out;
        s.speed_tps = agent.speed_tps;
        s.tokens_in_display = agent.tokens_in_display;
        s.tokens_out_display = agent.tokens_out_display;
        s.streaming_tail = agent.streaming_buffer.tail().to_string();
    }

    fn fill_snapshot_config(&self, s: &mut Snapshot) {
        let config = self.config();
        s.provider = config.current_provider.clone();
        s.model = config.current_model.clone();
        s.has_models = self.has_models();
        s.theme_name = config.theme_name.clone();
        s.thinking_level = config.thinking_level;
        s.read_only = config.read_only;
        // Build input title: "provider/model · read-only"
        s.input_title = build_input_title(
            &config.current_provider,
            &config.current_model,
            config.read_only,
        );
    }

    fn fill_snapshot_dialog(&self, s: &mut Snapshot) {
        s.dialog = self.open_dialog().cloned();
        s.palette_items = self.palette_items();
        s.model_selector_items = self.model_selector_items();
        s.settings_items = self.settings_items();
        s.session_tree_items = self.session_tree_items();
        s.auth_providers = self.auth_providers();
    }

    fn fill_snapshot_meta(&self, s: &mut Snapshot) {
        s.transient_message = self.transient_message().cloned();
        s.transient_level = self.transient_level;
        s.git_info = self.git_info().cloned();
        s.cwd_name = self.cwd_name().to_string();
        s.pending_edits = self.session().pending_edits.clone();
        s.scoped_models = self.config().scoped_models.clone();
        s.image_attachments = self.session().image_attachments.clone();
        s.permission_request = self.permission_request().cloned();
        s.last_visible_height = self.view().last_visible_height;
    }
}
/// Build the input box title string.
/// Format: `provider/model · read-only` when read-only, otherwise `provider/model`.
fn build_input_title(provider: &str, model: &str, read_only: bool) -> String {
    let base = format!("{}/{}", provider, model);
    if read_only {
        format!("{} · read-only", base)
    } else {
        base
    }
}
#[cfg(test)]
mod tests;
