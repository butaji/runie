use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

// ── Event taxonomy generation ─────────────────────────────────────────────────

/// Generate event taxonomy Rust source from `taxonomy.json`.
/// Produces: generated/kind.rs, generated/category.rs, generated/facts.rs,
/// generated/intent_impl.rs.
fn generate_event_taxonomy(manifest_dir: &Path) -> Result<(), String> {
    let taxonomy_path = manifest_dir.join("src/event/taxonomy.json");
    let json = fs::read_to_string(&taxonomy_path)
        .map_err(|e| format!("failed to read taxonomy.json: {}", e))?;
    let taxonomy: serde_json::Value = serde_json::from_str(&json)
        .map_err(|e| format!("failed to parse taxonomy.json: {}", e))?;

    let out_dir = manifest_dir.join("src/event/generated");
    fs::create_dir_all(&out_dir)
        .map_err(|e| format!("failed to create generated dir: {}", e))?;

    generate_kind_rs(&taxonomy, &out_dir)?;
    generate_category_rs(&taxonomy, &out_dir)?;
    generate_facts_rs(&taxonomy, &out_dir)?;
    generate_intent_impl_rs(&taxonomy, &out_dir)?;

    Ok(())
}

fn generate_kind_rs(taxonomy: &serde_json::Value, out_dir: &Path) -> Result<(), String> {
    let categories = taxonomy["categories"].as_object().unwrap();
    let mut intent_variants = Vec::new();
    let mut fact_variants = Vec::new();
    let mut control_variants = Vec::new();

    for (_cat_name, cat_obj) in categories {
        let kind = cat_obj["kind"].as_str().unwrap();

        // Collect variants from each array separately
        let mut all_variants = Vec::new();

        // intent_variants are always Intent kind
        if let Some(arr) = cat_obj.get("intent_variants").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    intent_variants.push(s.to_string());
                }
            }
        }
        // fact_variants are always Fact kind
        if let Some(arr) = cat_obj.get("fact_variants").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    fact_variants.push(s.to_string());
                }
            }
        }
        // variants take the category's kind
        if let Some(arr) = cat_obj.get("variants").and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    all_variants.push(s.to_string());
                }
            }
        }

        match kind {
            "Intent" => intent_variants.extend(all_variants),
            "Fact" => fact_variants.extend(all_variants),
            "Control" => control_variants.extend(all_variants),
            other => return Err(format!("unknown kind: {}", other)),
        }
    }

    let mut out = String::new();
    out.push_str("//! `Event::kind()` impl and `EVENT_NAMES` bindable-variant table.\n");
    out.push_str("//! Generated from `taxonomy.json`. DO NOT EDIT.\n\n");
    out.push_str("use super::super::kind::EventKind;\n");
    out.push_str("use super::super::variants::Event;\n\n");
    out.push_str("// ── Event → Kind ──────────────────────────────────────────────────────────────\n\n");
    out.push_str("impl Event {\n");
    out.push_str("    /// Return the kind for this event variant.\n");
    out.push_str("    pub fn kind(&self) -> EventKind {\n");
    out.push_str("        match self {\n");

    for v in &intent_variants {
        out.push_str(&format!("            Event::{}{} => EventKind::Intent,\n",
            v, pattern_suffix_for(v)));
    }
    out.push_str("            // Fact variants\n");
    for v in &fact_variants {
        out.push_str(&format!("            Event::{}{} => EventKind::Fact,\n",
            v, pattern_suffix_for(v)));
    }
    out.push_str("            // Control variants\n");
    for v in &control_variants {
        out.push_str(&format!("            Event::{}{} => EventKind::Control,\n",
            v, pattern_suffix_for(v)));
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n\n");

    // Named variants (zero-arg) for EVENT_NAMES
    let named: Vec<_> = intent_variants.iter()
        .chain(control_variants.iter())
        .filter(|v| !has_fields(v))
        .collect();

    out.push_str("/// Zero-argument event constructor signature.\n");
    out.push_str("pub type EventCtor = fn() -> Event;\n\n");
    out.push_str("/// Bindable event names paired with their zero-arg constructors.\n");
    out.push_str("pub const EVENT_NAMES: &[(&str, EventCtor)] = &[\n");
    for v in &named {
        out.push_str(&format!("    (\"{}\", || Event::{}),\n", v, v));
    }
    out.push_str("];\n");

    let path = out_dir.join("kind.rs");
    fs::write(&path, &out).map_err(|e| format!("failed to write kind.rs: {}", e))?;
    eprintln!("  generated {}", path.display());
    Ok(())
}

