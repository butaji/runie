# Reduction 09: declarative schema generation

Status: partial

Identify repetitive registries, dispatch tables, and metadata suitable for
typed declarations or narrowly scoped macros.

Acceptance: generated output is type checked, inspectable, and does not hide
async lifecycle or domain decisions.

Progress: `component_specs!` and the existing view/layout DSLs now generate
repetitive immutable view metadata. Domain reducers and async lifecycle remain
ordinary Rust.

The paint layer also uses `paint!` for repetitive immutable paint rows.
Static provider and theme registries now use typed constant tables instead of
large function bodies; `declare_reducer_actor!` generates the remaining
typed actor handle boilerplate.
Palette slash-command and description metadata now use declarative Rust
macros, keeping the `PaletteAction` methods as thin typed accessors.
Model-declared effort levels now use one macro-backed data table for picker
options, selection validation, and model capability checks.
Tool-card paint intent conversion is now one typed mapping shared by the
renderer and pure paint projection, removing a duplicate dispatch table.
Feed navigation-to-snapshot projection now uses one typed field declaration
macro, including explicit clone fields and the derived selected-member field,
so the projection schema is data-shaped instead of repeated assignments.
Activity-tool aliases now use a typed grouped alias macro, keeping the
classifier vocabulary declarative and exhaustively mapped to `ActivityKind`.
Builtin theme names now use one typed macro table that generates both the
loader dispatch and its test inventory, eliminating the split hand-written
match while keeping every theme mapping inspectable as data.
