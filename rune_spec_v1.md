# Rune Specification v1.0

> A compiler driver that makes JavaScript / TypeScript / JSX / TSX a valid, zero-overhead source language for Rust.

---

## 1. Philosophy

**Rune is not a preprocessor.** It is a compiler driver that sits between your `.r.ts` files and `rustc`.

You write `*.r.ts` and `*.r.tsx` files natively inside Rust crates. `cargo rune dev` compiles them to a hot-reloadable dylib. `cargo rune build` compiles them to a static release binary. At no point do generated `.rs` files pollute your source tree.

---

## 2. Architecture

### 2.1 The Compiler Driver

```
.r.ts / .r.tsx  ──►  SWC Parser  ──►  Rune Analyzer  ──►  Rust Codegen  ──►  rustc  ──►  binary
     │                    │                │                    │
     │                    │                │                    └── writes to target/rune-cache/
     │                    │                └── borrow check, subset validation
     │                    └── produces TS AST (valid JS/TS)
     └── you edit this
```

Rune is implemented as a **`cargo` subcommand** (`cargo-rune`) and a **standalone compiler** (`rune rustc` wrapper).

### 2.2 Project Layout

```
todox/
├── Cargo.toml                  # workspace = ["crates/*"]
├── rune.toml                   # Rune compiler configuration
│
└── crates/
    ├── protocol/               # SHARED RLIB (host + app contract)
    │   └── src/lib.rs          # AppState + App trait
    │
    ├── host/                   # THIN BINARY (~80 lines, rarely edited)
    │   └── src/main.rs         # event loop + dylib loader
    │
    └── app/                    # HOT-RELOADABLE CDYLIB
        ├── Cargo.toml          # crate-type = ["cdylib"]
        └── src/
            ├── lib.rs          # hand-written wiring layer (~15 lines)
            ├── main.r.ts       # 👉 Rune entry point
            ├── state.r.ts      # 👉 Rune state logic
            ├── handlers/
            │   ├── keyboard.r.ts
            │   └── api.r.ts
            ├── views/
            │   ├── root.r.tsx
            │   └── task_list.r.tsx
            └── native/         # 👉 HAND-WRITTEN RUST (coexists)
                ├── mod.rs
                └── fast_math.rs
```

**Rules:**
- `*.r.ts` and `*.r.tsx` live anywhere in the crate tree, mixed with `.rs` files.
- `native/` contains hand-written Rust. Rune imports from it via `import { foo } from "native:bar"`.
- **No `.generated/` folder in source tree.** All ephemeral output goes to `target/rune-cache/`.

### 2.3 Build Pipeline

#### Development (`cargo rune dev`)

```
1. Scan crates/app/src/ for *.r.ts / *.r.tsx
2. SWC parse → validate Rune subset → TS AST
3. Codegen Rust source → write to target/rune-cache/crates/app/src/
   (mirrors original directory structure)
4. Generate shadow Cargo.toml in target/rune-cache/crates/app/
   (copies dependencies, sets crate-type = ["cdylib"])
5. cargo build --manifest-path target/rune-cache/crates/app/Cargo.toml
6. Copy artifact to target/hot/libapp_<timestamp>.so
7. Atomic write path to target/hot/.current
8. Host polls .current, sees new dylib, unloads old, loads new
   AppState preserved in host heap
```

#### Release (`cargo rune build --release`)

```
1. Scan for *.r.ts / *.r.tsx
2. Transpile to target/rune-cache/crates/app/src/
3. Generate static lib.rs that inlines generated modules + native modules
4. cargo build --release -p app (static rlib, not cdylib)
5. Link with host into single binary
6. Zero generated artifacts in source tree. Zero runtime overhead.
```

---

## 3. The Rune Subset

### 3.1 Core Constraint

> **Every Rune file is valid ECMAScript / TypeScript / JSX.** It parses in SWC, Babel, and `tsc` without errors. The "subset" is enforced by the Rune analyzer after parsing.

### 3.2 Type System

Rune uses standard TypeScript type syntax. The compiler infers Rust mappings.

#### Primitives

| Rune (TS) | Rust | Inference Rule |
|---|---|---|
| `number` | `f64` | Default floating-point. |
| `number` (integer literal `5`) | `i32` | Literal without decimal → `i32`. |
| `number` (bitwise context) | `i32` | Used with `\|`, `&`, `^`, `<<`, `>>` → `i32`. |
| `number` (array index) | `usize` | Cast inserted at index boundary. |
| `bigint` | `i64` | `123n` syntax. |
| `string` | `String` | Heap-allocated, UTF-8. |
| `string` (literal, read-only) | `&str` | String literal not mutated → borrow. |
| `boolean` | `bool` | `true` / `false`. |
| `null` | `Option<T>` | Only valid as `T \| null`. No bare `null`. |
| `undefined` | `()` | Void function returns. Missing arg → `Option<T>`. |