fn generate_category_rs(taxonomy: &serde_json::Value, out_dir: &Path) -> Result<(), String> {
    let categories = taxonomy["categories"].as_object().unwrap();

    let mut out = String::new();
    out.push_str("//! `EventCategory` enum and `Event::category()` mapping.\n");
    out.push_str("//! Generated from `taxonomy.json`. DO NOT EDIT.\n\n");
    out.push_str("use super::super::variants::Event;\n\n");
    out.push_str("// ── EventCategory enum ─────────────────────────────────────────────────────────\n\n");
    out.push_str("/// Event category — routing taxonomy for the dispatcher.\n");
    out.push_str("#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, strum::Display, strum::IntoStaticStr, strum::VariantNames)]\n");
    out.push_str("pub enum EventCategory {\n");
    let mut cats: Vec<_> = categories.keys().collect();
    cats.sort();
    for cat in &cats {
        out.push_str(&format!("    {},\n", cat));
    }
    out.push_str("    #[default]\n");
    out.push_str("    Unknown,\n");
    out.push_str("}\n\n");

    out.push_str("// ── Event → Category ─────────────────────────────────────────────────────────\n\n");
    out.push_str("impl Event {\n");
    out.push_str("    /// Return the category for this event variant.\n");
    out.push_str("    pub fn category(&self) -> EventCategory {\n");
    out.push_str("        match self {\n");

    for (cat_name, cat_obj) in categories {
        // Collect variants from all arrays
        let mut variants = Vec::new();
        for key in ["intent_variants", "fact_variants", "variants"] {
            if let Some(arr) = cat_obj.get(key).and_then(|v| v.as_array()) {
                for v in arr {
                    if let Some(s) = v.as_str() {
                        variants.push(s.to_string());
                    }
                }
            }
        }
        for v in &variants {
            out.push_str(&format!("            Event::{}{} => EventCategory::{},\n",
                v, pattern_suffix_for(v), cat_name));
        }
    }
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    let path = out_dir.join("category.rs");
    fs::write(&path, &out).map_err(|e| format!("failed to write category.rs: {}", e))?;
    eprintln!("  generated {}", path.display());
    Ok(())
}

fn generate_facts_rs(taxonomy: &serde_json::Value, out_dir: &Path) -> Result<(), String> {
    let categories = taxonomy["categories"].as_object().unwrap();
    let mut fact_variants = Vec::new();

    for cat_obj in categories.values() {
        let kind = cat_obj["kind"].as_str().unwrap();
        if kind == "Fact" {
            fact_variants.extend(collect_variants(cat_obj));
        }
    }

    let mut out = String::new();
    out.push_str("//! `is_fact_variant()` fast-path predicate.\n");
    out.push_str("//! Generated from `taxonomy.json`. DO NOT EDIT.\n\n");
    out.push_str("use super::super::variants::Event;\n\n");
    out.push_str("/// Returns true if this event is a fact (not an intent or control).\n");
    out.push_str("pub fn is_fact_variant(e: &Event) -> bool {\n");
    out.push_str("    matches!(\n        e,\n");
    for (i, v) in fact_variants.iter().enumerate() {
        if i > 0 {
            out.push_str("        | ");
        } else {
            out.push_str("        ");
        }
        out.push_str(&matches_pattern(v));
        out.push_str("\n");
    }
    out.push_str("    )\n");
    out.push_str("}\n");

    let path = out_dir.join("facts.rs");
    fs::write(&path, &out).map_err(|e| format!("failed to write facts.rs: {}", e))?;
    eprintln!("  generated {}", path.display());
    Ok(())
}

