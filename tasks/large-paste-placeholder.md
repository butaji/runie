# Large-paste placeholder

## Objective

Collapse large pastes into a `[Pasted Text: N lines]` placeholder to prevent accidental wall-of-text submits. Expand on second paste or send.

## Agent landscape finding

gemini-cli collapses big pastes into placeholders. This prevents accidental multi-line submits from clipboard history.

## runie current state

Runie pastes text directly into the input box, including bracketed paste.

## Required runie changes

- Detect paste events larger than a threshold (e.g., >10 lines or >500 chars).
- Replace the pasted content with a placeholder like `[Pasted Text: 12 lines]`.
- Pressing paste again on the placeholder expands it to the full text.
- Sending the message expands the placeholder automatically.

## Test scenarios

1. **Large paste becomes placeholder**
   - Keys: paste a 15-line block.
   - Assert: input shows `[Pasted Text: 15 lines]`.

2. **Second paste expands**
   - Keys: paste again while placeholder is selected.
   - Assert: full pasted text appears in input.

3. **Send expands placeholder**
   - Keys: paste large block, press `Enter`.
   - Assert: message sent with full content.

## Edge / negative cases

- Small pastes (< threshold) are not collapsed.
- Placeholder is editable/deletable like normal text.
- Placeholder expansion does not trigger a second submit.

## Dependencies

- `input_composition`

## Acceptance checklist

- [ ] All scenarios pass with `AppTest::mock()`.
- [ ] Edge cases are covered.
- [ ] No `sleep()` in resulting Rust tests.
