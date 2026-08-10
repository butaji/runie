# Command palette

## Grok reference

Runie's reference implementation is the local Grok pager at
`/Users/admin/Code/agents/grok-build`, especially `app/modals.rs`,
`views/modal_window.rs`, and `views/picker.rs`. The shared modal uses 50% width,
80-column maximum, 44-column minimum, and a four-row vertical margin. The
focused search bar is `search: ` with the query appended; Grok's unfocused
vim-nav placeholder is `/ to search`. Footer vocabulary is `↑/↓ nav`,
`Enter select`, and `Esc close`.

## States

Closed, open, query-filtered, selected command, activated command, and
dismissed.

## Contract

Input events go to the UI actor. The palette is a pure view of its snapshot;
activation publishes a command event and never mutates core state directly.
The palette registry includes every Runie-supported built-in command and
discovered skill. Commands requiring arguments push the shared parameter form;
submitting it emits the corresponding core command event.

## Acceptance

`visual-command-palette.yaml`, `visual-command-palette-activate.yaml`, nested
parameter-form replay tests, and a live 120×36 tmux capture.