fn generate_intent_impl_rs(taxonomy: &serde_json::Value, out_dir: &Path) -> Result<(), String> {
    let categories = taxonomy["categories"].as_object().unwrap();

    // Build event → intent name lookup from taxonomy
    let renames: HashMap<String, String> = taxonomy["intent_renames"]
        .as_object()
        .unwrap()
        .iter()
        .filter(|(k, _)| !k.starts_with('_'))
        .map(|(k, v)| (k.clone(), v.as_str().unwrap().to_string()))
        .collect();

    let skips: Vec<String> = taxonomy["intent_skips"]["_list"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();

    // Collect ALL fact variant names.
    // A variant is a fact if:
    // 1. It appears in a `fact_variants` array (overrides category kind), OR
    // 2. It belongs to a `kind: "Fact"` category.
    // These must NOT appear in intent match arms.
    let mut all_fact_variants: Vec<String> = categories
        .values()
        .filter_map(|cat_obj| cat_obj.get("fact_variants").and_then(|v| v.as_array()))
        .flat_map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)))
        .collect();
    for cat_obj in categories.values() {
        let kind = cat_obj["kind"].as_str().unwrap();
        if kind == "Fact" {
            all_fact_variants.extend(collect_variants(cat_obj));
        }
    }
    let all_fact_set: std::collections::HashSet<&str> =
        all_fact_variants.iter().map(|s| s.as_str()).collect();

    // Collect all events that have an Intent counterpart,
    // EXCLUDING fact_variants (they have no Intent counterpart).
    let mut has_intent: Vec<String> = Vec::new();
    for cat_obj in categories.values() {
        let kind = cat_obj["kind"].as_str().unwrap();
        if kind == "Intent" || kind == "Control" {
            let variants = collect_variants(cat_obj);
            has_intent.extend(
                variants
                    .into_iter()
                    .filter(|v| !all_fact_set.contains(v.as_str())),
            );
        }
    }

    let mut out = String::new();
    out.push_str("//! `Event::into_intent()` implementation.\n");
    out.push_str("//! Generated from `taxonomy.json`. DO NOT EDIT.\n\n");
    out.push_str("use super::super::intent::Intent;\n");
    out.push_str("use super::super::variants::Event;\n");
    out.push_str("use super::facts::is_fact_variant;\n\n");
    out.push_str("impl Event {\n");
    out.push_str("    /// Convert this event to a typed `Intent`, if it is an intent.\n");
    out.push_str("    pub fn into_intent(self) -> Option<Intent> {\n");
    out.push_str("        if is_fact_variant(&self) {\n");
    out.push_str("            return None;\n");
    out.push_str("        }\n");
    out.push_str("        match self {\n");

    for event_name in &has_intent {
        if skips.contains(event_name) {
            continue;
        }
        let intent_name = renames.get(event_name).map(|s| s.as_str()).unwrap_or(event_name);
        let arm = build_intent_arm(event_name, intent_name);
        out.push_str("            ");
        out.push_str(&arm);
        out.push('\n');
    }

    // Catch-all for any variants not covered above (shouldn't happen with correct taxonomy).
    out.push_str("            _ => None,\n");
    out.push_str("        }\n");
    out.push_str("    }\n");
    out.push_str("}\n");

    let path = out_dir.join("intent_impl.rs");
    fs::write(&path, &out).map_err(|e| format!("failed to write intent_impl.rs: {}", e))?;
    eprintln!("  generated {}", path.display());
    Ok(())
}

