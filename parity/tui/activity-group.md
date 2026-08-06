# Activity group

## Purpose

Fold consecutive tools into one Grok vocabulary row: `Listed N dir`,
`Read N files`, `Ran N commands`, and mixed combinations.

## States

- open/running with live labels
- completed
- failed with `· N failed`
- collapsed members
- expanded members

## Events

Tool start/update/end events reduce into the scrollback actor. Parallel tool
completion order must not merge unrelated batches.

## Acceptance

`visual-activity-{mixed,collapsed,subagent}.yaml` and `visual-tool-error.yaml`.
