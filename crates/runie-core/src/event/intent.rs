//! Typed intent enum for the declarative DSL.
//!
//! Every intent maps to a target actor's message type. When `Event::kind()`
//! returns `EventKind::Intent`, callers should use `Event::into_intent()` to
//! get the typed intent and then route it to the appropriate actor via
//! `ActorHandles`.
//!
//! ## Naming
//!
//! Intents are imperative or noun-phrase requests: `SetTheme`, `TrustProject`,
//! `SubmitInput`. This differs from Facts which are past-tense: `ConfigLoaded`,
//! `SessionSaved`.
//!
//! ## Routing
//!
//! See [`kind`](kind) for the routing rules (facts → projection, intents → actors,
//! control → system handler).

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use strum::IntoStaticStr;

use crate::event::TransientLevel;
use crate::settings::SettingsCategory;
use crate::trust::TrustDecision;

// ─────────────────────────────────────────────────────────────────────────────
// Intent enum
// ─────────────────────────────────────────────────────────────────────────────

/// Typed intent for the declarative DSL.
///
/// Variants are organized by the actor that handles them. Handlers convert
/// `Event` to `Intent` via `Event::into_intent()` and then route to actors
/// via `ActorHandles`.
///
/// Each variant carries the minimal data needed by the target actor; the
/// authoritative source of all intent data is the original `Event` variant.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, IntoStaticStr)]
#[serde(tag = "type", content = "data")]
pub enum Intent {
    // ── ConfigActor ─────────────────────────────────────────────────────────

    /// Request ConfigActor to set the active theme name.
    SetTheme { name: String },
    /// Request ConfigActor to reload config from disk.
    ReloadConfig,

    // ── SessionActor ────────────────────────────────────────────────────────

    /// Request SessionActor to set a trust decision for a project path.
    SetTrust { path: PathBuf, decision: TrustDecision },
    /// Request SessionActor to append to the input history.
    AppendHistory { entry: String },

    // ── IoActor ────────────────────────────────────────────────────────────

    /// Request IoActor to run a bash command and emit `BashOutput`.
    RunBash { command: String },
    /// Request IoActor to write files and emit `FilesWritten`.
    WriteFiles { edits: Vec<(PathBuf, String)> },

    // ── UiControl (not yet a dedicated actor) ─────────────────────────────
    // These intents don't have a dedicated actor yet; they fall back to
    // `AppState::update()` direct mutations until the UiControlActor is built.

    /// Show a transient notification.
    Notify { content: String, level: TransientLevel },
    /// Clear the current transient notification.
    ClearTransient,
    /// Quit the application.
    Quit,
    /// Force-quit without cleanup.
    ForceQuit,
    /// Reset the current session.
    Reset,
    /// Abort the current agent turn.
    Abort,
    /// Request a follow-up turn.
    FollowUp,
    /// Toggle collapse/expand for a message.
    ToggleExpand,
    /// Dequeue the next pending message.
    Dequeue,
    /// Open the external editor.
    OpenExternalEditor,
    /// External editor finished with content.
    ExternalEditorDone { content: String },
    /// Share the current session.
    ShareSession,
    /// Suspend the application.
    Suspend,
    /// Toggle vim navigation mode.
    ToggleVimMode,
    /// Copy the last assistant response.
    CopyLastResponse,
    /// Open the session list dialog.
    OpenSessionList,
    /// Start a new session.
    NewSession,
    /// Resume a paused session.
    ResumeSession,
    /// Select a session by id.
    SelectSession { id: String },
    /// Star a session by id.
    StarSession { id: String },
    /// Rename a session.
    RenameSession { id: String, name: String },
    /// Delete a session by id.
    DeleteSession { id: String },

    // ── ModelConfig (routed to AppState direct mutation for now) ───────────

    /// Switch to a different provider/model.
    SwitchModel { provider: String, model: String, explicit: bool },
    /// Cycle to the next scoped model.
    CycleModelNext,
    /// Cycle to the previous scoped model.
    CycleModelPrev,
    /// Toggle the scoped models dialog.
    ToggleScopedModelsDialog,
    /// Toggle a scoped model enabled/disabled.
    ScopedModelToggle { provider: String, name: String },
    /// Enable all scoped models.
    ScopedModelEnableAll,
    /// Disable all scoped models.
    ScopedModelDisableAll,
    /// Toggle all models for a provider.
    ScopedModelToggleProvider { provider: String },
    /// Toggle the settings dialog.
    ToggleSettingsDialog,
    /// Settings dialog navigation.
    SettingsUp,
    SettingsDown,
    SettingsLeft,
    SettingsRight,
    SettingsSelect,
    SettingsClose,
    SettingsSwitchCategory { category: SettingsCategory },
    /// Cycle the thinking level.
    CycleThinkingLevel,
    /// Set a specific thinking level.
    SetThinkingLevel( crate::model::ThinkingLevel),
    /// Toggle read-only mode.
    ToggleReadOnly,
    /// Trust the current project.
    TrustProject,
    /// Untrust the current project.
    UntrustProject,
    /// Reload all runtime resources (keybindings, skills).
    ReloadAll,
    /// Show diagnostics.
    ShowDiagnostics,

    // ── Dialog intents (UiControl territory) ──────────────────────────────

