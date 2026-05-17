# Rune Compiler — Architecture Fix Plan

## Status: Partially Validated

The review accurately identifies architectural problems. Key findings:

| Issue | Status | Impact |
|-------|--------|--------|
| Module references | ✅ VALID - files exist | Low |
| Analyzer-codegen disconnect | ✅ VALIDATED | **Critical** |
| Box::from_raw null check | ✅ VALIDATED | **Critical** |
| No signal handling | ✅ VALIDATED | **Critical** |
| Float f64 suffix | ✅ VALIDATED | High |
| SubsetValidator first-only | ✅ VALIDATED | Medium |
| Ownership stubbed | ✅ VALIDATED | High |
| Hardcoded function inference | ✅ VALIDATED | High |
| JSX stub | ✅ VALIDATED | High |
| Error translator regex | ✅ VALIDATED | Medium |

---

## Phase 1: Critical Fixes (Week 1) ✅ COMPLETED

### 1. Connect Analyzer → Codegen Pipeline ⏳ DEFERRED

**Problem**: `RustEmitter` clones `AnalysisResult` but ignores it, re-parses with SWC.

Status: **Deferred to Phase 2** - Requires significant refactoring of AstWalker.

### 2. Null Check Box::from_raw ✅ FIXED

**Files changed**:
- `crates/rune/src/driver/templates/template_host.rs`
- `examples/todox/crates/host/src/main.rs`

```rust
fn create_app(&self) -> Box<dyn App> {
    unsafe {
        let ptr = (self.creator)();
        if ptr.is_null() {
            panic!("create_app() returned null pointer - dylib may be malformed");
        }
        Box::from_raw(ptr)
    }
}
```

### 3. Signal Handling for Watch Loop ✅ FIXED

**Files changed**:
- `Cargo.toml` - Added `ctrlc` dependency
- `crates/rune/Cargo.toml` - Added `ctrlc` workspace reference
- `crates/rune/src/driver/watch.rs` - Added SIGINT/SIGTERM handlers

```rust
let running = Arc::new(AtomicBool::new(true));
let r = running.clone();

ctrlc::set_handler(move || {
    r.store(false, Ordering::SeqCst);
}).expect("Error setting Ctrl-C handler");

while running.load(Ordering::SeqCst) {
    // Watch loop
}
```

### 4. Float Literal Suffix ✅ FIXED

**File changed**: `crates/rune/src/codegen/emitter/literals.rs`

```rust
Lit::Num(n) => {
    if n.value.fract() == 0.0 && n.value.abs() < f64::from(i32::MAX) {
        emitter.push_str(&format!("{}i32", n.value as i32));
    } else {
        emitter.push_str(&format!("{}_f64", n.value));  // Now emits f64 suffix
    }
}
```

### 5. Return All Validation Errors ✅ FIXED

**Files changed**:
- `crates/rune/src/analyzer/validator/validation/mod.rs`
- `crates/rune/src/analyzer/mod.rs`

Changed `validate()` to return `Result<(), Vec<ValidationError>>` instead of `Result<(), ValidationError>`. All check functions now return `()` and accumulate errors.

---

## Phase 2: High Priority (Week 2)

### 5. Wire OwnershipAnalysis into Codegen

**Problem**: `OwnershipAnalyzer` produces unused data.

**Fix**: Add parameter emission based on borrow mode:
```rust
fn emit_param(&self, name: &str, mode: BorrowMode) -> String {
    match mode {
        BorrowMode::Owned => format!("{}: {}", name, self.infer_type(name)),
        BorrowMode::Shared => format!("&{}: &{}", name, self.infer_type(name)),
        BorrowMode::Mut => format!("&mut {}: &mut {}", name, self.infer_type(name)),
        // ...
    }
}
```

### 6. Return All Validation Errors

**Problem** (validator/mod.rs):
```rust
pub fn validate(&mut self, source: &SourceFile) -> Result<(), ValidationError> {
    for line in source.lines() {
        self.check_forbidden_features(line, line_num)?;
        // Early return on first error!
    }
    // ...
    Err(self.errors[0].clone())  // Only first
}
```

**Fix**: Return `Vec<ValidationError>` or use an iterator pattern.

### 7. Replace Hardcoded Function Inference

**Problem**: 25+ function names hardcoded, rest returns `()`.

**Fix**: Infer from `AnalysisResult::types` or implement real call graph analysis.

---

## Phase 3: Medium Priority (Week 3)

### 6. SystemTime Unwrap ✅ FIXED

**File changed**: `crates/rune/src/reload/host.rs`

```rust
let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .map(|d| d.as_millis() as u64)
    .unwrap_or(0);  // Now handles clock before epoch gracefully
```

### 8. Error Translator → JSON Mode

**Problem**: Regex parsing of rustc text output.

**Fix**: Use `rustc --error-format=json`:
```rust
let output = Command::new("rustc")
    .args(["--error-format=json", file.as_str()])
    .output()?;
let errors: Vec<rustc_errors::JsonSpan> = serde_json::from_slice(&output.stderr)?;
```

### 9. Platform-Specific Dylib Extensions

**Problem**: Hardcoded `.so` everywhere.

**Fix**:
```rust
fn dylib_extension() -> &'static str {
    #[cfg(target_os = "macos")]
    return ".dylib";
    #[cfg(target_os = "windows")]
    return ".dll";
    #[cfg(target_os = "linux")]
    return ".so";
}
```

### 10. Fix SystemTime Unwrap

**Problem** (host.rs):
```rust
let timestamp = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()  // Can panic!
    .as_millis();
```

**Fix**:
```rust
match SystemTime::now().duration_since(UNIX_EPOCH) {
    Ok(d) => d.as_millis() as u64,
    Err(_) => 0,  // or use std::time::Instant
}
```

---

## Phase 4: Test Coverage (Week 4)

### 11. Real Parser Tests

**Current**: Manual `SourceFile` construction, no actual parsing.

**Fix**: Add integration tests:
```rust
#[test]
fn test_parse_and_analyze() {
    let temp_dir = TempDir::new().unwrap();
    let ts_file = temp_dir.path().join("test.r.ts");
    fs::write(&ts_file, "export function add(a: number): number { return a; }").unwrap();
    
    let files = scan_directory(temp_dir.path()).unwrap();
    assert!(!files.is_empty());
    
    let ast = SwcAst::parse_ts(&files[0].source, "test").unwrap();
    assert!(!ast.module.body.is_empty());
}
```

### 12. End-to-End Pipeline Tests

Add tests that go through: Parse → Analyze → Codegen → Verify output.

---

## Deferred / Long Term

- Abstract traits for Parser, Analyzer, Emitter
- Replace file-based signaling with Unix sockets
- Proper benchmark suite
- Loader process for Linux dylib unloading
