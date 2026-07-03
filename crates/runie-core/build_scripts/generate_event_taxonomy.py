#!/usr/bin/env python3
"""Generate event taxonomy match tables from taxonomy.json.

This script generates Rust match tables for Event::kind(), Event::category(),
Event::into_intent(), and is_fact_variant() from the canonical taxonomy.json file.

Usage:
    python3 build_scripts/generate_event_taxonomy.py
"""
import json
import sys
from pathlib import Path

# Unit variants (no fields) — pattern is Event::{v}, not Event::{v} {{ .. }}
UNIT_VARIANTS = {
    "Abort", "ApproveEdit", "AtFilePicker", "Backspace", "Cancel", "ClearQueues",
    "ClearTransient", "CloneSession", "CommandFormBackspace", "CommandFormClose",
    "CommandFormDown", "CommandFormSubmit", "CommandFormUp", "CopyBlockMetadata",
    "CopyLastResponse", "CopySelectedBlock", "CursorEnd", "CursorLeft", "CursorRight",
    "CursorStart", "CursorWordLeft", "CursorWordRight", "CycleModelNext", "CycleModelPrev",
    "CycleThinkingLevel", "Dequeue", "DeleteToEnd", "DeleteToStart", "DeleteWord", "DialogBack",
    "Down", "FollowUp", "ForceQuit",
    "Escape", "FocusGained", "FocusLost", "GoToBottom", "GoToTop", "HistoryNext",
    "HistoryPrev", "KeybindingsReloaded", "KillChar", "ModelSelectorBackspace",
    "ModelSelectorClose", "ModelSelectorDown", "ModelSelectorSelect", "ModelSelectorUp",
    "MouseScrollDown", "MouseScrollUp", "NewSession", "Newline", "OpenExternalEditor",
    "OpenSessionList", "PageDown", "PageUp", "PaletteBackspace", "PaletteClose",
    "PaletteDown", "PaletteSelect", "PaletteUp", "PasteImage", "PathCompletionClose",
    "PathCompletionDown", "PathCompletionSelect", "PathCompletionUp",
    "PermissionRequestDismissed", "PlanModeDisabled", "ProcessResumed", "ProvidersAdd",
    "ProvidersDialog", "QueuesCleared", "Quit", "Redo", "RejectEdit", "ReloadAll", "Reset",
    "ResumeSession", "Save", "ScopedModelDisableAll", "ScopedModelEnableAll",
    "SessionTreeFilterCycle", "SettingsClose", "SettingsDown", "SettingsLeft",
    "SettingsRight", "SettingsSelect", "SettingsUp", "ShareSession", "ShowDiagnostics",
    "Start", "Submit", "Suspend", "ToggleCommandPalette", "ToggleExpand",
    "ToggleModelSelector", "TogglePathCompletion", "ToggleReadOnly",
    "ToggleScopedModelsDialog", "ToggleSessionTree", "ToggleSettingsDialog", "ToggleVimMode",
    "ToggleWelcome", "TrustProject", "TurnAborted", "TurnCompleted", "Undo", "UntrustProject",
    "Up",
}

# Variants with fields (non-unit variants) — for into_intent() return type
# Any variant with (char) or {{..}} pattern needs to use self.clone()
VARIANTS_WITH_FIELDS = {
    "SetPrompt", "RunLoadCommand", "RunSaveCommand", "RunDeleteCommand",
    "RunImportCommand", "RunExportCommand", "RunSkillCommand", "RunLoginCommand",
    "RunLogoutCommand", "RunNameCommand", "RunForkCommand", "RunCompactCommand",
    "RunPromptCommand", "RunThinkingCommand", "RunPaletteCommand",
    "PlanModeEnabled", "ExternalEditorDone", "SelectSession", "StarSession",
    "RenameSession", "DeleteSession", "ProvidersSelectModel", "ProvidersDisconnect",
    "ProvidersEditModels", "CopyToClipboard", "InsertAtRef", "PendingEdit",
    "Paste", "MouseClick", "MouseRelease", "MouseDrag", "MouseMove",
    "TerminalSize", "SelectProvider", "SubmitKey", "ToggleModel", "SwitchModel",
    "SwitchTheme", "ScopedModelToggle", "ScopedModelToggleProvider",
    "SettingsSwitchCategory", "SetThinkingLevel", "ForkSession", "SessionTreeSelect",
    "SessionLoaded", "SessionImported", "SessionList", "SessionOperationFailed",
    "SessionMessageAdded", "SessionMessageUpdated", "SessionMetadataUpdated",
    "SessionChanged", "SessionTreeSnapshot",
    # Intent variants with (char) field
    "Input", "PaletteFilter", "ModelSelectorFilter", "CommandFormInput",
    # Intent variants with {..} fields
    "PermissionResponse", "TransientMessage", "TransientError",
}

