//! Unified Component trait for all UI components.
//!
//! This trait provides a common interface for rendering components,
//! with each component having an associated ViewModel type.

use ratatui::{buffer::Buffer, layout::Rect};
use crossterm::event::KeyEvent;
use crate::theme::ThemeWrapper;
use crate::tui::state::Msg;

use super::{
    MessageList, CommandPalette, PermissionModal, DiffViewer, ModelPicker,
    SessionTreeNavigator, onboarding,
};
use crate::components::onboarding::Onboarding;
use crate::tui::view_models::{
    MessageListViewModel,
    StatusBarViewModel, InputBarViewModel, AgentListViewModel as AgentListVm,
};
use crate::components::top_bar::TopBarViewModel;

/// Unified trait for all renderable UI components.
///
/// Each component has an associated ViewModel type that holds
/// the data needed for rendering. This trait provides a common
/// interface to render any component.
pub trait Component {
    /// The ViewModel type this component uses for rendering
    type ViewModel;

    /// Render the component using the provided ViewModel
    fn render(
        &self,
        vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    );

    /// Handle keyboard input. Returns Some(Msg) if consumed, None to bubble up.
    fn handle_key(&mut self, _key: KeyEvent) -> Option<Msg> {
        None // default: don't consume
    }

    /// Whether this component wants keyboard focus
    fn wants_focus(&self) -> bool {
        false
    }
}

// ─── InputBar ─────────────────────────────────────────────────────────────────

/// InputBar wraps the TextArea for keyboard handling in Chat/Select modes
pub struct InputBar;

impl InputBar {
    pub fn new() -> Self {
        InputBar
    }
}

impl Default for InputBar {
    fn default() -> Self {
        Self::new()
    }
}

impl Component for InputBar {
    type ViewModel = InputBarViewModel;

    fn render(
        &self,
        vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        super::input_bar::render_input_bar(
            &vm.textarea,
            &vm.prompt,
            &vm.right_info,
            area,
            buf,
            theme,
        );
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── MessageList ─────────────────────────────────────────────────────────────

impl Component for MessageListViewModel {
    type ViewModel = MessageListViewModel;

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        MessageList::render_ref(self, area, buf, theme);
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── CommandPalette ───────────────────────────────────────────────────────────

/// CommandPalette - uses () as ViewModel since render ignores the vm param
impl Component for CommandPalette {
    type ViewModel = ();

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        self.render_ref(area, buf, theme);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Msg> {
        use crossterm::event::KeyCode;

        // Esc cancels argument mode if active, otherwise bubbles up (palette closes via mode change)
        if matches!(key.code, KeyCode::Esc) {
            return Some(Msg::CommandPaletteCancelArgument);
        }
        match key.code {
            KeyCode::Enter => Some(Msg::CommandPaletteConfirm),
            KeyCode::Up => Some(Msg::CommandPaletteUp),
            KeyCode::Down => Some(Msg::CommandPaletteDown),
            KeyCode::Backspace => Some(Msg::CommandPaletteBackspace),
            KeyCode::Char(c) => Some(Msg::CommandPaletteFilter(c)),
            _ => None,
        }
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── PermissionModal ──────────────────────────────────────────────────────────

impl Component for PermissionModal {
    type ViewModel = ();

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        self.render_ref(area, buf, theme);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Msg> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Ctrl+C and Ctrl+Q in Permission mode both cancel permission
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Msg::PermissionCancel),
                _ => {}
            }
        }

        match key.code {
            KeyCode::Enter | KeyCode::Char('y') => Some(Msg::PermissionConfirm),
            KeyCode::Esc | KeyCode::Char('n') => Some(Msg::PermissionCancel),
            KeyCode::Char('a') => Some(Msg::PermissionAlways),
            KeyCode::Char('s') => Some(Msg::PermissionSkip),
            _ => None,
        }
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── DiffViewer ───────────────────────────────────────────────────────────────

/// DiffViewer renders via its own render_ref method
impl Component for DiffViewer {
    type ViewModel = ();

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        self.render_ref(area, buf, theme);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Msg> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Ctrl+C and Ctrl+Q close the modal
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('c') | KeyCode::Char('q') => return Some(Msg::CloseModal),
                _ => {}
            }
        }
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('x') => Some(Msg::CloseModal),
            KeyCode::Down | KeyCode::Char('j') => Some(Msg::ScrollDown),
            KeyCode::Up | KeyCode::Char('k') => Some(Msg::ScrollUp),
            KeyCode::PageDown => Some(Msg::ScrollDown),
            KeyCode::PageUp => Some(Msg::ScrollUp),
            _ => None,
        }
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── ModelPicker ──────────────────────────────────────────────────────────────

impl Component for ModelPicker {
    type ViewModel = ();

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        self.render_ref(area, buf, theme);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Msg> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => Some(Msg::CloseModal),
            KeyCode::Up | KeyCode::Char('k') => Some(Msg::SelectUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Msg::SelectDown),
            KeyCode::Enter => Some(Msg::SelectConfirm),
            KeyCode::Char('d') => Some(Msg::SelectToggleDetails),
            _ => None,
        }
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── SessionTreeNavigator ─────────────────────────────────────────────────────

impl Component for SessionTreeNavigator {
    type ViewModel = ();

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        self.render_ref(area, buf, theme);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Msg> {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Esc => Some(Msg::CloseModal),
            KeyCode::Up | KeyCode::Char('k') => Some(Msg::SessionTreeUp),
            KeyCode::Down | KeyCode::Char('j') => Some(Msg::SessionTreeDown),
            KeyCode::Enter => Some(Msg::SessionTreeConfirm),
            _ => None,
        }
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── StatusBarViewModel ───────────────────────────────────────────────────────

impl Component for StatusBarViewModel {
    type ViewModel = StatusBarViewModel;

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        let colors = crate::theme::ThemeColors::from(theme);
        super::status_bar::render_ref(self, area, buf, &colors);
    }
}

// ─── TopBarViewModel ─────────────────────────────────────────────────────────

/// TopBar is rendered via TopBarViewModel using the render_top_bar function
impl Component for TopBarViewModel {
    type ViewModel = TopBarViewModel;

