//! Cached view data and animation state.
//!
//! View feed cache (elements, posts, line_counts, total_lines) is built
//! on-demand when building Snapshot. The projection is owned by UiActor;
//! AppState holds raw domain state and computes derived values on-demand.

use std::sync::Arc;

use crate::commands::DialogKind;
use crate::model::state::AppState;
use crate::model::view_cache::ViewCache;
use crate::model::{build_model_selector_items, ModelSelectorItem};
use crate::model_catalog::configured_models_catalog;
use crate::snapshot::{
    compute_current_top_element, compute_hovered_element, compute_mouse_target, Snapshot,
};

mod animation;
mod snapshot_fill;

/// Extracted view values to avoid borrow conflicts.
struct ViewValues {
    mouse_position: Option<(u16, u16)>,
    last_content_width: u16,
    last_visible_height: u16,
    scroll: usize,
    selected_post: Option<usize>,
}

/// Pure computation of mouse targeting without needing AppState borrow.
fn snapshot_mouse_impl(
    elements: &[crate::view::Element],
    line_counts: &[usize],
    total_lines: usize,
    view: &ViewValues,
    input: &str,
    has_models: bool,
) -> (crate::snapshot::MouseTarget, Option<usize>) {
    let mouse_pos = view.mouse_position;
    let target = compute_mouse_target(
        mouse_pos,
        view.last_content_width,
        view.last_visible_height,
        input,
        has_models,
    );
    let hovered = compute_hovered_element(
        mouse_pos,
        view.last_content_width,
        view.last_visible_height,
        input,
        elements,
        line_counts,
        total_lines,
        has_models,
    );
    (target, hovered)
}

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
            .map(|cmd| (cmd.name.clone(), cmd.desc.clone(), cmd.category.to_string()))
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
                    "Skill".to_owned(),
                ));
            }
        }
    }

    fn session_tree_items(&self) -> Arc<[(usize, String)]> {
        Arc::clone(&self.view().cached_session_tree_items)
    }

    fn refresh_session_tree_items(&mut self) {
        let filter = match self.open_dialog() {
            Some(crate::commands::DialogState::Active {
                kind: DialogKind::SessionTree,
                panels: _,
            }) => crate::session::tree::SessionTreeFilter::All,
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
                        .map(|(depth, node_id)| {
                            let node = tree.arena()[node_id].get();
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

    /// Build a view cache on-demand from the current state.
    /// This is called by ensure_fresh and snapshot methods.
    fn build_view_cache(&mut self) -> ViewCache {
        let feed = crate::view::LazyCache::feed(self);
        let width = self.view().last_content_width.max(1);
        let line_counts: Vec<usize> = feed
            .elements
            .iter()
            .map(|e| crate::layout::element_line_count(e, width))
            .collect();
        let total_lines: usize = line_counts.iter().sum();
        ViewCache {
            elements: feed.elements.into(),
            posts: feed.posts.into(),
            line_counts: line_counts.into(),
            total_lines,
            cached_gen: self.view().message_gen,
        }
    }

    /// Rebuild caches when inputs changed — O(n) but gated.
    pub fn ensure_fresh(&mut self) {
        // Extract view state values first to avoid borrow conflicts.
        let prev_total_lines = self.view().total_lines;
        let was_streaming = self.agent_state().streaming;
        let prev_scroll = self.view().scroll;
        let prev_selected_post = self.view().selected_post;

        // Build the view cache on-demand and store for reuse by snapshot_feed().
        let cache = self.build_view_cache();
        self.view_mut().cached_feed = Some(cache.clone());

        // Extract needed values before any other mutable operations.
        let posts_len = cache.posts.len();
        let total_lines = cache.total_lines;
        let line_counts = Arc::clone(&cache.line_counts);

        self.refresh_palette_items();
        self.refresh_model_selector_items();
        self.refresh_session_tree_items();
        self.refresh_settings_items();

        // Keep the nav-mode selection valid after the feed changes.
        if let Some(sel) = prev_selected_post {
            let max = posts_len.saturating_sub(1);
            self.view_mut().selected_post = Some(sel.min(max));
        }

        // Update scroll-computed values in ViewState for event handlers.
        self.view_mut().total_lines = total_lines;
        self.view_mut().line_counts = line_counts;

        // While streaming, preserve the user's scroll position so new content
        // does not auto-scroll the viewport when the user has scrolled up.
        if was_streaming && prev_scroll > 0 {
            let delta = total_lines.saturating_sub(prev_total_lines);
            self.view_mut().scroll = prev_scroll.saturating_add(delta);
        }

        self.view_mut().dirty = false;
    }

    /// Build an immutable Snapshot for the render actor.
    pub fn snapshot(&mut self) -> Snapshot {
        self.ensure_fresh();
        let mut s = self.snapshot_base();
        snapshot_fill::fill_snapshot_config(&mut s, self);
        snapshot_fill::fill_snapshot_dialog(&mut s, self);
        snapshot_fill::fill_snapshot_meta(&mut s, self);
        s
    }

    fn snapshot_base(&mut self) -> Snapshot {
        let mut s = self.snapshot_feed();
        snapshot_fill::fill_snapshot_input(&mut s, self);
        snapshot_fill::fill_snapshot_agent(&mut s, self);
        s
    }

    fn snapshot_feed(&mut self) -> Snapshot {
        let view_values = self.extract_view_values();
        // Reuse the cache built in ensure_fresh() if the generation matches.
        let cache = match &self.view().cached_feed {
            Some(c) if c.cached_gen == self.view().message_gen => c.clone(),
            _ => {
                // Cache stale or absent — rebuild.
                self.ensure_fresh();
                self.view()
                    .cached_feed
                    .clone()
                    .expect("cached_feed must be set after ensure_fresh")
            }
        };
        let (mouse_target, hovered_element) = snapshot_mouse_impl(
            &cache.elements,
            &cache.line_counts,
            cache.total_lines,
            &view_values,
            &self.input().input,
            self.has_models(),
        );
        let current_top = compute_current_top_element(
            &cache.elements,
            &cache.line_counts,
            cache.total_lines,
            view_values.scroll,
            view_values.last_visible_height,
        );

        Snapshot {
            elements: cache.elements,
            line_counts: cache.line_counts,
            total_lines: cache.total_lines,
            scroll: view_values.scroll,
            content_width: view_values.last_content_width,
            current_top_element: current_top,
            posts: cache.posts,
            selected_post: view_values.selected_post,
            last_visible_height: view_values.last_visible_height,
            mouse_target,
            hovered_element,
            mouse_position: view_values.mouse_position,
            ..Default::default()
        }
    }

    fn extract_view_values(&self) -> ViewValues {
        let view = self.view();
        ViewValues {
            mouse_position: view.mouse_position,
            last_content_width: view.last_content_width,
            last_visible_height: view.last_visible_height,
            scroll: view.scroll,
            selected_post: view.selected_post,
        }
    }
}
