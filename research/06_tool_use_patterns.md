# Tool Use Patterns in Coding Agents

## Source
- Article: "Components of A Coding Agent" by Sebastian Raschka, PhD
- URL: https://magazine.sebastianraschka.com/p/components-of-a-coding-agent
- Date: April 4, 2026

---

## 1. Tool Use Flow (Core Loop)

```
Model emits structured action → Harness validates → Optional approval → Execute → Feed bounded result back to loop
```

**Key insight**: Model outputs structured actions (not prose commands), harness intercepts and can stop/reject before execution.

---

## 2. Common Tool Categories

| Category | Examples |
|----------|----------|
| **File System** | list files, read file, write file |
| **Shell/Terminal** | execute shell commands, run test suites |
| **Search** | grep, find symbols, find definitions |
| **Code Editing** | apply diff, edit file |
| **Delegation** | spawn subagent |

Tool descriptions are **pre-defined** with clear inputs/outputs—not improvised syntax.

---

## 3. Tool Schema Design

- **Named tools** with explicit input/output schemas
- Model provides arguments in a shape harness can validate programmatically
- Harness defines what tools exist, not model

Example tools in Mini Coding Agent:
- `build_tools` - defines available tools
- `run_tool` - executes validated tool
- `validate_tool` - checks tool validity
- `approve` - user approval gate
- `parse` - parse model output

---

## 4. Tool Selection Mechanism

**Not** model improvisation. Selection is constrained:

1. Model chooses from **pre-defined tool list**
2. Arguments must match expected schema
3. Harness performs **programmatic checks**:
   - "Is this a known tool?"
   - "Are the arguments valid?"
   - "Does this need user approval?"
   - "Is requested path inside the workspace?"

Only after checks pass → execution proceeds.

---

## 5. Safety Mechanisms

### Approval Gating
- Certain dangerous operations require explicit user approval before execution
- Model asks to do something → runtime stops and prompts user

### Path Sandboxing
- File access constrained to repo/workspace boundaries
- Prevents model from accessing files outside working directory

### Validation Layers
```
Tool exists? → Arguments valid? → Needs approval? → Path in workspace? → Execute
```

### Bounded Execution
- Large outputs clipped before returning to model
- Results fed back are bounded, not raw unlimited output

---

## 6. Error Handling

Article does **not** explicitly detail error handling, but implied:

- Invalid tools/arguments rejected before execution
- Malformed actions blocked
- Clip mechanism prevents runaway outputs from breaking the loop

---

## 7. Subagent Delegation

Parallel work via bounded subagents:

| Aspect | Main Agent | Subagent |
|--------|-----------|----------|
| Context | Full workspace | Inherited but limited |
| Permissions | Read/write | Often read-only |
| Recursion depth | Unbounded | Restricted |
| Task scope | Full | Bounded subtask |

Subagent pattern allows parallelization without unbounded proliferation.

---

## 8. Context Management for Tools

### Stable Prompt Prefix (cached, rarely changes)
- General instructions
- Tool descriptions
- Workspace summary

### Dynamic Components (updated each turn)
- Short-term memory
- Recent transcript
- Latest user request

### Tool Output Handling
- **Clipping**: Long outputs truncated
- **Deduplication**: Repeated file reads collapsed
- **Compression**: Older events compressed more aggressively

---

## 9. Key Architectural Principles

1. **Structured over unstructured**: Model outputs actions, not prose
2. **Harness validates everything**: Model cannot self-execute
3. **Bounded resources**: Clip, deduplicate, compress
4. **Separation of concerns**: Agent loop vs runtime scaffold
5. **User in the loop**: Approval gating for sensitive operations
6. **Workspace isolation**: Path sandboxing

---

## 10. Mini Coding Agent Code Structure (Reference)

```python
# Six Agent Components (from source):
# 1) Live Repo Context -> WorkspaceContext
# 2) Prompt Shape And Cache Reuse -> build_prefix, memory_text, prompt
# 3) Structured Tools, Validation, And Permissions -> build_tools, run_tool, validate_tool, approve, parse, path, tool_*
# 4) Context Reduction And Output Management -> clip, history_text
# 5) Transcripts, Memory, And Resumption -> SessionStore, record, note_tool, ask, reset
# 6) Delegation And Bounded Subagents -> tool_delegate
```

Repo: https://github.com/rasbt/mini-coding-agent