    /// Toggle the welcome dialog.
    ToggleWelcome,
    /// Toggle the command palette.
    ToggleCommandPalette,
    /// Filter the palette by a character.
    PaletteFilter(char),
    PaletteBackspace,
    PaletteUp,
    PaletteDown,
    PaletteSelect,
    PaletteClose,
    /// Toggle the model selector dialog.
    ToggleModelSelector,
    ModelSelectorFilter(char),
    ModelSelectorBackspace,
    ModelSelectorUp,
    ModelSelectorDown,
    ModelSelectorSelect,
    ModelSelectorClose,
    /// Toggle path completion.
    TogglePathCompletion,
    PathCompletionUp,
    PathCompletionDown,
    PathCompletionSelect,
    PathCompletionClose,
    /// Command form input.
    CommandFormInput(char),
    CommandFormBackspace,
    CommandFormUp,
    CommandFormDown,
    CommandFormSubmit,
    CommandFormClose,
    /// Go back in the dialog stack.
    DialogBack,
    /// Open providers dialog.
    ProvidersDialog,
    ProvidersSelectModel { provider: String, model: String },
    ProvidersDisconnect { provider: String },
    ProvidersAdd,
    ProvidersEditModels { provider: String },
    /// Copy text to clipboard.
    CopyToClipboard(String),
    /// Copy the selected message block.
    CopySelectedBlock,
    /// Copy the selected block's metadata.
    CopyBlockMetadata,
    /// Open the `@` file picker.
    AtFilePicker,
    /// Insert a file reference at the cursor.
    InsertAtRef(String),

    // ── Edit intents ──────────────────────────────────────────────────────

    /// Pending edit awaiting approval.
    PendingEdit { path: String, original: String, proposed: String },
    /// Approve a pending edit.
    ApproveEdit,
    /// Reject a pending edit.
    RejectEdit,

    // ── Command intents ────────────────────────────────────────────────────

    /// Run `/load <name>`.
    RunLoadCommand { name: String },
    /// Run `/save <name>`.
    RunSaveCommand { name: String },
    /// Run `/delete <name>`.
    RunDeleteCommand { name: String },
    /// Run `/import <path>`.
    RunImportCommand { path: String },
    /// Run `/export <path>`.
    RunExportCommand { path: String },
    /// Run `/skill <name>`.
    RunSkillCommand { name: String },
    /// Run `/login <provider> <token>`.
    RunLoginCommand { provider: String, token: String },
    /// Run `/logout <provider>`.
    RunLogoutCommand { provider: String },
    /// Run `/name <name>`.
    RunNameCommand { name: String },
    /// Run `/fork <index>`.
    RunForkCommand { message_index: String },
    /// Run `/compact`.
    RunCompactCommand { keep: String, focus: String },
    /// Run `/prompt <name>`.
    RunPromptCommand { name: String },
    /// Run `/thinking <level>`.
    RunThinkingCommand { level: crate::model::ThinkingLevel },
    /// Run a palette command by name.
    RunPaletteCommand { name: String, args: String },

    // ── LoginFlow intents ──────────────────────────────────────────────────

    /// Start the login/auth flow.
    LoginStart,
    /// Select a provider in the login flow.
    SelectProvider { provider: String },
    /// Submit an API key for a provider.
    SubmitKey { provider: String, key: String },
    /// Toggle a model selection in the login flow.
    ToggleModel { model: String },
    /// Save the login/auth configuration.
    LoginSave,
    /// Cancel the login/auth flow.
    LoginCancel,

    // ── Session tree intents ───────────────────────────────────────────────

    /// Fork the session at a message index.
    ForkSession { message_index: usize },
    /// Clone the current session.
    CloneSession,
    /// Toggle the session tree view.
    ToggleSessionTree,
    /// Cycle the session tree filter.
    SessionTreeFilterCycle,
    /// Select a session in the tree.
    SessionTreeSelect { id: String },

    // ── Scroll intents (input territory) ──────────────────────────────────

    /// Scroll up.
    ScrollUp,
    /// Scroll down.
    ScrollDown,

    // ── Input intents ──────────────────────────────────────────────────────

    /// Raw character input.
    Input(char),
    Backspace,
    Newline,
    Submit,
    Escape,
    CursorLeft,
    CursorRight,
    CursorStart,
    CursorEnd,
    DeleteWord,
    DeleteToEnd,
    DeleteToStart,
    KillChar,
    HistoryPrev,
    HistoryNext,
    Undo,
    Redo,
    CursorWordLeft,
    CursorWordRight,
    PageUp,
    PageDown,
    GoToTop,
    GoToBottom,
    Paste(String),
    PasteImage,
    MouseClick { row: u16, col: u16, button: String },
    MouseRelease { row: u16, col: u16, button: String },
    MouseDrag { row: u16, col: u16, button: String },
    MouseMove { row: u16, col: u16 },
    MouseScrollUp,
    MouseScrollDown,
    FocusGained,
    FocusLost,
    TerminalSize { width: u16, height: u16 },

    // ── Permission intents ─────────────────────────────────────────────────

    /// Respond to a permission request.
    PermissionResponse { request_id: String, action: crate::permissions::PermissionAction },
}
