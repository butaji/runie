# Command palette

## States

Closed, open, query-filtered, selected command, activated command, and
dismissed.

## Contract

Input events go to the UI actor. The palette is a pure view of its snapshot;
activation publishes a command event and never mutates core state directly.

## Acceptance

`visual-command-palette.yaml` and `visual-command-palette-activate.yaml`.
