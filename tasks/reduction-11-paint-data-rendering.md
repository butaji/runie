# Reduction 11: paint-data rendering

Status: partial

Introduce a small declarative paint/layout intermediate representation for
widgets where it materially reduces repeated terminal rendering code.

Acceptance: pure view-to-paint tests and unchanged cell-level snapshots.

Progress: `PaintDocument` and `PaintText` now provide a renderer-neutral,
testable paint representation, `status_paint` projects a real actor snapshot
into it, `prompt_paint` covers prompt mode/caption projection, and
`render_status_paint` and `render_prompt_paint` provide Ratatui adapters.
Both adapters now share one generic `render_paint_document` boundary, keeping
the terminal interpretation centralized. Existing full widgets still need
incremental migration where their borders and interactive rows fit the IR.
Semantic `ToolCardRow` values now also project through `tool_card_paint`,
providing a renderer-neutral migration seam for scrollback cards.
The semantic-to-renderer intent conversion is centralized in one
`From<ToolCardPaintIntent>` implementation, so card projection no longer
duplicates the mapping table.
The shared renderer now resolves each semantic intent through the selected
theme, so paint data controls terminal styling instead of being discarded.
The paint vocabulary and documents are now serde-backed, with YAML round-trip
coverage so renderer-neutral projections can participate in replay fixtures.