/// Build a match arm converting Event→Intent for a given event/intent name pair.
///
/// All struct fields are bound as VALUES in the match pattern (not references),
/// so we use `.clone()` — NOT `*field` — to construct the Intent variant.
fn build_intent_arm(event_name: &str, intent_name: &str) -> String {
    match event_name {
        "Input" => "Event::Input(c) => Some(Intent::Input(c)),".to_string(),
        "Paste" => "Event::Paste(s) => Some(Intent::Paste(s)),".to_string(),
        "CommandFormInput" => "Event::CommandFormInput(c) => Some(Intent::CommandFormInput(c)),".to_string(),
        "PaletteFilter" => "Event::PaletteFilter(c) => Some(Intent::PaletteFilter(c)),".to_string(),
        "ModelSelectorFilter" => "Event::ModelSelectorFilter(c) => Some(Intent::ModelSelectorFilter(c)),".to_string(),
        "PaletteUp" => "Event::PaletteUp => Some(Intent::PaletteUp),".to_string(),
        "PaletteDown" => "Event::PaletteDown => Some(Intent::PaletteDown),".to_string(),
        "PaletteSelect" => "Event::PaletteSelect => Some(Intent::PaletteSelect),".to_string(),
        "PaletteClose" => "Event::PaletteClose => Some(Intent::PaletteClose),".to_string(),
        "ModelSelectorUp" => "Event::ModelSelectorUp => Some(Intent::ModelSelectorUp),".to_string(),
        "ModelSelectorDown" => "Event::ModelSelectorDown => Some(Intent::ModelSelectorDown),".to_string(),
        "ModelSelectorSelect" => "Event::ModelSelectorSelect => Some(Intent::ModelSelectorSelect),".to_string(),
        "ModelSelectorClose" => "Event::ModelSelectorClose => Some(Intent::ModelSelectorClose),".to_string(),
        "PathCompletionUp" => "Event::PathCompletionUp => Some(Intent::PathCompletionUp),".to_string(),
        "PathCompletionDown" => "Event::PathCompletionDown => Some(Intent::PathCompletionDown),".to_string(),
        "PathCompletionSelect" => "Event::PathCompletionSelect => Some(Intent::PathCompletionSelect),".to_string(),
        "PathCompletionClose" => "Event::PathCompletionClose => Some(Intent::PathCompletionClose),".to_string(),
        "CopyToClipboard" => "Event::CopyToClipboard(s) => Some(Intent::CopyToClipboard(s)),".to_string(),
        "InsertAtRef" => "Event::InsertAtRef(s) => Some(Intent::InsertAtRef(s)),".to_string(),
        "Submit" => "Event::Submit => Some(Intent::Submit),".to_string(),
        // RunForkCommand: Intent expects String, Event has String (no parse needed)
        "RunForkCommand" => "Event::RunForkCommand { message_index } => Some(Intent::RunForkCommand { message_index: message_index.clone() }),".to_string(),
        "RunCompactCommand" => "Event::RunCompactCommand { keep, focus } => Some(Intent::RunCompactCommand { keep: keep.clone(), focus: focus.clone() }),".to_string(),
        "ForkSession" => "Event::ForkSession { message_index } => Some(Intent::ForkSession { message_index }),".to_string(),
        "SessionTreeSelect" => "Event::SessionTreeSelect { id } => Some(Intent::SessionTreeSelect { id: id.clone() }),".to_string(),
        "SelectSession" => "Event::SelectSession { id } => Some(Intent::SelectSession { id: id.clone() }),".to_string(),
        "StarSession" => "Event::StarSession { id } => Some(Intent::StarSession { id: id.clone() }),".to_string(),
        "RenameSession" => "Event::RenameSession { id, name } => Some(Intent::RenameSession { id: id.clone(), name: name.clone() }),".to_string(),
        "DeleteSession" => "Event::DeleteSession { id } => Some(Intent::DeleteSession { id: id.clone() }),".to_string(),
        // NOTE: struct fields are bound BY VALUE in the match pattern (not references),
        // so we must NOT dereference Copy types (u16, bool) or owned types (enum variants).
        // - For Copy types and owned types: bind with `ref`, deref to copy/move into Intent
        // - For owned String/Vec fields: bind by value, clone into Intent
        "TerminalSize" => "Event::TerminalSize { ref width, ref height } => Some(Intent::TerminalSize { width: *width, height: *height }),".to_string(),
        "SwitchModel" => "Event::SwitchModel { ref provider, ref model, ref explicit } => Some(Intent::SwitchModel { provider: (*provider).clone(), model: (*model).clone(), explicit: *explicit }),".to_string(),
        "SettingsSwitchCategory" => "Event::SettingsSwitchCategory { ref category } => Some(Intent::SettingsSwitchCategory { category: (*category).clone() }),".to_string(),
        "PermissionResponse" => "Event::PermissionResponse { ref request_id, ref action } => Some(Intent::PermissionResponse { request_id: request_id.clone(), action: (*action).clone() }),".to_string(),
        "ScopedModelToggle" => "Event::ScopedModelToggle { provider, name } => Some(Intent::ScopedModelToggle { provider: provider.clone(), name: name.clone() }),".to_string(),
        "ScopedModelToggleProvider" => "Event::ScopedModelToggleProvider { provider } => Some(Intent::ScopedModelToggleProvider { provider: provider.clone() }),".to_string(),
        // SetThinkingLevel: Intent takes `ThinkingLevel` (owned), Event has `ThinkingLevel` (owned in tuple)
        "SetThinkingLevel" => "Event::SetThinkingLevel(lvl) => Some(Intent::SetThinkingLevel(lvl)),".to_string(),
        "RunLoadCommand" => "Event::RunLoadCommand { name } => Some(Intent::RunLoadCommand { name: name.clone() }),".to_string(),
        "RunSaveCommand" => "Event::RunSaveCommand { name } => Some(Intent::RunSaveCommand { name: name.clone() }),".to_string(),
        "RunDeleteCommand" => "Event::RunDeleteCommand { name } => Some(Intent::RunDeleteCommand { name: name.clone() }),".to_string(),
        "RunImportCommand" => "Event::RunImportCommand { path } => Some(Intent::RunImportCommand { path: path.clone() }),".to_string(),
        "RunExportCommand" => "Event::RunExportCommand { path } => Some(Intent::RunExportCommand { path: path.clone() }),".to_string(),
        "RunSkillCommand" => "Event::RunSkillCommand { name } => Some(Intent::RunSkillCommand { name: name.clone() }),".to_string(),
        "RunLoginCommand" => "Event::RunLoginCommand { provider, token } => Some(Intent::RunLoginCommand { provider: provider.clone(), token: token.clone() }),".to_string(),
        "RunLogoutCommand" => "Event::RunLogoutCommand { provider } => Some(Intent::RunLogoutCommand { provider: provider.clone() }),".to_string(),
        "RunNameCommand" => "Event::RunNameCommand { name } => Some(Intent::RunNameCommand { name: name.clone() }),".to_string(),
        "RunPromptCommand" => "Event::RunPromptCommand { name } => Some(Intent::RunPromptCommand { name: name.clone() }),".to_string(),
        "RunThinkingCommand" => "Event::RunThinkingCommand { level } => Some(Intent::RunThinkingCommand { level: level.clone() }),".to_string(),
        "RunPaletteCommand" => "Event::RunPaletteCommand { name, args } => Some(Intent::RunPaletteCommand { name: name.clone(), args: args.clone() }),".to_string(),
        "SelectProvider" => "Event::SelectProvider { provider } => Some(Intent::SelectProvider { provider: provider.clone() }),".to_string(),
        "SubmitKey" => "Event::SubmitKey { provider, key } => Some(Intent::SubmitKey { provider: provider.clone(), key: key.clone() }),".to_string(),
        "ToggleModel" => "Event::ToggleModel { model } => Some(Intent::ToggleModel { model: model.clone() }),".to_string(),
        "ExternalEditorDone" => "Event::ExternalEditorDone { content } => Some(Intent::ExternalEditorDone { content: content.clone() }),".to_string(),
        "MouseClick" => "Event::MouseClick { ref row, ref col, ref button } => Some(Intent::MouseClick { row: *row, col: *col, button: button.clone() }),".to_string(),
        "MouseRelease" => "Event::MouseRelease { ref row, ref col, ref button } => Some(Intent::MouseRelease { row: *row, col: *col, button: button.clone() }),".to_string(),
        "MouseDrag" => "Event::MouseDrag { ref row, ref col, ref button } => Some(Intent::MouseDrag { row: *row, col: *col, button: button.clone() }),".to_string(),
        "MouseMove" => "Event::MouseMove { ref row, ref col } => Some(Intent::MouseMove { row: *row, col: *col }),".to_string(),
        "PendingEdit" => "Event::PendingEdit { path, original, proposed } => Some(Intent::PendingEdit { path: path.clone(), original: original.clone(), proposed: proposed.clone() }),".to_string(),
        "ProvidersSelectModel" => "Event::ProvidersSelectModel { provider, model } => Some(Intent::ProvidersSelectModel { provider: provider.clone(), model: model.clone() }),".to_string(),
        "ProvidersDisconnect" => "Event::ProvidersDisconnect { provider } => Some(Intent::ProvidersDisconnect { provider: provider.clone() }),".to_string(),
        "ProvidersEditModels" => "Event::ProvidersEditModels { provider } => Some(Intent::ProvidersEditModels { provider: provider.clone() }),".to_string(),
        // Events that map to Notify
        "TransientMessage" => {
            "Event::TransientMessage { content, level } => Some(Intent::Notify { content: content.clone(), level: *level }),".to_string()
        }
        "TransientError" => {
            "Event::TransientError { content } => Some(Intent::Notify { content: content.clone(), level: super::TransientLevel::Error }),".to_string()
        }
        // Renamed unit variants (no fields)
        "Start" => "Event::Start => Some(Intent::LoginStart),".to_string(),
        "Save" => "Event::Save => Some(Intent::LoginSave),".to_string(),
        "Cancel" => "Event::Cancel => Some(Intent::LoginCancel),".to_string(),
        "SwitchTheme" => "Event::SwitchTheme { name } => Some(Intent::SetTheme { name: name.clone() }),".to_string(),
        "Up" => "Event::Up => Some(Intent::ScrollUp),".to_string(),
        "Down" => "Event::Down => Some(Intent::ScrollDown),".to_string(),
        "TrustProject" => "Event::TrustProject => Some(Intent::TrustProject),".to_string(),
        "UntrustProject" => "Event::UntrustProject => Some(Intent::UntrustProject),".to_string(),
        "ReloadAll" => "Event::ReloadAll => Some(Intent::ReloadConfig),".to_string(),
        // Simple zero-arg events (intent name matches event name)
        _ if event_name == intent_name && !has_fields(event_name) => {
            format!("Event::{} => Some(Intent::{}),", event_name, intent_name)
        }
        // Simple events with same name but different fields (auto-convert)
        _ if event_name == intent_name => {
            let fields = guess_fields(event_name);
            format!("Event::{}{} => Some(Intent::{}{}),", event_name, fields, intent_name, fields)
        }
        // Renamed events with fields
        _ => {
            let fields = guess_fields(event_name);
            format!("Event::{}{} => Some(Intent::{}{}),", event_name, fields, intent_name, fields)
        }
    }
}

