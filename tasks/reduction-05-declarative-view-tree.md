# Reduction 05: declarative view tree

Status: adopted

Extend the existing component specifications in `runie-tui::view` into a
data-driven layout/overlay scene graph.

The component ownership table now uses `component_specs!`, keeping component,
slot, and actor ownership as declarative data while leaving layout and render
semantics explicit.

Acceptance: pure layout tests and unchanged visual fixtures at documented
terminal sizes.
