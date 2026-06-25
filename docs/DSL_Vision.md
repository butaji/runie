# Declarative DSL Vision for Runie

The long-term goal is to make most new features a matter of **declaring what they are**, not wiring *how* they mutate state. The DSL layer turns UI, commands, and agentic behavior into small, composable data descriptions that the runtime executes.

## The shape of the DSL

At the center are three primitives:

- **Intent** — what the user or system wants to happen.
- **Fact** — what an actor decided actually happened.
- **Command / Dialog / Behavior** — declarative blueprints bound to intents and facts.

```rust
use runie_core::dsl::{Intent, Fact, on};

// A user keystroke becomes an intent.
let flow = on(KeyEvent::char('/'))
    .intent(Intent::OpenCommandPalette)
    .fact(Fact::ViewInvalidated);
```

## 1. Commands today vs. commands with the DSL

### Today (imperative)

A new command currently touches at least three places:

```rust
// 1. Add a variant to the command enum
pub enum Command {
    Quit,
    ToggleTheme,
    MyNewCommand { arg: String }, // <-- new
}

// 2. Register it in the command registry
registry.insert("my-new", Command::MyNewCommand { arg: "".into() });

// 3. Handle it inside update/command.rs
match cmd {
    Command::MyNewCommand { arg } => {
        state.set_my_thing(arg);
        state.ensure_fresh();
    }
    _ => {}
}
```

### With the DSL

A command becomes a single declarative definition:

```rust
CommandDef::new("my-new")
    .desc("Do the new thing")
    .category(CommandCategory::System)
    .dialog(form()
        .field("arg", Field::string().label("Argument"))
        .submit(Intent::RunMyNewCommand))
    .on_execute(Intent::RunMyNewCommand)
```

The runtime takes care of:
- palette registration,
- argument parsing,
- dialog opening,
- intent routing.

Adding a command means adding **one data structure**, not three manual edits.

## 2. Dialogs today vs. dialogs with the DSL

### Today

Dialog state is built imperatively and threaded through `AppState`:

```rust
state.open_dialog = Some(DialogState::MyForm {
    arg: String::new(),
});
state.push_dialog_back_stack(...);
```

### With the DSL

A dialog is a declarative form panel:

```rust
form("New Provider")
    .field("name", Field::string().validate(non_empty))
    .field("api_key", Field::secret())
    .field("model", Field::select(["gpt-4o", "claude"]))
    .on_submit(Intent::AddProvider {
        name: field("name"),
        api_key: field("api_key"),
        model: field("model"),
    })
```

The DSL handles:
- field binding,
- validation,
- back-stack navigation,
- emitting the final intent.

## 3. Agentic behavior today vs. with intents/facts

### Today

Agent events are dispatched through `update/agent/mod.rs` and mutate `AppState` directly:

```rust
match event {
    AgentEvent::ToolStart { id, name, .. } => {
        state.start_tool(id, name);
        state.ensure_turn_complete_last();
    }
    AgentEvent::ToolEnd { output, .. } => {
        state.end_tool(output);
        state.ensure_turn_complete_last();
    }
}
```

### With the DSL

The agent produces intents; the `TurnActor` owns tool state and emits facts:

```rust
// Agent side: streaming event -> intent
on(AgentEvent::ToolStart { id, name })
    .intent(TurnIntent::ToolStart { id, name })

// TurnActor side: intent -> fact
fn handle(&mut self, intent: TurnIntent) -> Vec<TurnFact> {
    match intent {
        TurnIntent::ToolStart { id, name } => {
            self.tools.insert(id, ToolState::Running { name });
            vec![TurnFact::ToolStatusChanged { id }]
        }
        TurnIntent::ToolOutput { id, output } => {
            self.tools[&id].complete(output);
            vec![TurnFact::ToolCompleted { id }]
        }
    }
}
```

No `AppState` mutation. No `ensure_turn_complete_last` sprinkled in five places. The state machine lives in one actor.

## 4. UI rendering

UI is already pure in the desired end state:

```rust
fn draw(frame: &mut Frame, snapshot: &Snapshot) {
    let view = snapshot.view();
    let input = snapshot.input();

    frame.render_widget(Chat::new(view.messages()), layout.chat);
    frame.render_widget(InputLine::new(input.buffer()), layout.input);

    if let Some(dialog) = snapshot.dialog() {
        frame.render_widget(dialog.widget(), layout.dialog);
    }
}
```

The DSL makes this cheap to extend: a new panel is just a new `Fact` + a new widget branch in `draw`. No handler changes.

## 5. Keybindings

Keybindings become a declarative map from input events to intents:

```rust
keys! {
    Char('q') + Ctrl => Intent::Quit,
    Char(':')        => Intent::OpenCommandPalette,
    Esc              => Intent::CloseDialog,
    Tab              => Intent::CycleCompletion,
}
```

Adding a shortcut is a one-line declaration, not a new `handle_key` branch.

## 6. Adding a new feature end-to-end

Imagine adding a “/bookmark message” command.

With the DSL it is:

1. **Command definition** (`commands/user.rs`):
   ```rust
   CommandDef::new("bookmark")
       .desc("Bookmark the current assistant message")
       .intent(Intent::BookmarkMessage)
   ```

2. **Intent handling** (`SessionActor`):
   ```rust
   SessionIntent::BookmarkMessage { message_id } => {
       self.bookmarks.insert(message_id);
       vec![Fact::BookmarksChanged]
   }
   ```

3. **UI fact** (`draw`):
   ```rust
   if snapshot.bookmarks().contains(msg.id) {
       line.push_span(Span::from(" ★").style(Style::new().yellow()));
   }
   ```

Three small, independent declarations. No `AppState` edits, no dispatch boilerplate, no manual state sync.

## Where we are now

The DSL layer is already scaffolded in `crates/runie-core/src/dsl/` and the command/dialog builders exist. What is missing is making it the **default path**:

- Finish the actor-SSOT refactor so intents route to actors instead of `AppState`.
- Convert the existing command palette and dialogs to the declarative builders.
- Replace direct `AppState` mutations in handlers with intent emission.

Once that happens, adding a feature will look like the examples above instead of a multi-file refactor.
