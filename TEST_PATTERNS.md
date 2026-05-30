# Test Patterns for Runie

Extracted from codex, opencode, pi, and runie codebases.

## Table of Contents
1. [Harness Pattern](#1-harness-pattern)
2. [Faux Provider Pattern](#2-faux-provider-pattern)
3. [Table-Driven Tests](#3-table-driven-tests)
4. [Mock Streams](#4-mock-streams)
5. [Async Fixtures](#5-async-fixtures)
6. [Event Assertions](#6-event-assertions)
7. [Concurrent Tests](#7-concurrent-tests)
8. [Cancellation Tests](#8-cancellation-tests)
9. [Tool Execution Tests](#9-tool-execution-tests)
10. [State Management Tests](#10-state-management-tests)

---

## 1. Harness Pattern

Create test harness for common setup, state management, and assertions.

### Structure

```rust
// runie/crates/runie-tui/src/tui/tests/test_harness.rs
pub struct AgentTestHarness {
    pub state: AppState,
    pub messages: Vec<AgentEvent>,
    pub start_time: Instant,
}

impl AgentTestHarness {
    pub fn new() -> Self {
        let mut state = AppState::default();
        state.current_model = Some("test-model".to_string());
        Self { state, messages: Vec::new(), start_time: Instant::now() }
    }

    pub fn submit_user_message(&mut self, text: &str) -> Vec<Cmd> {
        self.state.textarea.insert_str(text);
        handle_submit(&mut self.state)
    }

    pub fn handle_agent_event(&mut self, event: AgentEvent) {
        handle_agent_event(&mut self.state, event)
    }

    pub fn assert_agent_running(&self) {
        assert!(self.state.agent_running, "agent should be running");
    }
}
```

### Builder Pattern for Scenarios

```rust
pub struct ScenarioBuilder {
    harness: AgentTestHarness,
}

impl ScenarioBuilder {
    pub fn new() -> Self { Self { harness: AgentTestHarness::new() } }

    pub fn with_model(mut self, model: &str) -> Self {
        self.harness.state.current_model = Some(model.to_string());
        self
    }

    pub fn user_says(mut self, text: &str) -> Self {
        self.harness.submit_user_message(text);
        self
    }

    pub fn agent_stream_text(mut self, text: &str) -> Self {
        let event = AgentEvent::MessageUpdate {
            message: AgentMessage {
                role: "assistant".to_string(),
                content: vec![ContentPart::Text { text: text.to_string() }],
                timestamp: 0, usage: None, stop_reason: None,
                error_message: None, tool_calls: vec![],
            },
            turn: 1,
            delta: text.to_string(),
        };
        self.harness.handle_agent_event(event);
        self
    }

    pub fn build(self) -> AgentTestHarness { self.harness }
}
```

### Usage

```rust
#[test]
fn test_stream_text_accumulation() {
    let mut harness = ScenarioBuilder::new()
        .with_model("test-model")
        .user_says("Hello")
        .build();

    harness.handle_agent_event(AgentEvent::MessageStart { ... });
    harness.handle_agent_event(AgentEvent::MessageUpdate { ... });
    harness.assert_last_assistant_text("expected text");
}
```

---

## 2. Faux Provider Pattern

Mock LLM provider for testing without real API calls.

### TypeScript Implementation (pi)

```typescript
// pi/packages/coding-agent/test/test-harness.ts
const FAUX_PROVIDER = "faux";
const FAUX_MODEL_ID = "faux-1";

export const fauxModel: Model<typeof FAUX_API> = {
    id: FAUX_MODEL_ID, name: "Faux Model", api: FAUX_API,
    provider: FAUX_PROVIDER, baseUrl: "http://localhost:0",
    reasoning: false, input: ["text", "image"], cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0 },
    contextWindow: 128000, maxTokens: 16384,
};

export interface FauxResponse {
    text?: string;
    toolCalls?: Array<{ id?: string; name: string; args: Record<string, unknown> }>;
    thinking?: string;
    stopReason?: StopReason;
    error?: string;
    usage?: Partial<Usage>;
    delayMs?: number;
    model?: { provider?: string; id?: string };
}
```

### Stream Function Factory

```typescript
export function createFauxStreamFn(responses: FauxResponseInput[]): {
    streamFn: (model: Model<any>, context: Context, options?: SimpleStreamOptions)
        => AssistantMessageEventStream;
    state: FauxStreamFnState;
} {
    const state: FauxStreamFnState = { callCount: 0, contexts: [] };

    const streamFn = (_model: Model<any>, context: Context, _options?: SimpleStreamOptions) => {
        const index = state.callCount % responses.length;
        state.callCount++;
        state.contexts.push(context);

        const message = buildAssistantMessage(normalizeResponse(responses[index]));
        const stream = createAssistantMessageEventStream();
        const emit = () => streamWithDeltas(stream, message);

        if (resp.delayMs && resp.delayMs > 0) {
            setTimeout(emit, resp.delayMs);
        } else {
            queueMicrotask(emit);
        }
        return stream;
    };

    return { streamFn, state };
}
```

### Rust Implementation (runie)

```rust
// runie/crates/runie-agent/src/tests/provider_tests.rs
pub struct AlwaysToolProvider;

#[async_trait]
impl Provider for AlwaysToolProvider {
    fn name(&self) -> &str { "always_tool" }
    fn model(&self) -> &str { "test" }
    fn supports_tools(&self) -> bool { true }
    fn supports_vision(&self) -> bool { false }
    fn max_context_tokens(&self) -> usize { 128_000 }

    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>)
        -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };
        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name, arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
        };
        Ok(Box::pin(s))
    }
}
```

### Session Harness with Full Wiring

```typescript
// pi/packages/coding-agent/test/test-harness.ts
export interface Harness {
    session: AgentSession;
    agent: Agent;
    sessionManager: SessionManager;
    settingsManager: SettingsManager;
    faux: FauxStreamFnState;
    events: AgentSessionEvent[];
    eventsOfType<T extends AgentSessionEvent["type"]>(type: T): Extract<AgentSessionEvent, { type: T }>[];
    tempDir: string;
    cleanup: () => void;
}

export function createHarness(options: HarnessOptions = {}): Harness {
    const tempDir = createTempDir();
    const { streamFn, state: fauxState } = createFauxStreamFn(options.responses ?? ["ok"]);

    const agent = new Agent({
        getApiKey: () => "faux-key",
        initialState: { model, systemPrompt: options.systemPrompt ?? "You are a test assistant.", tools: options.tools ?? [] },
        streamFn,
    });

    const session = new AgentSession({
        agent, sessionManager, settingsManager, cwd: tempDir,
        modelRegistry, resourceLoader, baseToolsOverride: options.baseToolsOverride,
    });

    const events: AgentSessionEvent[] = [];
    session.subscribe((event) => events.push(event));

    const cleanup = () => {
        session.dispose();
        if (existsSync(tempDir)) rmSync(tempDir, { recursive: true });
    };

    return { session, agent, sessionManager, settingsManager, faux: fauxState, events,
        eventsOfType<T>(type) { return events.filter((e): e is ... => e.type === type); },
        tempDir, cleanup };
}
```

---

## 3. Table-Driven Tests

Structure for state transition tests with minimal boilerplate.

### Go Implementation (opencode)

```go
// opencode/internal/llm/tools/ls_test.go
func TestShouldSkip(t *testing.T) {
    testCases := []struct {
        name           string
        path           string
        ignorePatterns []string
        expected       bool
    }{
        { name: "hidden file", path: "/path/to/.hidden_file", ignorePatterns: []string{}, expected: true },
        { name: "hidden directory", path: "/path/to/.hidden_dir", ignorePatterns: []string{}, expected: true },
        { name: "pycache directory", path: "/path/to/__pycache__/file.pyc", ignorePatterns: []string{}, expected: true },
        { name: "normal file", path: "/path/to/normal_file.txt", ignorePatterns: []string{}, expected: false },
        { name: "ignored by pattern", path: "/path/to/ignore_me.txt", ignorePatterns: []string{"ignore_*.txt"}, expected: true },
        { name: "not ignored by pattern", path: "/path/to/keep_me.txt", ignorePatterns: []string{"ignore_*.txt"}, expected: false },
    }

    for _, tc := range testCases {
        t.Run(tc.name, func(t *testing.T) {
            result := shouldSkip(tc.path, tc.ignorePatterns)
            assert.Equal(t, tc.expected, result)
        })
    }
}
```

### Rust Implementation

```rust
#[test]
fn test_sandbox_detection_cases() {
    struct TestCase {
        exit_code: i32,
        stderr: String,
        sandbox_type: SandboxType,
        expected: bool,
    }

    let cases = vec![
        TestCase { exit_code: 1, stderr: "Operation not permitted".into(),
            sandbox_type: SandboxType::LinuxSeccomp, expected: true },
        TestCase { exit_code: 127, stderr: "command not found".into(),
            sandbox_type: SandboxType::LinuxSeccomp, expected: false },
    ];

    for case in cases {
        let output = make_exec_output(case.exit_code, "", &case.stderr, "");
        assert_eq!(is_likely_sandbox_denied(case.sandbox_type, &output), case.expected);
    }
}
```

---

## 4. Mock Streams

Event-driven testing with token-level streaming simulation.

### Token Chunking

```typescript
// pi/packages/coding-agent/test/test-harness.ts
function chunkString(text: string): string[] {
    const chunks: string[] = [];
    let i = 0;
    while (i < text.length) {
        const size = 3 + Math.floor(Math.random() * 3); // 3, 4, or 5
        chunks.push(text.slice(i, i + size));
        i += size;
    }
    return chunks.length > 0 ? chunks : [""];
}

function streamWithDeltas(stream: AssistantMessageEventStream, message: AssistantMessage): void {
    const partial: AssistantMessage = { ...message, content: [] };
    stream.push({ type: "start", partial: { ...partial } });

    for (let i = 0; i < message.content.length; i++) {
        const block = message.content[i];
        if (block.type === "text") {
            partial.content = [...partial.content, { type: "text", text: "" }];
            stream.push({ type: "text_start", contentIndex: i, partial: { ...partial } });
            for (const chunk of chunkString(block.text)) {
                (partial.content[i] as TextContent).text += chunk;
                stream.push(makeEvent("text_delta", i, chunk, partial));
            }
            stream.push({ type: "text_end", contentIndex: i, content: block.text, partial: { ...partial } });
        }
        // ... similar for thinking, toolCall
    }
    stream.push({ type: "done", reason: message.stopReason as "stop" | "length" | "toolUse", message });
}
```

### Stream Event Collection

```typescript
async function collectEvents(streamResult: ReturnType<typeof stream>): Promise<AssistantMessageEvent[]> {
    const events: AssistantMessageEvent[] = [];
    for await (const event of streamResult) {
        events.push(event);
    }
    return events;
}

// Usage
const events = await collectEvents(stream(model, context));
expect(events.map(e => e.type)).toEqual([
    "start", "thinking_start", "thinking_delta", "thinking_end",
    "text_start", "text_delta", "text_end",
    "toolcall_start", "toolcall_delta", "toolcall_end", "done"
]);
```

### Rust Stream Testing

```rust
// runie/crates/runie-tui/src/tui/tests/streaming_tests.rs
#[tokio::test]
async fn test_stream_text_accumulation() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    harness.handle_agent_event(AgentEvent::MessageStart {
        message: agent_message("assistant", ""), turn: 1,
    });

    // Stream text in chunks
    let full_texts = vec!["Hi", "Hi there", "Hi there!"];
    let deltas = vec!["Hi", " there", "!"];
    for (full, delta) in full_texts.iter().zip(deltas.iter()) {
        harness.handle_agent_event(AgentEvent::MessageUpdate {
            message: agent_message("assistant", full),
            turn: 1,
            delta: delta.to_string(),
        });
    }
    harness.assert_last_assistant_text("Hi there!");
}
```

---

## 5. Async Fixtures

Temp directory management with automatic cleanup.

### TypeScript

```typescript
// pi/packages/coding-agent/test/test-harness.ts
function createTempDir(): string {
    const tempDir = join(tmpdir(), `pi-harness-${Date.now()}-${Math.random().toString(36).slice(2)}`);
    mkdirSync(tempDir, { recursive: true });
    return tempDir;
}

// Usage in test
let harness: Harness;
afterEach(() => { harness?.cleanup(); });

it("simple text response", async () => {
    harness = createHarness({ responses: ["hello world"] });
    await harness.session.prompt("hi");
    expect(harness.faux.callCount).toBe(1);
});
```

### Go

```go
// opencode/internal/llm/tools/ls_test.go
func TestLsTool_Run(t *testing.T) {
    tempDir, err := os.MkdirTemp("", "ls_tool_test")
    require.NoError(t, err)
    defer os.RemoveAll(tempDir)

    // Create test directory structure
    testDirs := []string{"dir1", "dir2", "dir2/subdir1"};
    for _, dir := range testDirs {
        dirPath := filepath.Join(tempDir, dir)
        err := os.MkdirAll(dirPath, 0755)
        require.NoError(t, err)
    }

    // ... tests
}
```

### Rust

```rust
// codex/codex-rs/core/src/exec_tests.rs
#[test]
fn windows_elevated_supports_unreadable_split_carveouts() {
    let temp_dir = tempfile::TempDir::new().expect("tempdir");
    let blocked = temp_dir.path().join("blocked");
    std::fs::create_dir_all(&blocked).expect("create blocked");
    // ... tests
} // temp_dir automatically cleaned up when dropped
```

---

## 6. Event Assertions

Verify event sequences and state changes.

### Event Collection and Filtering

```typescript
// pi/packages/coding-agent/test/test-harness.test.ts
it("event capture", async () => {
    harness = createHarness({ responses: ["hello"] });
    await harness.session.prompt("hi");

    const agentStarts = harness.eventsOfType("agent_start");
    expect(agentStarts).toHaveLength(1);

    const agentEnds = harness.eventsOfType("agent_end");
    expect(agentEnds).toHaveLength(1);

    const messageEnds = harness.eventsOfType("message_end");
    expect(messageEnds.length).toBeGreaterThanOrEqual(2);
});
```

### Rust Event Stream Collection

```rust
// runie/crates/runie-agent/tests/integration_test.rs
async fn collect_agent_events(stream: &mut impl AgentLoop) -> (bool, bool, bool, bool) {
    let mut got_message_start = false;
    let mut got_message_update = false;
    let mut got_message_end = false;
    let mut got_agent_end = false;

    while let Some(event) = stream.next().await {
        match event {
            AgentEvent::MessageStart { .. } => got_message_start = true,
            AgentEvent::MessageUpdate { .. } => got_message_update = true,
            AgentEvent::MessageEnd { .. } => got_message_end = true,
            AgentEvent::AgentEnd { .. } => { got_agent_end = true; break; }
            AgentEvent::PermissionRequest { tool_call_id, tool_name, tool_args, .. } => {
                let _ = stream.send_permission(
                    PermissionDecision::Allow { tool_call_id, tool_name, tool_args }
                ).await;
            }
            _ => {}
        }
    }
    (got_message_start, got_message_update, got_message_end, got_agent_end)
}
```

### Event Order Verification

```typescript
// pi/packages/ai/test/faux-provider.test.ts
it("streams an exact event order for fixed-size chunks", async () => {
    registration.setResponses([fauxAssistantMessage(
        [fauxThinking("go"), fauxText("ok"), fauxToolCall("echo", {}, { id: "tool-1" })],
        { stopReason: "toolUse" }
    )]);

    const events = await collectEvents(stream(model, context));
    expect(events.map((event) => event.type)).toEqual([
        "start", "thinking_start", "thinking_delta", "thinking_end",
        "text_start", "text_delta", "text_end",
        "toolcall_start", "toolcall_delta", "toolcall_end", "done"
    ]);
});
```

---

## 7. Concurrent Tests

Race condition testing with controlled parallelism.

### Tokio Spawn and Timeout

```rust
// codex/codex-rs/core/src/exec_tests.rs
#[tokio::test]
async fn kill_child_process_group_kills_grandchildren_on_timeout() -> Result<()> {
    let command = vec!["/bin/bash".to_string(), "-c".to_string(),
        "sleep 60 & echo $!; sleep 60".to_string()];
    let cwd = codex_utils_absolute_path::AbsolutePathBuf::current_dir()?;
    let params = ExecParams {
        command, cwd, expiration: 500.into(),
        capture_policy: ExecCapturePolicy::ShellTool,
        env: std::env::vars().collect(),
        // ...
    };

    let output = exec(params, NetworkSandboxPolicy::Restricted, None, None).await?;
    assert!(output.timed_out);

    // Verify grandchild was killed
    let mut killed = false;
    for _ in 0..20 {
        if unsafe { libc::kill(pid, 0) } == -1
            && let Some(libc::ESRCH) = std::io::Error::last_os_error().raw_os_error()
        {
            killed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(killed, "grandchild process with pid {pid} is still alive");
    Ok(())
}
```

### Semaphore-Controlled Concurrency

```rust
// codex/codex-rs/core/src/session/tests.rs
#[tokio::test]
async fn concurrent_turn_processing() {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_TURNS));
    // Launch multiple turns and verify they serialize correctly
}
```

---

## 8. Cancellation Tests

Abort and timeout testing with proper cleanup.

### TypeScript Abort Testing

```typescript
// pi/packages/ai/test/abort.test.ts
async function testAbortSignal<TApi extends Api>(llm: Model<TApi>, options: StreamOptionsWithExtras = {}) {
    const context: Context = {
        messages: [{ role: "user", content: "What is 15 + 27?", timestamp: Date.now() }],
        systemPrompt: "You are a helpful assistant.",
    };

    let abortFired = false;
    let text = "";
    const controller = new AbortController();
    const response = await stream(llm, context, { ...options, signal: controller.signal });

    for await (const event of response) {
        if (abortFired) return;
        if (event.type === "text_delta" || event.type === "thinking_delta") {
            text += event.delta;
        }
        if (text.length >= 50) {
            controller.abort();
            abortFired = true;
        }
    }

    const msg = await response.result();
    expect(msg.stopReason).toBe("aborted");
}

async function testImmediateAbort<TApi extends Api>(llm: Model<TApi>, options: StreamOptionsWithExtras = {}) {
    const controller = new AbortController();
    controller.abort();

    const context: Context = {
        messages: [{ role: "user", content: "Hello", timestamp: Date.now() }],
    };

    const response = await complete(llm, context, { ...options, signal: controller.signal });
    expect(response.stopReason).toBe("aborted");
}
```

### Rust Cancellation Testing

```rust
// codex/codex-rs/core/src/exec_tests.rs
#[tokio::test]
async fn process_exec_tool_call_respects_cancellation_token() -> Result<()> {
    let cancel_token = CancellationToken::new();
    let cancel_tx = cancel_token.clone();
    let params = ExecParams {
        command: long_running_command(),
        cwd: cwd.clone(),
        expiration: ExecExpiration::Cancellation(cancel_token),
        capture_policy: ExecCapturePolicy::ShellTool,
        env,
        // ...
    };

    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(1_000)).await;
        cancel_tx.cancel();
    });

    let result = timeout(Duration::from_secs(5), process_exec_tool_call(params, ...)).await;
    let output = result.expect("cancellation should return a non-timeout exec result");
    assert!(!output.timed_out);
    assert_ne!(output.exit_code, 0);
    Ok(())
}
```

### Retry with Abort

```typescript
// pi/packages/ai/test/abort.test.ts
it("supports aborting mid-stream when paced", async () => {
    const registration = registerFauxProvider({ tokensPerSecond: 100, tokenSize: { min: 3, max: 3 } });
    registration.setResponses([fauxAssistantMessage("abcdefghijklmnopqrstuvwxyz")]);

    const controller = new AbortController();
    const events: string[] = [];
    let textDeltaCount = 0;

    const s = stream(model, { messages: [...] }, { signal: controller.signal });
    for await (const event of s) {
        events.push(event.type);
        if (event.type === "text_delta") {
            textDeltaCount++;
            controller.abort();
        }
    }

    expect(textDeltaCount).toBe(1);
    expect(events).toContain("text_start");
    expect(events).toContain("text_delta");
    expect(events).toContain("error");
    expect(events).not.toContain("text_end");
});
```

---

## 9. Tool Execution Tests

Tool mocking and execution verification.

### Tool Mock with Execute Handler

```typescript
// pi/packages/coding-agent/test/test-harness.test.ts
it("tool call response triggers tool execution", async () => {
    let toolExecuted = false;
    const echoTool: AgentTool = {
        name: "echo",
        label: "Echo",
        description: "Echo back",
        parameters: Type.Object({ text: Type.String() }),
        execute: async () => {
            toolExecuted = true;
            return { content: [{ type: "text", text: "echoed" }], details: {} };
        },
    };

    harness = createHarness({
        responses: [{ toolCalls: [{ name: "echo", args: { text: "hi" } }] }, "done after tool"],
        tools: [echoTool],
        baseToolsOverride: { echo: echoTool },
    });

    await harness.session.prompt("use the tool");
    expect(toolExecuted).toBe(true);
    expect(harness.faux.callCount).toBe(2);
});
```

### Rust Tool Provider

```rust
// runie/crates/runie-agent/src/tests/provider_tests.rs
pub struct SimpleToolProvider;

#[async_trait]
impl Provider for SimpleToolProvider {
    async fn chat(&self, _messages: Vec<Message>, tools: Vec<ToolSchema>)
        -> Result<BoxStream<'static, LlmEvent>, ProviderError> {
        let tool_name = if tools.is_empty() { "bash".to_string() } else { tools[0].name.clone() };
        let s = stream! {
            yield LlmEvent::MessageStart { role: "assistant".to_string(), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolCallDelta { id: "call_0".to_string(), name: tool_name.clone(), arguments: "{}".to_string() };
            yield LlmEvent::MessageEnd;
            yield LlmEvent::ToolExecutionStart { tool_call_id: "call_1".to_string(), tool_name, args: serde_json::json!({}), timestamp: chrono::Utc::now() };
            yield LlmEvent::ToolExecutionEnd { tool_call_id: "call_1".to_string(),
                result: ToolOutput { content: "executed".to_string(), metadata: serde_json::json!({}), terminate: false },
                timestamp: chrono::Utc::now() };
        };
        Ok(Box::pin(s))
    }
}
```

### Go Tool Testing

```go
// opencode/internal/llm/tools/ls_test.go
func TestLsTool_Run(t *testing.T) {
    tool := NewLsTool()
    params := LSParams{Path: tempDir}
    paramsJSON, err := json.Marshal(params)
    require.NoError(t, err)

    call := ToolCall{Name: LSToolName, Input: string(paramsJSON)}
    response, err := tool.Run(context.Background(), call)
    require.NoError(t, err)

    assert.Contains(t, response.Content, "dir1")
    assert.Contains(t, response.Content, "dir2")
    assert.Contains(t, response.Content, "file1.txt")
    assert.NotContains(t, response.Content, ".hidden_dir")
    assert.NotContains(t, response.Content, "__pycache__")
}
```

---

## 10. State Management Tests

State transitions and persistence verification.

### Session Persistence

```typescript
// pi/packages/coding-agent/test/test-harness.test.ts
it("session persistence works", async () => {
    harness = createHarness({ responses: ["persisted"] });
    await harness.session.prompt("hi");

    const entries = harness.sessionManager.getEntries();
    const messageEntries = entries.filter((e) => e.type === "message");
    expect(messageEntries.length).toBeGreaterThanOrEqual(2);
});
```

### Token Usage Accumulation

```rust
// runie/crates/runie-tui/src/tui/tests/streaming_tests.rs
#[test]
fn test_rapid_token_usage_accumulation() {
    let mut harness = AgentTestHarness::new();
    harness.submit_user_message("Hello");

    for _ in 0..50 {
        harness.handle_agent_event(token_usage(1, 1));
    }

    assert_eq!(harness.state.session_token_usage.total_tokens, 100, "50 events × (1 + 1) = 100 tokens");
    assert_eq!(harness.state.session_token_usage.prompt_tokens, 50, "50 prompt tokens");
    assert_eq!(harness.state.session_token_usage.completion_tokens, 50, "50 completion tokens");
}
```

### Context Capture

```typescript
// pi/packages/coding-agent/test/test-harness.test.ts
it("context capture", async () => {
    harness = createHarness({ responses: ["reply"] });
    await harness.session.prompt("my question");

    expect(harness.faux.contexts).toHaveLength(1);
    const ctx = harness.faux.contexts[0];
    const userMsg = ctx.messages.find((m) => m.role === "user");
    expect(userMsg).toBeDefined();
});
```

### State Transition with Events

```rust
// runie/crates/runie-agent/tests/integration_test.rs
#[tokio::test]
async fn test_agent_end_to_end() {
    let (provider, registry, config, messages) = setup_agent_test();
    let mut stream = agent_loop(messages, config, provider, vec![], registry, vec![]);

    let (got_message_start, got_message_update, got_message_end, got_agent_end) =
        collect_agent_events(&mut stream).await;

    let _final_messages = stream.result().await;

    assert!(got_message_start, "Should receive MessageStart");
    assert!(got_message_update, "Should receive MessageUpdate");
    assert!(got_message_end, "Should receive MessageEnd");
    assert!(got_agent_end, "Should receive AgentEnd");
}
```

---

## Implementation Plan

### Step 1: Create Test Harness Module

```rust
// crates/runie-tui/src/tui/tests/test_harness.rs
// - AgentTestHarness struct with state, messages, timing
// - ScenarioBuilder for fluent test setup
// - Assertion helpers for common checks
```

### Step 2: Implement Mock Provider

```rust
// crates/runie-agent/src/tests/mock_provider.rs
// - MockProvider struct with response queue
// - Stream generation with token chunking
// - Error injection support
// - Token counting and usage tracking
```

### Step 3: Add Table-Driven Test Utilities

```rust
// crates/runie-core/src/testing/table_driven.rs
// - TestCase trait for table-driven tests
// - TestCaseSuite for running multiple cases
// - Generic assertion helpers
```

### Step 4: Create Async Test Fixtures

```rust
// crates/runie-core/src/testing/fixtures.rs
// - TempDir fixture with automatic cleanup
// - ResourceTracker for managing test resources
// - Setup/teardown hooks
```

### Step 5: Event Assertion Library

```rust
// crates/runie-core/src/testing/events.rs
// - EventCollector for capturing event sequences
// - EventMatcher for pattern matching
// - OrderedEventChecker for sequence verification
```

### Step 6: Concurrent Test Helpers

```rust
// crates/runie-core/src/testing/concurrent.rs
// - SpawnWithTimeout helper
// - Semaphore-controlled parallelism
// - Race condition detectors
```

### Step 7: Cancellation Test Utilities

```rust
// crates/runie-core/src/testing/cancellation.rs
// - AbortController wrapper
// - Timeout helpers
// - Cancellation token factories
```

### Step 8: Tool Mock Framework

```rust
// crates/runie-tools/src/testing/tool_mock.rs
// - MockTool struct with execute handler
// - ToolCallRecorder for verification
// - FakeToolExecutor for integration tests
```

### Step 9: State Transition Testing

```rust
// crates/runie-core/src/testing/state_machine.rs
// - StateMachineTester for transition verification
// - TransitionRecorder for capturing state changes
// - Snapshot comparison utilities
```

### Step 10: Integration Tests

```rust
// crates/runie-agent/tests/integration_test.rs
// - Full harness integration tests
// - End-to-end agent loop tests
// - Multi-turn conversation tests
```

---

## Quick Reference

| Pattern | Key Types | When to Use |
|---------|-----------|-------------|
| Harness | `AgentTestHarness`, `ScenarioBuilder` | UI state, event handling |
| Faux Provider | `MockProvider`, `FauxResponse` | LLM integration, streaming |
| Table-Driven | `TestCase`, `struct` with cases | State transitions, edge cases |
| Mock Streams | `EventStream`, `chunkString()` | Token-level streaming |
| Async Fixtures | `TempDir`, `afterEach` cleanup | File I/O, resource management |
| Event Assertions | `eventsOfType()`, `collectEvents()` | Sequence verification |
| Concurrent | `tokio::spawn`, `Semaphore` | Race conditions, parallelism |
| Cancellation | `AbortController`, `CancellationToken` | Timeout, abort behavior |
| Tool Execution | `MockTool`, `execute()` | Tool mocking, verification |
| State Management | `sessionManager`, `state` | Persistence, transitions |
