# Reduction 06: event projection registry

Status: adopted

`runie-tui-model::events` now centralizes ownership classification and the
primary feed projection dispatch. Remaining UI/status adapters should migrate
to the same registry rather than adding new parallel matches.

`EventProjection` now packages scope, feed, status, and UI projections for a
single event. Status and UI actor delivery now use this shared projection
value; the feed actor retains its stateful tool-context enrichment boundary.

Acceptance: every currently projected event retains its output and unknown
events remain safely ignored.
