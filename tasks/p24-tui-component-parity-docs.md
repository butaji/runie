# p24 — Source-backed TUI component parity docs

## Objective

Maintain one discoverable parity contract per Grok TUI component so source
research, implementation, and YAML replay verification stay aligned.

## Deliverable

`parity/tui/` contains a component page and canonical `index.json` manifest for
layout, header, scrollback, prompt, thinking, assistant, activity, tools,
status, themes, animation, command palette, and welcome.

## Rules

- Every page names its authoritative Grok source paths.
- Every state/variant names an event or actor-owned projection.
- Every visual contract names a YAML fixture or cell oracle.
- Changes to a component update its page and its fixture together.

## Status

In progress. Documentation scaffolding is complete; per-block measurement,
vertical-padding metadata, sticky headers, and strict cast parity remain open.