/// Heuristic: does this event variant have fields?
fn has_fields(name: &str) -> bool {
    matches!(name,
        "Input" | "Paste" | "MouseClick" | "MouseRelease" |
        "MouseDrag" | "MouseMove" | "TerminalSize" | "SwitchModel" | "SwitchTheme" |
        "SetThinkingLevel" | "ScopedModelToggle" | "ScopedModelToggleProvider" |
        "SettingsSwitchCategory" | "ForkSession" | "SelectSession" | "StarSession" |
        "RenameSession" | "DeleteSession" | "ExternalEditorDone" | "PendingEdit" |
        "TransientMessage" | "TransientError" | "RunLoadCommand" | "RunSaveCommand" |
        "RunDeleteCommand" | "RunImportCommand" | "RunExportCommand" | "RunSkillCommand" |
        "RunLoginCommand" | "RunLogoutCommand" | "RunNameCommand" | "RunForkCommand" |
        "RunCompactCommand" | "RunPromptCommand" | "RunThinkingCommand" |
        "RunPaletteCommand" | "SelectProvider" | "SubmitKey" | "ToggleModel" |
        "SessionTreeSelect" | "SessionList" | "SessionOperationFailed" |
        "SessionChanged" | "SessionLoaded" | "SessionSaved" | "SessionDeleted" |
        "SessionImported" | "SessionExported" | "PaletteFilter" | "CommandFormInput" |
        "ModelSelectorFilter" | "CopyToClipboard" | "InsertAtRef" |
        "ProvidersSelectModel" | "ProvidersDisconnect" | "ProvidersEditModels" |
        "PermissionResponse" | "AssistantMessageReady" | "BashOutput" |
        "ClipboardRead" | "ClipboardWritten" | "CompletionChanged" | "ConfigLoaded" |
        "Done" | "EnvDetected" | "Error" | "ExternalEditorClosed" |
        "FffSearchResult" | "FilesWritten" | "FollowUpDelivered" |
        "GistShared" | "HistoryAppend" | "HistoryLoaded" | "InputChanged" |
        "MessageDequeued" | "MessageReplayed" | "ModelsFetched" |
        "PermissionRequest" | "QueueAborted" | "ReadOnlyChanged" | "Response" |
        "ResponseDelta" | "SetPrompt" | "SteeringDelivered" | "StreamStarted" |
        "SystemMessage" | "TextEnd" | "TextStart" | "Thinking" |
        "ThinkingDelta" | "ThinkingEnd" | "ThinkingStart" | "ThoughtDone" |
        "TokenStatsUpdated" | "ToolConstraintError" | "ToolEnd" |
        "ToolInputDelta" | "ToolStart" | "TrustChanged" | "TrustLoaded" |
        "TrustSet" | "TurnComplete" | "TurnConstraintError" | "TurnErrored" |
        "TurnStarted" | "UserMessageSubmitted" | "ValidationFailed" | "ViewChanged" |
        "IdGenerated"
    )
}