**Type annotations are optional for locals, required for exported function signatures.**

```typescript
// main.r.ts
const count = 0;           // inferred i32
const rate = 3.14;         // inferred f64
const name = "TODOX";      // inferred &str (literal, read-only)
const active = true;       // inferred bool

// Exported: explicit types required
export function add(a: number, b: number): number {
  return a + b;
}
```

**Emitted Rust:**
```rust
let count: i32 = 0;
let rate: f64 = 3.14;
let name: &str = "TODOX";
let active: bool = true;

pub fn add(a: f64, b: f64) -> f64 { a + b }
```

#### The Integer Division Rule (Critical Semantic Difference)

JavaScript: `5 / 2 === 2.5`
Rune: If both operands are inferred as integers, `/` emits Rust integer division (`5 / 2 == 2`).

**To get float division, ensure one operand is f64:**
```typescript
const a = 5;        // i32
const b = 2;        // i32
const c = a / b;    // i32 = 2 (integer division)

const d = a / 2.0;  // f64 = 2.5 (float division)
const e = a as number / b;  // explicit cast to f64
```

**The compiler emits a warning when integer division is inferred.**

#### Structs

Fixed-shape object types only. No dynamic property addition.

```typescript
// state.r.ts
export type Point = {
  x: number,
  y: number,
};

export type Task = {
  id: number,
  title: string,
  done: boolean,
};
```

**Emitted:**
```rust
pub struct Point { pub x: f64, pub y: f64 }
#[derive(Clone)]
pub struct Task { pub id: i32, pub title: String, pub done: bool }
```

#### Tagged Unions (Enums)

Standard discriminated unions. Mandatory `tag` (or `kind`) field.

```typescript
// handlers/keyboard.r.ts
export type Message =
  | { tag: "Move", x: number, y: number }
  | { tag: "Quit" }
  | { tag: "Write", text: string };

export function handle(msg: Message): number {
  switch (msg.tag) {
    case "Move": return msg.x + msg.y;
    case "Quit": return 0;
    case "Write": return msg.text.length;
  }
}
```

**Emitted:**
```rust
pub enum Message {
    Move { x: f64, y: f64 },
    Quit,
    Write { text: String },
}

pub fn handle(msg: &Message) -> i32 {
    match msg {
        Message::Move { x, y } => (*x as i32) + (*y as i32),
        Message::Quit => 0,
        Message::Write { text } => text.len() as i32,
    }
}
```

#### Arrays and Tuples

```typescript
const nums: number[] = [1, 2, 3];     // Vec<i32> (inferred from literals)
const names: string[] = [];           // Vec<String>

// Tuple: fixed arity, heterogeneous
const pair: [string, number] = ["hello", 5];  // (String, i32)
```

#### Option

```typescript
function find(tasks: Task[], id: number): Task | null {
  for (const t of tasks) {
    if (t.id === id) return t;
  }
  return null;
}

// Usage with narrowing (valid TS, zero overhead)
const t = find(tasks, 5);
if (t !== null) {
  console.log(t.title);  // t narrowed to Task
}
```

**Emitted:**
```rust
pub fn find(tasks: &Vec<Task>, id: i32) -> Option<&Task> {
    for t in tasks { if t.id == id { return Some(t); } }
    None
}

let t = find(&tasks, 5);
if let Some(t) = t {
    println!("{}", t.title);
}
```

#### Result (No Exceptions)

`try / catch / throw` is forbidden. Use discriminated result objects.

```typescript
function divide(a: number, b: number):
  | { ok: true, value: number }
  | { ok: false, error: string }
{
  if (b === 0) {
    return { ok: false, error: "division by zero" };
  }
  return { ok: true, value: a / b };
}

function caller():
  | { ok: true, value: number }
  | { ok: false, error: string }
{
  const r = divide(10, 2);
  if (!r.ok) {
    return r;  // error propagation
  }
  return { ok: true, value: r.value + 1 };
}
```

**Emitted:**
```rust
pub fn divide(a: f64, b: f64) -> Result<f64, String> {
    if b == 0.0 { return Err(String::from("division by zero")); }
    Ok(a / b)
}

pub fn caller() -> Result<f64, String> {
    let r = divide(10.0, 2.0)?;
    Ok(r + 1.0)
}
```