    fn render(
        &self,
        vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        // Use the free function which takes TopBarViewModel
        let colors = crate::theme::ThemeColors::from(theme);
        super::top_bar::render_top_bar(vm, area, buf, &colors);
    }
}

// ─── Onboarding ────────────────────────────────────────────────────────────────

/// Onboarding uses () as ViewModel since render ignores the vm param
impl Component for Onboarding {
    type ViewModel = ();

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        super::onboarding::render::render_onboarding(self, area, buf, theme);
    }

    fn handle_key(&mut self, key: KeyEvent) -> Option<Msg> {
        use crossterm::event::KeyCode;

        let is_picker_step = matches!(
            self.step,
            onboarding::OnboardingStep::ProviderSelect
                | onboarding::OnboardingStep::ModelSelect
        );

        match key.code {
            KeyCode::Enter => Some(Msg::OnboardingNext),
            KeyCode::Esc => Some(Msg::OnboardingBack),
            KeyCode::Up => Some(Msg::OnboardingNavigateUp),
            KeyCode::Down => Some(Msg::OnboardingNavigateDown),
            KeyCode::Char(c) => {
                if is_picker_step {
                    Some(Msg::OnboardingSearchInput(c))
                } else {
                    Some(Msg::OnboardingKeyInput(c))
                }
            }
            KeyCode::Backspace | KeyCode::Delete => {
                if is_picker_step {
                    Some(Msg::OnboardingSearchBackspace)
                } else {
                    Some(Msg::OnboardingKeyBackspace)
                }
            }
            _ => None,
        }
    }

    fn wants_focus(&self) -> bool {
        true
    }
}

// ─── AgentListViewModel ───────────────────────────────────────────────────────

/// AgentListViewModel renders via the render_agent_list free function
impl Component for AgentListVm {
    type ViewModel = AgentListVm;

    fn render(
        &self,
        _vm: &Self::ViewModel,
        area: Rect,
        buf: &mut Buffer,
        theme: &ThemeWrapper,
    ) {
        let colors = crate::theme::ThemeColors::from(theme);
        crate::tui::render::render_agent_list(self, area, buf, &colors);
    }
}