/// Guess field pattern for a named event variant.
fn guess_fields(name: &str) -> String {
    match name {
        "Input" => "(c)".to_string(),
        "Paste" => "(s)".to_string(),
        "MouseClick" | "MouseRelease" | "MouseDrag" => " { row, col, button }".to_string(),
        "MouseMove" => " { row, col }".to_string(),
        "TerminalSize" => " { width, height }".to_string(),
        "SwitchModel" => " { provider, model, explicit }".to_string(),
        "SwitchTheme" => " { name }".to_string(),
        "SetThinkingLevel" => "(lvl)".to_string(),
        "ScopedModelToggle" | "ScopedModelToggleProvider" => " { provider, name }".to_string(),
        "ForkSession" => " { message_index }".to_string(),
        "SelectSession" | "StarSession" | "DeleteSession" | "SessionTreeSelect" => " { id }".to_string(),
        "RenameSession" => " { id, name }".to_string(),
        "ExternalEditorDone" => " { content }".to_string(),
        "PendingEdit" => " { path, original, proposed }".to_string(),
        "TransientMessage" => " { content, level }".to_string(),
        "TransientError" => " { content }".to_string(),
        "RunLoadCommand" | "RunSaveCommand" | "RunDeleteCommand" | "RunNameCommand" | "RunPromptCommand" => " { name }".to_string(),
        "RunImportCommand" | "RunExportCommand" => " { path }".to_string(),
        "RunLoginCommand" => " { provider, token }".to_string(),
        "RunLogoutCommand" => " { provider }".to_string(),
        "RunForkCommand" => " { message_index }".to_string(),
        "RunCompactCommand" => " { keep, focus }".to_string(),
        "RunThinkingCommand" => " { level }".to_string(),
        "RunPaletteCommand" => " { name, args }".to_string(),
        "SelectProvider" | "ToggleModel" => " { provider }".to_string(),
        "SubmitKey" => " { provider, key }".to_string(),
        "PaletteFilter" | "CommandFormInput" | "ModelSelectorFilter" => "(c)".to_string(),
        "CopyToClipboard" | "InsertAtRef" => "(s)".to_string(),
        "ProvidersSelectModel" => " { provider, model }".to_string(),
        "ProvidersDisconnect" | "ProvidersEditModels" => " { provider }".to_string(),
        // NOTE: PermissionResponse is handled explicitly in build_intent_arm.
        // NOTE: SettingsSwitchCategory is handled explicitly in build_intent_arm.
        "ValidationFailed" => " { provider, key, error }".to_string(),
        "ModelsFetched" => " { provider, key, models }".to_string(),
        "SessionLoaded" => " { name, events, metadata }".to_string(),
        "SessionSaved" => " { name }".to_string(),
        "SessionDeleted" => " { name }".to_string(),
        "SessionImported" => " { session }".to_string(),
        "SessionExported" => " { path }".to_string(),
        "SessionList" => " { sessions }".to_string(),
        "SessionOperationFailed" => " { operation, error }".to_string(),
        "SessionChanged" => " { state }".to_string(),
        _ => " { /* unknown */ }".to_string(),
    }
}