The compiler recognizes the `if (!r.ok) return r;` pattern and emits the `?` operator.

---

### 3.3 Functions

#### Basics

```typescript
function add(a: number, b: number): number {
  return a + b;
}

// Arrow functions (inferred types allowed for non-exported)
const mult = (a, b) => a * b;

// Rest params
function logAll(...items: string[]): void {
  for (const item of items) {
    console.log(item);
  }
}
```

#### Generics (Monomorphized)

```typescript
function first<T>(arr: T[]): T | null {
  return arr.length > 0 ? arr[0] : null;
}

function printArea<T extends { area(): number }>(shape: T): void {
  console.log(shape.area());
}
```

**Emitted:**
```rust
pub fn first<T>(arr: &Vec<T>) -> Option<&T> {
    if !arr.is_empty() { Some(&arr[0]) } else { None }
}

pub fn print_area<T: Drawable>(shape: &T) {
    println!("{}", shape.area());
}
```

#### Async (Futures)

```typescript
async function fetchData(url: string): Promise<string> {
  const resp = await httpGet(url);
  return resp.body;
}
```

**Emitted:**
```rust
pub async fn fetch_data(url: &str) -> String {
    let resp = http_get(url).await;
    resp.body
}
```

---

### 3.4 Ownership & Borrowing (Inferred, Zero Syntax)

The compiler runs borrow-check analysis on the JS/TS AST. You write normal JS. The compiler decides `&T`, `&mut T`, or owned `T`.

#### Bindings

| Rune | Rust | Rule |
|---|---|---|
| `const x = ...` | `let x = ...` | Immutable binding. |
| `let x = ...` | `let mut x = ...` | Mutable binding. |

#### Function Parameters

```typescript
// Only reads → immutable borrow
function strlen(s: string): number {
  return s.length;
}

// Mutates → mutable borrow
function push(arr: number[], val: number): void {
  arr.push(val);
}

// Takes ownership (returns derived value)
function consume(s: string): string {
  return s + "!";
}
```

**Emitted:**
```rust
pub fn strlen(s: &String) -> f64 { s.len() as f64 }
pub fn push(arr: &mut Vec<i32>, val: i32) { arr.push(val); }
pub fn consume(s: String) -> String { s + "!" }
```

#### Move Errors

If you use a value after passing it to a consuming function, the compiler errors:

```typescript
const s = "hello";
consume(s);
console.log(s);  // Rune ERROR: borrow of moved value: `s`
```

Fix: explicit clone (valid JS method, recognized by compiler):

```typescript
consume(s.clone());
console.log(s);  // OK
```

#### Closures

Closures capture by reference. Mutable captures require `FnMut`.

```typescript
let mut count = 0;
const increment = () => {
  count += 1;  // mutable capture
};
increment();
```

**Emitted:**
```rust
let mut count = 0;
let mut increment = || { count += 1; };
increment();
```

---

### 3.5 Control Flow

#### Pattern Matching

Standard `switch` with exhaustiveness checking.

```typescript
function handle(msg: Message): number {
  switch (msg.tag) {
    case "Move": return msg.x + msg.y;
    case "Quit": return 0;
    case "Write": return msg.text.length;
  }
}
```

#### Loops

```typescript
// Iterator-based
for (const item of items) {
  console.log(item);
}

// Index loop
for (let i = 0; i < items.length; i++) {
  console.log(items[i]);
}

while (active) {
  // ...
}
```

---

### 3.6 JSX / TSX (UI Layer)

Standard JSX syntax. Emitted as ratatui builder chains with zero overhead.

Supported widgets: `Paragraph`, `Block`, `List`, `ListItem` (extensible fallback for unknown tags).

| JSX attribute | Ratatui mapping |
|---|---|
| `text="…"` | `Widget::new("…")` |
| `title="…"` | `.title("…")` |
| `borders="ALL"` | `.borders(Borders::ALL)` |
| `border_type="rounded"` | `.border_type(BorderType::Rounded)` |
| `alignment="center"` | `.alignment(Alignment::Center)` |
| `style={…}` | `.style(…)` |

Nesting rules:
- `<Block>` wrapping a child → `.block(Block::bordered()…)` on the child
- `<List>` with `<ListItem>` children → `List::new(vec![…])`
- `<ListItem>` text children → `ListItem::new(text)`