# Control variants that are also intents (from taxonomy.json intent_variants for Control)
CONTROL_INTENT_VARIANTS = {
    "Quit", "ForceQuit", "Reset", "Abort", "ClearQueues", "FollowUp",
    "ToggleExpand", "Dequeue", "OpenExternalEditor", "ExternalEditorDone",
    "ShareSession", "Suspend", "ToggleVimMode", "CopyLastResponse",
    "OpenSessionList", "NewSession", "ResumeSession", "SelectSession",
    "StarSession", "RenameSession", "DeleteSession",
}


def main():
    script_dir = Path(__file__).parent.resolve()
    # Go up from build_scripts/ -> runie-core/ -> crates/ -> workspace root
    core_src = script_dir.parent / "src"
    taxonomy_path = core_src / "event" / "taxonomy.json"
    output_path = core_src / "event" / "generated.rs"

    with open(taxonomy_path) as f:
        data = json.load(f)

    categories = data["categories"]

    # Build lookup tables
    variant_category = {}  # variant -> category
    variant_kind = {}     # variant -> "Intent" | "Fact" | "Control"
    intent_variants = []
    fact_variants = []

    for cat, cat_data in categories.items():
        cat_kind = cat_data.get("kind", "Fact")

        # Handle uniform categories (just "variants" key)
        if "variants" in cat_data:
            for v in cat_data["variants"]:
                variant_category[v] = cat
                if cat_kind == "Control":
                    variant_kind[v] = "Control"
                    if v in CONTROL_INTENT_VARIANTS:
                        intent_variants.append(v)
                else:
                    variant_kind[v] = cat_kind
                    if cat_kind == "Intent":
                        intent_variants.append(v)
                    else:
                        fact_variants.append(v)

        # Handle split categories (intent_variants + fact_variants)
        for v in cat_data.get("intent_variants", []):
            variant_category[v] = cat
            variant_kind[v] = "Intent"
            intent_variants.append(v)

        for v in cat_data.get("fact_variants", []):
            variant_category[v] = cat
            variant_kind[v] = "Fact"
            fact_variants.append(v)

    # intent_skips — Agent events that are Fact even though in Agent category
    intent_skips = set(data.get("intent_skips", {}).get("_list", []))
    for v in intent_skips:
        if v in intent_variants:
            intent_variants.remove(v)
        if v not in fact_variants:
            fact_variants.append(v)
        variant_kind[v] = "Fact"

    # Remove duplicates and sort for deterministic output
    intent_variants = sorted(set(intent_variants), key=lambda v: (variant_category.get(v, ""), v))
    fact_variants = sorted(set(fact_variants), key=lambda v: (variant_category.get(v, ""), v))

    def pattern(v: str) -> str:
        """Return match pattern: unit variant vs struct variant."""
        return f"Event::{v}" if v in UNIT_VARIANTS else f"Event::{v} {{ .. }}"

    # Generate kind() match arms
    kind_lines = []
    for v in sorted(variant_kind.keys()):
        kind = variant_kind[v]
        if kind == "Control":
            kind_lines.append(f"            {pattern(v)} => EventKind::Control,")
        elif kind == "Intent":
            kind_lines.append(f"            {pattern(v)} => EventKind::Intent,")
        else:
            kind_lines.append(f"            {pattern(v)} => EventKind::Fact,")

    # Generate category() match arms
    category_lines = []
    for v in sorted(variant_category.keys()):
        cat = variant_category[v]
        category_lines.append(f"            {pattern(v)} => EventCategory::{cat},")

    # Generate into_intent() match arms
    intent_lines = []
    for v in intent_variants:
        if v in VARIANTS_WITH_FIELDS:
            intent_lines.append(f"            Event::{v} {{ .. }} => Some(self.clone()),")
        else:
            intent_lines.append(f"            Event::{v} => Some(self),")

    # Generate is_fact_variant() match arms (pipe-separated)
    fact_lines = []
    for i, v in enumerate(fact_variants):
        if i == 0:
            fact_lines.append(f"            | {pattern(v)}")
        else:
            fact_lines.append(f"            | {pattern(v)}")

    # Write generated file
    with open(output_path, "w") as f:
        f.write("// AUTO-GENERATED from taxonomy.json — do not edit manually\n")
        f.write("// Regenerate: python3 build_scripts/generate_event_taxonomy.py\n\n")
        f.write("use super::{Event, EventCategory, EventKind};\n\n")
        f.write("impl Event {\n")
        f.write("    /// Return the [`EventKind`] for this event variant.\n")
        f.write("    pub fn kind(&self) -> EventKind {\n")
        f.write("        match self {\n")
        for line in kind_lines:
            f.write(line + "\n")
        f.write("        }\n")
        f.write("    }\n\n")
        f.write("    /// Return the [`EventCategory`] for this event variant.\n")
        f.write("    pub fn category(&self) -> EventCategory {\n")
        f.write("        match self {\n")
        for line in category_lines:
            f.write(line + "\n")
        f.write("        }\n")
        f.write("    }\n\n")
        f.write("    /// Convert this event to an intent [`Event`], if it maps to one.\n")
        f.write("    ///\n")
        f.write("    /// Returns `None` for Fact variants. Control variants like Quit, Reset, Abort\n")
        f.write("    /// are also convertible to intent.\n")
        f.write("    pub fn into_intent(self) -> Option<Event> {\n")
        f.write("        match self {\n")
        for line in intent_lines:
            f.write(line + "\n")
        f.write("            _ => None,\n")
        f.write("        }\n")
        f.write("    }\n")
        f.write("}\n\n")
        f.write("/// Returns true if this event is a fact (not an intent or control).\n")
        f.write("pub fn is_fact_variant(e: &Event) -> bool {\n")
        f.write("    matches!(\n")
        f.write("        e,\n")
        for line in fact_lines:
            f.write(line + "\n")
        f.write("    )\n")
        f.write("}\n\n")
        f.write("// ─────────────────────────────────────────────────────────────────────────────\n")
        f.write("// EVENT_NAMES — zero-arg constructor table for bindable variants\n")
        f.write("// ─────────────────────────────────────────────────────────────────────────────\n\n")
        f.write("/// Zero-argument event constructor signature.\n")
        f.write("pub type EventCtor = fn() -> Event;\n\n")
        f.write("/// Bindable event names paired with their zero-arg constructors.\n")
        f.write("///\n")
        f.write("/// These are events that can be constructed with no arguments, used for\n")
        f.write("/// keybinding resolution and command palette lookup.\n")
        f.write("pub const EVENT_NAMES: &[(&str, EventCtor)] = &[\n")
        f.write('    ("ToggleWelcome", || Event::ToggleWelcome),\n')
        f.write('    ("ToggleCommandPalette", || Event::ToggleCommandPalette),\n')
        f.write('    ("PaletteBackspace", || Event::PaletteBackspace),\n')
        f.write('    ("PaletteUp", || Event::PaletteUp),\n')
        f.write('    ("PaletteDown", || Event::PaletteDown),\n')
        f.write('    ("PaletteSelect", || Event::PaletteSelect),\n')
        f.write('    ("PaletteClose", || Event::PaletteClose),\n')
        f.write('    ("ToggleModelSelector", || Event::ToggleModelSelector),\n')
        f.write('    ("ModelSelectorBackspace", || Event::ModelSelectorBackspace),\n')
        f.write('    ("ModelSelectorUp", || Event::ModelSelectorUp),\n')
        f.write('    ("ModelSelectorDown", || Event::ModelSelectorDown),\n')
        f.write('    ("ModelSelectorSelect", || Event::ModelSelectorSelect),\n')
        f.write('    ("ModelSelectorClose", || Event::ModelSelectorClose),\n')
        f.write('    ("TogglePathCompletion", || Event::TogglePathCompletion),\n')
        f.write('    ("PathCompletionUp", || Event::PathCompletionUp),\n')
        f.write('    ("PathCompletionDown", || Event::PathCompletionDown),\n')
        f.write('    ("PathCompletionSelect", || Event::PathCompletionSelect),\n')
        f.write('    ("PathCompletionClose", || Event::PathCompletionClose),\n')
        f.write('    ("CommandFormBackspace", || Event::CommandFormBackspace),\n')
        f.write('    ("CommandFormUp", || Event::CommandFormUp),\n')
        f.write('    ("CommandFormDown", || Event::CommandFormDown),\n')
        f.write('    ("CommandFormSubmit", || Event::CommandFormSubmit),\n')
        f.write('    ("CommandFormClose", || Event::CommandFormClose),\n')
        f.write('    ("DialogBack", || Event::DialogBack),\n')
        f.write('    ("ProvidersDialog", || Event::ProvidersDialog),\n')
        f.write('    ("ProvidersAdd", || Event::ProvidersAdd),\n')
        f.write('    ("CopySelectedBlock", || Event::CopySelectedBlock),\n')
        f.write('    ("CopyBlockMetadata", || Event::CopyBlockMetadata),\n')
        f.write('    ("AtFilePicker", || Event::AtFilePicker),\n')
        f.write('    ("ApproveEdit", || Event::ApproveEdit),\n')
        f.write('    ("RejectEdit", || Event::RejectEdit),\n')
        f.write('    ("Backspace", || Event::Backspace),\n')
        f.write('    ("Newline", || Event::Newline),\n')
        f.write('    ("Submit", || Event::Submit),\n')
        f.write('    ("Escape", || Event::Escape),\n')
        f.write('    ("CursorLeft", || Event::CursorLeft),\n')
        f.write('    ("CursorRight", || Event::CursorRight),\n')
        f.write('    ("CursorStart", || Event::CursorStart),\n')
        f.write('    ("CursorEnd", || Event::CursorEnd),\n')
        f.write('    ("DeleteWord", || Event::DeleteWord),\n')
        f.write('    ("DeleteToEnd", || Event::DeleteToEnd),\n')
        f.write('    ("DeleteToStart", || Event::DeleteToStart),\n')
        f.write('    ("KillChar", || Event::KillChar),\n')
        f.write('    ("HistoryPrev", || Event::HistoryPrev),\n')
        f.write('    ("HistoryNext", || Event::HistoryNext),\n')
        f.write('    ("Undo", || Event::Undo),\n')
        f.write('    ("Redo", || Event::Redo),\n')
        f.write('    ("CursorWordLeft", || Event::CursorWordLeft),\n')
        f.write('    ("CursorWordRight", || Event::CursorWordRight),\n')
        f.write('    ("PageUp", || Event::PageUp),\n')
        f.write('    ("PageDown", || Event::PageDown),\n')
        f.write('    ("GoToTop", || Event::GoToTop),\n')
        f.write('    ("GoToBottom", || Event::GoToBottom),\n')
        f.write('    ("PasteImage", || Event::PasteImage),\n')
        f.write('    ("MouseScrollUp", || Event::MouseScrollUp),\n')
        f.write('    ("MouseScrollDown", || Event::MouseScrollDown),\n')
        f.write('    ("FocusGained", || Event::FocusGained),\n')
        f.write('    ("FocusLost", || Event::FocusLost),\n')
        f.write('    ("Start", || Event::Start),\n')
        f.write('    ("Save", || Event::Save),\n')
        f.write('    ("Cancel", || Event::Cancel),\n')
        f.write('    ("CycleModelNext", || Event::CycleModelNext),\n')
        f.write('    ("CycleModelPrev", || Event::CycleModelPrev),\n')
        f.write('    ("ToggleScopedModelsDialog", || Event::ToggleScopedModelsDialog),\n')
        f.write('    ("ScopedModelEnableAll", || Event::ScopedModelEnableAll),\n')
        f.write('    ("ScopedModelDisableAll", || Event::ScopedModelDisableAll),\n')
        f.write('    ("ToggleSettingsDialog", || Event::ToggleSettingsDialog),\n')
        f.write('    ("SettingsUp", || Event::SettingsUp),\n')
        f.write('    ("SettingsDown", || Event::SettingsDown),\n')
        f.write('    ("SettingsLeft", || Event::SettingsLeft),\n')
        f.write('    ("SettingsRight", || Event::SettingsRight),\n')
        f.write('    ("SettingsSelect", || Event::SettingsSelect),\n')
        f.write('    ("SettingsClose", || Event::SettingsClose),\n')
        f.write('    ("CycleThinkingLevel", || Event::CycleThinkingLevel),\n')
        f.write('    ("ToggleReadOnly", || Event::ToggleReadOnly),\n')
        f.write('    ("TrustProject", || Event::TrustProject),\n')
        f.write('    ("UntrustProject", || Event::UntrustProject),\n')
        f.write('    ("ReloadAll", || Event::ReloadAll),\n')
        f.write('    ("Up", || Event::Up),\n')
        f.write('    ("Down", || Event::Down),\n')
        f.write('    ("CloneSession", || Event::CloneSession),\n')
        f.write('    ("ToggleSessionTree", || Event::ToggleSessionTree),\n')
        f.write('    ("SessionTreeFilterCycle", || Event::SessionTreeFilterCycle),\n')
        f.write('    ("ClearTransient", || Event::ClearTransient),\n')
        f.write('    ("ShowDiagnostics", || Event::ShowDiagnostics),\n')
        f.write('    ("Quit", || Event::Quit),\n')
        f.write('    ("ForceQuit", || Event::ForceQuit),\n')
        f.write('    ("Reset", || Event::Reset),\n')
        f.write('    ("Abort", || Event::Abort),\n')
        f.write('    ("ClearQueues", || Event::ClearQueues),\n')
        f.write('    ("FollowUp", || Event::FollowUp),\n')
        f.write('    ("ToggleExpand", || Event::ToggleExpand),\n')
        f.write('    ("Dequeue", || Event::Dequeue),\n')
        f.write('    ("OpenExternalEditor", || Event::OpenExternalEditor),\n')
        f.write('    ("ShareSession", || Event::ShareSession),\n')
        f.write('    ("Suspend", || Event::Suspend),\n')
        f.write('    ("ToggleVimMode", || Event::ToggleVimMode),\n')
        f.write('    ("CopyLastResponse", || Event::CopyLastResponse),\n')
        f.write('    ("OpenSessionList", || Event::OpenSessionList),\n')
        f.write('    ("NewSession", || Event::NewSession),\n')
        f.write('    ("ResumeSession", || Event::ResumeSession),\n')
        f.write("];\n\n")
        f.write("// ─────────────────────────────────────────────────────────────────────────────\n")
        f.write("// Helper constructors for variants with optional fields\n")
        f.write("// ─────────────────────────────────────────────────────────────────────────────\n\n")
        f.write("impl Event {\n")
        f.write("    /// Create a Response with default durable fields.\n")
        f.write("    pub fn response(id: impl Into<String>, content: impl Into<String>) -> Self {\n")
        f.write("        Event::Response {\n")
        f.write("            id: id.into(),\n")
        f.write("            content: content.into(),\n")
        f.write("            role: String::new(),\n")
        f.write("            timestamp: 0.0,\n")
        f.write("            provider: String::new(),\n")
        f.write("        }\n")
        f.write("    }\n\n")
        f.write("    /// Create a ToolEnd with default input field.\n")
        f.write("    pub fn tool_end(\n")
        f.write("        id: impl Into<String>,\n")
        f.write("        duration_secs: f64,\n")
        f.write("        output: impl Into<String>,\n")
        f.write("    ) -> Self {\n")
        f.write("        Event::ToolEnd {\n")
        f.write("            id: id.into(),\n")
        f.write("            input: None,\n")
        f.write("            duration_secs,\n")
        f.write("            output: output.into(),\n")
        f.write("        }\n")
        f.write("    }\n")
        f.write("}\n")

    print(f"Generated {output_path} ({len(kind_lines)} kind arms, {len(intent_lines)} intent arms, {len(fact_lines)} fact arms)")


if __name__ == "__main__":
    main()