/// Collect all variant names from a category JSON value.
fn collect_variants(cat_val: &serde_json::Value) -> Vec<String> {
    let mut variants = Vec::new();
    for key in ["intent_variants", "fact_variants", "variants"] {
        if let Some(arr) = cat_val.get(key).and_then(|v| v.as_array()) {
            for v in arr {
                if let Some(s) = v.as_str() {
                    variants.push(s.to_string());
                }
            }
        }
    }
    variants
}

/// Build a match pattern suffix for a variant name.
/// Returns ` { .. }` for struct/tuple variants, `""` for unit variants.
fn pattern_suffix_for(name: &str) -> String {
    if has_fields(name) { " { .. }".to_string() } else { String::new() }
}

/// Build a pattern for the `matches!` macro.
/// Unit variants use the variant name directly; struct variants use `{ .. }`.
fn matches_pattern(name: &str) -> String {
    if has_fields(name) { format!("Event::{} {{ .. }}", name) } else { format!("Event::{}", name) }
}

// ── AppState field-access guardrail ──────────────────────────────────────────
//
// Private AppState fields must be accessed through accessors, not directly.
const APPSTATE_PATTERNS: &[(&str, &str)] = &[
    ("state.session.", "state.session()"),
    ("state.input.", "state.input()"),
    ("state.agent.", "state.agent_state()"),
    ("state.view.", "state.view()"),
    ("state.config.", "state.config()"),
    ("state.completion.", "state.completion()"),
    ("state.should_quit ", "state.should_quit_mut()"),
    ("state.should_quit\n", "state.should_quit_mut()"),
    ("state.should_quit{", "state.should_quit_mut()"),
    ("state.open_dialog ", "state.open_dialog_mut()"),
    ("state.open_dialog.", "state.open_dialog_mut()"),
    ("state.dialog_back_stack.", "state.dialog_back_stack_mut()"),
    ("state.login_flow ", "state.login_flow_mut()"),
    ("state.login_flow.", "state.login_flow_mut()"),
    ("state.transient_message ", "state.transient_message_mut()"),
    ("state.transient_until ", "state.transient_until_mut()"),
    ("state.transient_level ", "state.transient_level_mut()"),
    ("state.fff_file_results.", "state.fff_file_results()"),
    ("state.fff_debounce ", "state.fff_debounce_mut()"),
    ("state.perm_req ", "state.permission_request_opt()"),
    ("state.perm_req.", "state.permission_request_opt()."),
    ("state.cwd_name ", "state.cwd_name_mut()"),
    ("state.git_info ", "state.git_info_mut()"),
    ("state.git_info.", "state.git_info_mut()"),
    ("state.skills ", "state.skills_mut()"),
    ("state.prompts ", "state.prompts_mut()"),
    ("state.trust_decisions ", "state.trust_decisions_mut()"),
    ("state.trust_decisions.", "state.trust_decisions_mut()"),
    ("state.actor_handles ", "state.actor_handles_mut()"),
    ("state.registry ", "state.registry_mut()"),
    ("state.registry.", "state.registry_mut()"),
    // self.xxx patterns (same replacement, different prefix)
    ("self.session.", "self.session()"),
    ("self.input.", "self.input()"),
    ("self.agent.", "self.agent_state()"),
    ("self.view.", "self.view()"),
    ("self.config.", "self.config()"),
    ("self.completion.", "self.completion()"),
    ("self.should_quit ", "self.should_quit_mut()"),
    ("self.should_quit\n", "self.should_quit_mut()"),
    ("self.should_quit{", "self.should_quit_mut()"),
    ("self.open_dialog ", "self.open_dialog_mut()"),
    ("self.open_dialog.", "self.open_dialog_mut()"),
    ("self.dialog_back_stack.", "self.dialog_back_stack_mut()"),
    ("self.login_flow ", "self.login_flow_mut()"),
    ("self.login_flow.", "self.login_flow_mut()"),
    ("self.transient_message ", "self.transient_message_mut()"),
    ("self.transient_until ", "self.transient_until_mut()"),
    ("self.transient_level ", "self.transient_level_mut()"),
    ("self.fff_file_results.", "self.fff_file_results_mut()"),
    ("self.fff_debounce ", "self.fff_debounce_mut()"),
    ("self.permission_request ", "self.permission_request_mut()"),
    ("self.cwd_name ", "self.cwd_name_mut()"),
    ("self.git_info ", "self.git_info_mut()"),
    ("self.git_info.", "self.git_info_mut()"),
    ("self.skills ", "self.skills_mut()"),
    ("self.prompts ", "self.prompts_mut()"),
    ("self.trust_decisions ", "self.trust_decisions_mut()"),
    ("self.trust_decisions.", "self.trust_decisions_mut()"),
    ("self.actor_handles ", "self.actor_handles_mut()"),
    ("self.registry ", "self.registry_mut()"),
    ("self.registry.", "self.registry_mut()"),
];