```tsx
// views/root.r.tsx
interface Props {
  tasks: Task[];
  selected: number;
}

export function RootView(props: Props): Widget {
  return (
    <Block title="TODOX" borders="ALL">
      <List selected={props.selected}>
        {props.tasks.map((task, i) => (
          <ListItem bold={i === props.selected}>
            {task.done ? "[x] " : "[ ] "}
            {task.title}
          </ListItem>
        ))}
      </List>
    </Block>
  );
}
```

**Emitted:**
```rust
use ratatui::layout::Alignment;
use ratatui::widgets::{Block, Borders, List, ListItem};
use ratatui::style::{Style, Modifier};

pub struct RootViewProps<'a> {
    pub tasks: &'a Vec<Task>,
    pub selected: usize,
}

pub fn root_view(props: &RootViewProps) -> impl ratatui::widgets::Widget {
    let items: Vec<ListItem> = props.tasks.iter().enumerate().map(|(i, task)| {
        let text = format!("{}{}", if task.done { "[x] " } else { "[ ] " }, task.title);
        ListItem::new(text)
    }).collect();

    List::new(items)
        .block(Block::bordered().title("TODOX").borders(Borders::ALL))
        .highlight_symbol(">")
}
```

---

### 3.7 Module System

Standard ES modules. File-based.

```typescript
// state.r.ts
export type Task = { id: number; title: string; done: boolean };
export function toggle(t: Task): Task {
  return { ...t, done: !t.done };  // spread allowed for structs
}

// main.r.ts
import { Task, toggle } from "./state.r.ts";
```

**Native interop:**
```typescript
// Import from hand-written Rust
import { fastSqrt } from "native:fast_math";

const x = fastSqrt(2.0);
```

**Emitted:**
```rust
use crate::native::fast_math::fast_sqrt;
let x = fast_sqrt(2.0);
```

---

### 3.8 Forbidden Features

The following parse as TypeScript but are rejected by the Rune analyzer:

| Feature | Rune Error |
|---|---|
| `any` | `Type 'any' requires dynamic dispatch. Use concrete types.` |
| `unknown` | Same as above. |
| `class` / `new` / `this` (prototypes) | `Classes and prototype inheritance are forbidden. Use plain objects and functions.` |
| `var` | `Use const or let.` |
| `==` (loose equality) | `Use ===.` |
| `try / catch / throw` | `Use Result<T,E> return pattern.` |
| `eval` / `with` | `Dynamic scoping forbidden.` |
| `typeof` / `instanceof` | `Runtime type inspection forbidden.` |
| `obj[key]` dynamic access | `Use Map<K,V> for dynamic keys.` |
| `delete` | `Use ownership and explicit drops.` |
| `for...in` on objects | `Use for...of with Object.keys() or Map.` |
| `arguments` | `Use rest parameters (...args).` |
| Implicit coercion | All types must be explicit or inferable. |

---

## 4. Native Rust Interop

### 4.1 Rune → Native

Rune calls hand-written Rust functions via the `native:` import prefix:

```typescript
import { fastSqrt, batchToggle } from "native:fast_math";
import { handleSignal } from "native:handlers";
```

The compiler resolves `native:fast_math` to `crate::native::fast_math` in the same crate.

### 4.2 Native → Rune

Hand-written Rust imports generated Rune modules via `crate::generated::*`:

```rust
// crates/app/src/native/fast_math.rs
use crate::generated::state::Task;

pub fn batch_toggle(tasks: &mut Vec<Task>) {
    for t in tasks {
        t.done = !t.done;
    }
}
```

### 4.3 The Wiring Layer (`lib.rs`)

```rust
// crates/app/src/lib.rs (hand-written, ~15 lines)
mod native;
mod generated;  // resolved by Rune compiler to target/rune-cache/

use protocol::{App, AppState};

pub struct AppImpl;

impl App for AppImpl {
    fn update(&mut self, state: &mut AppState) {
        generated::main::update(state);
    }
    fn render(&self, term: &mut Terminal, state: &AppState) {
        generated::views::root::render(term, state);
    }
}

#[no_mangle]
pub extern "C" fn create_app() -> *mut dyn App {
    Box::into_raw(Box::new(AppImpl))
}
```

---

## 5. Hot Reload Protocol

### 5.1 State Ownership

The **host binary** owns `AppState`. The **dylib** is stateless logic only.

```rust
// crates/host/src/main.rs
use libloading::{Library, Symbol};
use protocol::{App, AppState};

fn main() {
    let mut state = AppState::default();
    let mut app = load_dylib();

    loop {
        app.update(&mut state);
        app.render(&mut terminal, &state);

        if new_dylib_detected() {
            drop(app);      // destroy old instance
            app = load_dylib();  // load new logic, state survives
        }
    }
}
```

