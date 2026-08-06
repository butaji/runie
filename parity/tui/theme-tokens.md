# Theme tokens

Themes are semantic tokens, not component-local color literals. Grok palettes
are mapped through Opaline and resolved by `ThemeKind`.

Required token families: background, base text, muted text, accent, success,
error, warning, border, prompt, reasoning, tool, and selection.

Acceptance is cell-level fg/bg/style comparison under GrokNight and GrokDay.