fn find_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                files.extend(find_rust_files(&path));
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    files
}

fn relative_path(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn is_test_file(rel_path: &str) -> bool {
    rel_path.contains("/tests/")
        || rel_path.ends_with("/tests.rs")
        || rel_path.ends_with("_tests.rs")
        || rel_path.ends_with("_test.rs")
        || rel_path.contains("_tests.")
        || rel_path.contains("_test.")
}

fn needs_appstate_lint(rel_path: &str) -> bool {
    let exemptions = [
        "build.rs",
        "accessors.rs",
        "domain_ops.rs",
        "actors/config/actor.rs",
        "actors/config/ractor_config.rs",
        "actors/permission/actor.rs",
        "actors/permission/ractor_permission.rs",
        "actors/input/actor.rs",
        "actors/input/messages.rs",
        "actors/ui_control/actor.rs",
        "actors/handles.rs",
        "actors/leader/actor.rs",
        "update/input/text.rs",
        "update/input/submit.rs",
        "retry.rs",
        "session/replay.rs",
        "login_flow/validation.rs",
        "model/state/input.rs",
    ];
    !is_test_file(rel_path)
        && !rel_path.contains("/benches/")
        && !rel_path.contains("/harness_skills/")
        && !exemptions.iter().any(|e| rel_path.ends_with(e))
}

fn check_appstate_field_access(
    rel_path: &str,
    lines: &[&str],
    errors: &mut Vec<String>,
) {
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//")
            || trimmed.starts_with("/*")
            || trimmed.starts_with('*')
        {
            continue;
        }
        for (pattern, suggestion) in APPSTATE_PATTERNS {
            if line.contains(pattern) {
                errors.push(format!(
                    "{}:{}: direct AppState field access `{pattern}` — use {suggestion}",
                    rel_path,
                    i + 1
                ));
                break;
            }
        }
    }
}

fn lint_file(path: &Path, workspace_root: &Path, errors: &mut Vec<String>) {
    let rel_path = relative_path(path, workspace_root);
    if needs_appstate_lint(&rel_path) {
        let content = fs::read_to_string(path).unwrap();
        let lines: Vec<_> = content.lines().collect();
        check_appstate_field_access(&rel_path, &lines, errors);
    }
}

fn main() {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_default());
    let workspace_root = manifest_dir.parent().unwrap().parent().unwrap();

    // Generate event taxonomy from taxonomy.json
    eprintln!("cargo:rerun-if-changed=src/event/taxonomy.json");
    if let Err(msg) = generate_event_taxonomy(&manifest_dir) {
        eprintln!("\n=== EVENT TAXONOMY GENERATION FAILED ===\n  {}\n\n", msg);
        process::exit(1);
    }

    // Validate bundled subagent type checksums.
    if let Err(msg) =
        validate_agent_manifest(manifest_dir.join("resources").join("agents"))
    {
        eprintln!("\n=== AGENT MANIFEST VALIDATION FAILED ===\n  {}\n\n", msg);
        process::exit(1);
    }

    let mut errors = Vec::new();
    let crates_path = workspace_root.join("crates");

    for path in find_rust_files(&crates_path) {
        if !path.to_string_lossy().contains("target/") {
            lint_file(&path, workspace_root, &mut errors);
        }
    }

    if !errors.is_empty() {
        eprintln!("\n=== RUNIE LINT VIOLATIONS ===\n");
        for err in &errors {
            eprintln!("  {}", err);
        }
        eprintln!("\n{} violations found\n", errors.len());
        process::exit(1);
    }
}

/// Validate that all files in `resources/agents/manifest.json` match their
/// stored SHA-256 checksums.
fn validate_agent_manifest(agents_dir: PathBuf) -> Result<(), String> {
    let manifest_path = agents_dir.join("manifest.json");
    let manifest_json =
        fs::read_to_string(&manifest_path).map_err(|e| format!("failed to read manifest.json: {}", e))?;

    #[derive(serde::Deserialize)]
    struct Manifest {
        files: std::collections::HashMap<String, String>,
    }
    let manifest: Manifest = serde_json::from_str(&manifest_json)
        .map_err(|e| format!("failed to parse manifest.json: {}", e))?;

    use sha2::{Digest, Sha256};
    for (filename, expected_hash) in &manifest.files {
        let file_path = agents_dir.join(filename);
        let content =
            fs::read(&file_path).map_err(|e| format!("failed to read {}: {}", filename, e))?;
        let mut hasher = Sha256::new();
        hasher.update(&content);
        let actual = hex::encode(hasher.finalize());
        if &actual != expected_hash {
            return Err(format!(
                "checksum mismatch for {}: expected {}, got {}",
                filename, expected_hash, actual
            ));
        }
    }
    Ok(())
}