### 5.2 Dylib Swapping

Rune writes versioned dylibs to avoid overwrite-while-mapped issues:

```bash
target/hot/
  ├── libapp_1715900000000.so
  ├── libapp_1715900001000.so
  └── .current   ← atomic symlink to latest
```

The host polls `.current`. On change, it unloads the old `Library`, loads the new one, and calls `create_app()`.

### 5.3 Protocol Change (Full Restart)

If `crates/protocol/src/lib.rs` changes (ABI break), `cargo rune dev`:
1. Serializes `AppState` to `/tmp/rune_state_<pid>.json`
2. Kills host process
3. Rebuilds protocol + host + app
4. Restarts host
5. Deserializes state into new `AppState` struct

New fields fill with `Default`. Removed fields are lost.

---

## 6. CLI Commands

```bash
# Development: watch .r.ts, hot reload dylib
cargo rune dev

# Release: static binary, zero overhead
cargo rune build --release

# Type check without emitting
cargo rune check

# Transpile to stdout (for debugging)
cargo rune transpile crates/app/src/main.r.ts

# Initialize new Rune project
cargo rune init
```

---

## 7. Configuration (`rune.toml`)

```toml
[project]
name = "todox"
entry = "crates/app/src/main.r.ts"

[build]
# Target crate for hot reload
target_crate = "app"
# Host crate binary
host_crate = "host"

[dev]
hot_reload = true
# Debounce milliseconds
debounce = 100

[release]
# Static binary, no dylib
static = true
lto = true
```

---

## 8. Performance Characteristics

| Metric | Value |
|---|---|
| Runtime overhead | **Zero** (transpiles to Rust, compiled by rustc) |
| Memory footprint | Identical to hand-written Rust (~1-5MB for TUI apps) |
| Dispatch | Static monomorphization (no vtables by default) |
| Hot reload latency | ~500ms - 2s (transpile + incremental compile) |
| Dylib call overhead | ~1-2ns (negligible for 16ms UI frames) |
| Release binary | Single static executable, no runtime TS support |

---

## 9. Known Semantic Gaps & Mitigations

| JS Behavior | Rune Behavior | Mitigation |
|---|---|---|
| `5 / 2 === 2.5` | `5 / 2 == 2` (integer div) | Compiler warns. Use `5 / 2.0` or explicit cast. |
| `"hello" + 5` | Type error | Compiler emits `format!("{}{}", s, n)`. |
| `if ("")` is false | `String` is not boolean | Compiler emits `!s.is_empty()`. |
| `obj[key]` dynamic | Forbidden | Use `Map<K,V>` or arrays. |
| `try/catch` | Forbidden | Use `Result<T,E>` pattern. |
| `class` / `this` | Forbidden | Use plain objects + functions. |

---

## 10. Error Messages

The compiler translates Rust borrow checker errors back to `.r.ts` line numbers:

```
error[E0382]: borrow of moved value: `s`
  --> crates/app/src/main.r.ts:12:15
   |
11 | consume(s);
   |         - value moved here
12 | console.log(s);
   |               ^ value borrowed here after move
   |
   = help: consider cloning: `consume(s.clone());`
```

---

## 11. Comparison

| Aspect | Pure Rust | Rune | Node.js |
|---|---|---|---|
| Hot Reload | ❌ No | ✅ Yes (dylib) | ✅ Yes |
| Performance | ⚡ Native | ⚡ Native | 🐢 V8 overhead |
| Memory | ~1-5MB | ~1-5MB | ~50-100MB |
| Syntax Verbosity | High | Low (TS) | Low (JS) |
| Type Safety | Maximum | Maximum (subset) | Weak |
| Build Time | Medium | Medium+ (transpile) | Fast |
| Ecosystem | crates.io | crates.io + npm types | npm |

---

## 12. Adoption Strategy

1. **Start with UI**: Convert `views/*.rs` to `*.r.tsx` for Ratatui widgets. Immediate hot reload win.
2. **Migrate logic gradually**: Move state handlers from `.rs` to `.r.ts` as you gain confidence.
3. **Keep algorithms in Rust**: Native `.rs` for performance-critical math, parsing, I/O.
4. **Lock down for release**: `cargo rune build --release` produces a static binary indistinguishable from pure Rust.

---

*Rune makes Rust feel like a live environment without sacrificing its zero-cost guarantees.*
