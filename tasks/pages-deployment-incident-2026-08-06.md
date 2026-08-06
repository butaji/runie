# Pages deployment incident — 2026-08-06

## Finding

The failed `pages build and deployment` run `31116944792` for commit
`5c0bcb8278eca3a9426dc11a04484c77ece4fc19` failed before the build steps
started. GitHub Actions logged repeated:

```text
Failed to resolve action download info. Error: Service Unavailable
Failed to resolve action download info.
```

This is an Actions service-side dependency-resolution failure, not a broken
HTML, asset, Jekyll, or Rust build. The subsequent Pages run for the restored
static site was accepted and a new run was queued.

## Repository contract

- `docs/index.html` and the restored assets are the static Pages source.
- `docs/.nojekyll` disables Jekyll processing for that source.
- No `.github/workflows` is present or should be added; Pages is managed by
  the repository Pages configuration.

## Verification

Use the Pages run URL or `gh run view` to confirm the queued run reaches a
terminal state. If it fails with the same action-resolution message, retry
after GitHub Actions recovers. Do not modify Runie code or add a workflow for
this incident.
