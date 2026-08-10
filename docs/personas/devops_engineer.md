# Persona: The DevOps Engineer

**Primary Use Case:** Infrastructure automation, CI/CD pipeline development, deployment scripting, system observability, and incident response.

---

## 1. Persona Profile

### Background

Marcus Chen, 34, Senior DevOps Engineer at a mid-size fintech company. 10+ years in infrastructure and platform engineering. He's spent years building and maintaining Kubernetes clusters, Terraform modules, GitHub Actions workflows, and Ansible playbooks. When AI coding tools became mainstream, he was an early adopter — but remains skeptical of anything that feels "magical" or opaque.

**Current stack:**
- Kubernetes (EKS), Terraform, Ansible
- GitHub Actions, ArgoCD
- Prometheus + Grafana for observability
- Slack for alerts,PagerDuty for incidents
- tmux + neovim for daily work

### Expertise Level

**Advanced power user.** Marcus knows his tools deeply and expects the same from new ones. He's not afraid of configuration files, regex, or reading raw error logs. He uses tmux daily, has customized his neovim config extensively, and writes bash scripts for nearly everything.

### Work Style

- **Keyboard-first, always.** The mouse is a last resort.
- Prefers **composable tools** over all-in-one solutions
- Values **transparency** over convenience
- Documents everything; expects tools to be documentable too
- Treats scripts as artifacts — version-controlled and peer-reviewed
- On-call rotation means **incident response at 3 AM** — reliability is non-negotiable

---

## 2. Goals and Motivations

### Primary Goals

1. **Automate repetitive infrastructure tasks** — If he has to do it twice, he scripts it
2. **Reduce mean time to resolution (MTTR)** — Fast incident diagnosis saves sleep
3. **Maintain reliable, auditable systems** — Changes must be reproducible and reversible
4. **Enable self-service for developers** — Platform engineering mindset
5. **Continuously improve pipelines** — Faster deploys, fewer failures

### What Motivates Marcus

- **Control and predictability** — He wants to understand exactly what a tool will do before it does it
- **Composability** — Small tools that work together beat monolithic platforms
- **Reliability** — Tools that fail silently or unpredictably are worse than no tool
- **Efficiency** — If something takes 10 clicks, he wants to do it in 1 command
- **Learning** — He enjoys understanding how things work under the hood

### Secondary Goals

- Reduce alert fatigue through better observability tooling
- Improve CI/CD pipeline performance (faster feedback loops)
- Standardize infrastructure patterns across teams
- Mentor junior engineers on best practices

---

## 3. Pain Points with Current Tools

### The "Magic Box" Problem

Marcus despises tools that behave like black boxes. When something goes wrong, he needs to understand *why*. Current AI tools often:

- Make changes without showing the diff first
- Fail silently with opaque error messages
- Suggest plausible-but-wrong configurations that cause production incidents
- Don't explain their reasoning or assumptions

> "If I can't understand why the tool suggested something, I can't trust it. And if I can't trust it, I'll just do it myself."

### Verification Burden

According to the Coding Agents UX research, 66% of developers cite "almost right" solutions as their #1 frustration. Marcus experiences this constantly:

- AI suggests a Terraform resource that looks correct but uses deprecated attributes
- Pipeline YAML looks valid but has subtle logic errors
- Shell commands work in simple cases but fail edge cases

He ends up spending more time verifying AI output than if he'd written it from scratch.

### Poor CI/CD Integration

Most AI tools assume a "start from scratch" or "modify existing files" workflow. For CI/CD:

- Tools don't understand the context of running in a GitHub Actions runner
- Secrets management is an afterthought
- No understanding of idempotency requirements
- Can't read pipeline logs to diagnose failures

### Silent Failures

When tools fail, Marcus needs:

- Clear exit codes (not "something went wrong")
- Actionable error messages
- stderr output for scripting/debugging

Current AI tools often exit 0 even when they clearly failed, or provide unhelpful "An error occurred" messages.

### Lack of Composability

Tools that don't play well with pipes and scripts:

- Output in colors/formatting that breaks parsing
- Require interactive terminals
- Don't support headless/non-interactive modes
- Lock users into GUI-only workflows

---

## 4. What Would Delight This User

### Predictable, Transparent Behavior

Marcus wants tools that:

- **Show before doing** — Display the exact changes before executing
- **Explain their reasoning** — "I suggested this because..."
- **Fail loudly with context** — "Error at line 42: undefined variable 'db_host'"
- **Respect Unix conventions** — stdout for data, stderr for diagnostics, proper exit codes

### Headless and Scriptable Modes

Every operation should be available via CLI for scripting:

```bash
# Marcus wants to pipe AI output into other tools
runie "optimize this pipeline" | jq '.suggestions[] | .file'

# Or integrate into existing workflows
cat deployment.yaml | runie --validate --schema k8s
```

### Reliable Fixture/Testing Support

For CI/CD contexts, he wants tools that:

- Can run against recorded fixtures (no real API calls)
- Support deterministic output for testing
- Don't make network calls unless explicitly requested

### Deep IDE/Terminal Integration

Tools that feel native to his workflow:

- Inline diffs before accepting changes (like Cursor)
- Keyboard shortcuts that don't fight vim/mux muscle memory
- Can be embedded in tmux panes for monitoring

### Trust-Building Features

- **Dry-run modes** — Show what would happen without making changes
- **Audit trails** — Log all actions with timestamps and context
- **Rollback support** — Easy reversal when things go wrong
- **Version-controllable state** — No hidden state files that bypass git

---

## 5. Specific UI/UX Recommendations for Runie

### 5.1 Keyboard Navigation (TUI Best Practices)

Following vim/tmux conventions for familiarity:

| Key | Action |
|-----|--------|
| `j/k` | Navigate list items (up/down) |
| `h/l` | Collapse/expand panels or go back/forward |
| `/` | Search within current view |
| `:` | Open command palette |
| `Ctrl+P` | Quick open (files, commands) |
| `Esc` | Cancel/back out — always works |
| `?` | Context-sensitive help |
| `q` | Quit current panel or view |
| `gg/G` | Jump to top/bottom of list |

### 5.2 Information Density

Marcus prefers **medium-to-high density** interfaces. Don't waste screen real estate.

**Status bar should always show:**
- Current context (file, pipeline, cluster)
- Connection status (mock vs. live)
- Model in use
- Last action timestamp

**Good:**
```
┌────────────────────────────────────────────────────────────┐
│ [Deployment] prod-us-east-1 │ Model: claude-sonnet-4      │
│ Pipeline: deploy.yml │ Stage: validate │ Mock: ON         │
└────────────────────────────────────────────────────────────┘
```

### 5.3 Diff-First Workflow

Before any file modification:

1. Show the exact diff (additions in green, removals in red)
2. Require explicit confirmation to apply
3. Allow partial acceptance (accept some changes, reject others)
4. Support `diff-only` mode for CI integration

### 5.4 Progressive Disclosure

**Level 1 (Inline):** Show keyboard hints at bottom of screen
```
↑↓ Navigate │ Enter Accept │ Esc Cancel │ :cmd Command │ ? Help
```

**Level 2 (On-demand):** Press `?` for categorized help
```
┌────────────────────────────────────────────────────────────┐
│ HELP                                                       │
├────────────────────────────────────────────────────────────┤
│ NAVIGATION                                                 │
│ j/k        Move down/up                                   │
│ gg/G       Jump to top/bottom                             │
│ /          Search                                         │
│                                                            │
│ ACTIONS                                                    │
│ Enter     Accept change                                   │
│ Space     Toggle selection                                │
│ d         Discard change                                  │
│                                                            │
│ PIPELINE                                                  │
│ :diff     Show pending changes                            │
│ :dryrun   Preview without applying                        │
│ :mock     Toggle mock mode                                │
└────────────────────────────────────────────────────────────┘
```

**Level 3 (Documentation):** Full docs, man pages, examples

### 5.5 Color Semantics (TUI Best Practices)

Use colors consistently and meaningfully:

| Color | Meaning |
|-------|---------|
| Green | Added, success, staged |
| Red | Removed, error, danger |
| Yellow | Modified, warning |
| Blue | Selected, links, info |
| Cyan | Secondary info, hints |
| Gray | Disabled, muted |

**Critical:** Never rely on color alone. Always pair with text/icon.

### 5.6 Error Presentation

**Bad:**
```
Error: Something went wrong
```

**Good:**
```
✗ ERROR: terraform/eks-cluster.tf:42
  │
  │─ Undefined variable: 'db_host'
  │
  │  Suggested fix: Add 'db_host' to variables.tf or 
  │  import from 'data "aws_secretsmanager" "db"'
  │
  │  Press '?' for help on this error type
```

### 5.7 Command Palette Design

Fuzzy matching for quick access:

```
┌────────────────────────────────────────────────────────────┐
│ :▌                                                         │
├────────────────────────────────────────────────────────────┤
│ ► deploy to staging                                        │
│   show pipeline diff                                       │
│   switch to mock mode                                      │
│   set model claude-opus                                    │
│   open kubernetes context                                  │
└────────────────────────────────────────────────────────────┘
```

**Prefix conventions:**
- `:` — Commands
- `#` — Search content
- `@` — Symbols/users
- `>` — Run pipeline/action

---

## 6. Default Behaviors That Would Impress Them

### 6.1 Sensible Defaults Out of the Box

Marcus doesn't want to configure everything before getting value. Good defaults:

- **Mock mode enabled by default** — No accidental real API calls during exploration
- **Diff-first for all changes** — Must explicitly accept modifications
- **Conservative model selection** — Favor reliability over speed
- **Verbose logging off by default** — Enable with flag, not disable

### 6.2 Idempotent Operations

Every command should be safe to run multiple times:

```bash
# This should be safe to run repeatedly
runie apply --pipeline deploy.yml --dry-run

# Even with real execution, should detect no-op state
runie apply --pipeline deploy.yml
# → No changes detected. Pipeline is already in desired state.
```

### 6.3 Transparent Rate Limiting

When hitting limits:

```
⚠ RATE LIMIT: 50 requests/minute on claude-sonnet-4
   Used: 45/50 │ Resets: 14 seconds │ ━━━━━━━━━━━░ 90%

   Options:
   [1] Switch to claude-haiku (higher limit)
   [2] Wait 14 seconds
   [3] Continue anyway (may fail)
```

### 6.4 Context Preservation

Don't lose work on interruption:

```
┌────────────────────────────────────────────────────────────┐
│ ⚠ Session restored from /tmp/runie/backup-2024-01-15     │
│   3 pending changes │ Last edit: 2 hours ago              │
│                                                            │
│   [r] Restore session │ [d] Discard │ [?] Help          │
└────────────────────────────────────────────────────────────┘
```

### 6.5 Verbose Mode That's Actually Useful

Not a firehose, but actionable information:

```bash
$ runie --verbose "optimize this pipeline"

[13:42:01] Parsing deploy.yml
[13:42:01] ✓ Valid YAML structure (127 lines)
[13:42:02] Analyzing stage dependencies
[13:42:02] ⚠ Found 2 potential optimizations:
           - Stage 'test' could run parallel to 'build' (no dependency)
           - 'terraform init' could cache provider plugins
[13:42:03] Generating suggestions...
[13:42:03] ✓ Suggestion 1: Parallelize test stage (save ~2min)
[13:42:03] ✓ Suggestion 2: Cache terraform providers (save ~45s)
```

### 6.6 Mock/Recording Mode for CI

Enable deterministic testing:

```bash
# Record a session for later replay
RUNIE_RECORD=./fixtures/deploy-v1.runie runie apply --pipeline deploy.yml

# Replay in CI (no real API calls)
RUNIE_MOCK=./fixtures/deploy-v1.runie runie test --pipeline deploy.yml
# → All tests pass deterministically
```

---

## 7. Scripting and Automation Expectations

### 7.1 Exit Codes as Contracts

Following Unix philosophy, exit codes must be meaningful:

| Code | Meaning |
|------|---------|
| `0` | Success — operation completed as expected |
| `1` | General error — something went wrong |
| `2` | Misuse — invalid arguments or usage |
| `3` | Configuration error — invalid config file |
| `4` | Execution error — command failed during execution |
| `5` | Timeout — operation exceeded time limit |
| `130` | Interrupted (Ctrl+C) |

**Never exit 0 on error, even if "user-friendly."**

### 7.2 stdout/stderr Contract

| Stream | Content |
|--------|---------|
| **stdout** | Primary data output, structured (JSON when `--json`) |
| **stderr** | Diagnostics, progress, warnings, errors |

```bash
# Good: Data to stdout, diagnostics to stderr
runie analyze --pipeline deploy.yml > result.json 2> errors.log

# The above captures clean JSON in result.json
# And human-readable progress/errors in errors.log
```

### 7.3 Structured Output Mode

For scripting, provide machine-parseable output:

```bash
# JSON output for CI integration
runie --json "suggest improvements" | jq '.suggestions[] | select(.confidence > 0.8)'

# Line-based output for piping
runie --format=lines "list environments" | xargs -I {} runie deploy --env {}

# Exit code reflects outcome
runie --json "validate pipeline" > /dev/null && echo "Valid" || echo "Invalid"
```

### 7.4 Non-Interactive/Headless Mode

Every interactive feature must have a CLI equivalent:

```bash
# Interactive mode
runie  # Opens TUI

# Non-interactive mode
runie --non-interactive "optimize deploy.yml"

# Batch mode for scripts
for pipeline in pipelines/*.yml; do
    runie --non-interactive --json "validate $pipeline" > "results/$(basename $pipeline .yml).json"
done
```

### 7.5 Environment Variable Configuration

For automation, prefer env vars over flags:

```bash
# Configure via environment
export RUNIE_MODEL=claude-opus
export RUNIE_MOCK=./fixtures
export RUNIE_TIMEOUT=300
export RUNIE_LOG_LEVEL=debug

# Now runs with those settings
runie apply --pipeline deploy.yml
```

### 7.6 Config File Support

Human-readable, version-controllable config:

```yaml
# ~/.config/runie/config.toml
[defaults]
model = "claude-sonnet-4"
mock_mode = true
timeout_seconds = 120

[ci]
# Override defaults in CI
mock_mode = false  # Actually call APIs
log_level = "info"
require_confirmation = false

[pipeline]
# DevOps-specific defaults
k8s_context = "prod-us-east-1"
terraform_workspace = "production"
```

---

## 8. Composability and Piping Requirements

### 8.1 Unix Philosophy Alignment

Following McIlroy's principle: "Write programs that do one thing well... Expect the output of every program to become the input to another, as yet unknown, program."

Runie should:

1. **Do one thing well** — Don't try to be an all-in-one platform
2. **Output composable data** — stdout should pipe cleanly to other tools
3. **Handle text streams** — "Because that is a universal interface"
4. **Play well with others** — Don't fight the Unix toolchain

### 8.2 Input/Output Patterns

**Input patterns:**
```bash
# Pipe YAML in
cat deploy.yml | runie "suggest optimizations"

# File as argument
runie "explain" deploy.yml

# Multiple inputs
runie "compare" prod.env staging.env

# Stdin + file combination
cat base.yml | runie "merge with" override.yml
```

**Output patterns:**
```bash
# Structured data for piping
runie --json "list issues" | jq '.issues[] | select(.severity == "critical")'

# Line-based for xargs
runie --lines "list pipelines" | xargs -I {} runie test {}

# Formatted reports
runie --format=markdown "audit pipeline" > audit-report.md
```

### 8.3 Integration with Common Tools

Marcus wants seamless integration with his existing stack:

**Git integration:**
```bash
# Show what changed
runie diff HEAD --pipeline deploy.yml | git diff --cached

# Validate on pre-commit
runie --non-interactive "validate" $1 || exit 1
```

**CI/CD integration:**
```bash
# GitHub Actions
- name: Validate Pipeline
  run: runie --json "validate deploy.yml" | jq -e '.valid == true'

# GitLab CI
validate_script:
  - runie --json "validate .gitlab-ci.yml" > validation.json
  - cat validation.json | jq '.issues | length == 0'
```

**Monitoring integration:**
```bash
# Alert on issues
runie --json "analyze pipelines" | jq -r '.issues[].message' | while read msg; do
  pagerduty incident "$msg"
done
```

### 8.4 Tool Composition Examples

Following the grep/sed/awk tradition:

```bash
# Find all pipelines with security issues
runie --json "audit pipelines/" | jq -r '.[] | select(.issues[].type == "security") | .file'

# Batch optimize all YAML files
find . -name "*.yml" | xargs -P4 -I {} runie --non-interactive "optimize {}"

# Generate change summary
runie diff --format=json | jq '{added: [.changes[] | select(.type == "add")], removed: [.changes[] | select(.type == "remove")]}'

# Compare environments
diff <(runie --lines "show env prod") <(runie --lines "show env staging")
```

### 8.5 Minimal, Composable Architecture

Following Eric Raymond's Rule #1: **Modularity**

```
┌────────────────────────────────────────────────────────────┐
│                    Runie Architecture                      │
├────────────────────────────────────────────────────────────┤
│                                                            │
│   ┌─────────────┐      ┌─────────────┐      ┌─────────┐ │
│   │   TUI CLI   │ ←──→ │   Engine    │ ←──→ │  Model   │ │
│   └─────────────┘      └─────────────┘      └─────────┘ │
│        │                    │                    │         │
│        ↓                    ↓                    ↓         │
│   Interactive           Business              API          │
│   Experience            Logic                Calls         │
│                                                            │
│   ┌─────────────┐      ┌─────────────┐                   │
│   │   Headless  │      │  Fixtures/  │                   │
│   │   Mode      │      │  Mocking    │                   │
│   └─────────────┘      └─────────────┘                   │
│                                                            │
└────────────────────────────────────────────────────────────┘

  TUI CLI ←→ Engine ←→ Model Provider API
     ↑           ↑           ↑
     │           │           │
  User       Core        External
  Interface  Logic       Services
```

**Separation benefits:**
- Can replace TUI with CLI without changing engine
- Can test engine with fixtures (no real API calls)
- Can expose engine via API for other integrations

---

## 9. How Runie Can Exceed Their Expectations (Wow Factors)

### 9.1 Predictive Pipeline Optimization

Not just react to queries, but proactively suggest improvements:

```
┌────────────────────────────────────────────────────────────┐
│ 💡 INSIGHT: Your deploy.yml could be 47% faster           │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Current:  ~8.5 minutes                                   │
│  Potential: ~4.5 minutes                                  │
│                                                            │
│  Suggestions:                                             │
│  [1] Parallelize test stage (save 2m 12s)                │
│  [2] Cache Docker layers (save 1m 45s)                    │
│  [3] Skip redundant terraform validate (save 30s)        │
│                                                            │
│  [a] Apply all │ [1-3] Apply specific │ [?] Learn more   │
└────────────────────────────────────────────────────────────┘
```

### 9.2 Kubernetes-Native Integration

Understanding the container-native context:

```bash
# Runie understands k8s contexts
$ runie "debug why pods are crashing"
  
  🔍 Analyzing: pod/default/api-7d9f8b4c-xk2lp
  
  Last 5 restarts:
  ┌──────────────────────────────────────────────────────────┐
  │ Restart 1: 14:32:01  Exit code 1  Memory limit exceeded │
  │ Restart 2: 14:33:45  Exit code 1  OOMKilled             │
  │ Restart 3: 14:36:02  Exit code 137  Killed (signal 9)    │
  └──────────────────────────────────────────────────────────┘
  
  💡 Root cause likely: Memory limit (512Mi) too low for 
     current load. Current usage: 680Mi avg, 890Mi peak.
  
  Suggested fix:
  - Increase memory limit to 1Gi in deployment.yaml
  - Or add horizontal pod autoscaler
  
  [v] View full logs │ [f] Open in editor │ [?] Help
```

### 9.3 Incident Response Mode

Special mode for on-call engineers:

```
┌────────────────────────────────────────────────────────────┐
│ 🚨 INCIDENT MODE — PagerDuty #1234                         │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  Alert: High Error Rate in production                      │
│  Started: 03:14 AM (42 minutes ago)                       │
│                                                            │
│  Quick Actions:                                            │
│  [1] View recent deployments                               │
│  [2] Check resource utilization                            │
│  [3] Rollback last deployment                             │
│  [4] Run diagnostics                                      │
│                                                            │
│  ┌──────────────────────────────────────────────────────┐ │
│  │ Error Rate: ████████████░░░░░░░░ 34% (threshold 1%) │ │
│  │ P99 Latency: 2.3s (threshold 500ms)                 │ │
│  └──────────────────────────────────────────────────────┘ │
│                                                            │
│  Most likely cause: 78% confidence                        │
│  Database connection pool exhaustion in api-service       │
│                                                            │
│  [r] Run suggested remediation │ [d] Detailed analysis    │
└────────────────────────────────────────────────────────────┘
```

### 9.4 Multi-Environment Diff

Compare configurations across environments:

```bash
$ runie diff-environments prod staging

┌────────────────────────────────────────────────────────────┐
│ Environment Diff: prod vs staging                          │
├────────────────────────────────────────────────────────────┤
│                                                            │
│  DEPLOYMENT REPLICAS                                       │
│  prod:      12  │ staging:  4                             │
│                                                            │
│  RESOURCE LIMITS                                           │
│  prod:      2Gi │ staging:  512Mi                        │
│                                                            │
│  FEATURE FLAGS                                            │
│  prod: ✓     │ staging: ✗                                │
│   - new_checkout_flow                                     │
│   - ml_recommendations                                    │
│                                                            │
│  DATABASE                                                  │
│  prod:      rds-prod-xyz │ staging: rds-staging-abc       │
│                                                            │
│  ⚠ 3 significant differences — review before promoting    │
└────────────────────────────────────────────────────────────┘
```

### 9.5 Self-Documenting Pipelines

Generate and maintain documentation automatically:

```bash
$ runie document --pipeline deploy.yml

Generated: docs/pipelines/deploy.md

```markdown
# Deploy Pipeline

## Overview
Deploys application to EKS cluster with zero-downtime strategy.

## Stages

### 1. validate (2m)
- Terraform validation
- Security scanning
- Required approvals: 1

### 2. build (5m)
- Docker image build
- Push to ECR
- Tag with commit SHA

### 3. deploy-staging (3m)
- Deploy to staging cluster
- Run smoke tests
- Wait for approval

### 4. deploy-prod (4m)
- Blue/green deploy to production
- Automated rollback on failure
- Notify Slack #deployments

## Environment Variables
| Name | Required | Description |
|------|----------|-------------|
| AWS_REGION | Yes | Target AWS region |
| DOCKER_TAG | Yes | Image tag to deploy |
| SLACK_WEBHOOK | No | Notification URL |

## Estimated Duration
Total: ~14 minutes
```

### 9.6 Idempotent Configuration Generator

Generate configs that are safe to re-run:

```bash
$ runie scaffold --pipeline kubernetes-api

Generated files:
├── k8s/
│   ├── deployment.yaml     # Production-ready deployment
│   ├── service.yaml        # ClusterIP service
│   ├── hpa.yaml           # Horizontal pod autoscaler
│   └── network-policy.yaml # Security policies
└── README.md              # Generated documentation

All files are idempotent and safe to re-run with:
  kubectl apply -f k8s/
```

### 9.7 The "Least Surprise" Experience

Following Unix philosophy Rule #10: "In interface design, always do the least surprising thing."

```bash
# Marcus types what he means
$ runie "make it faster"
  → Shows optimization suggestions, doesn't auto-apply

$ runie "make it faster --force"
  → Applies changes with confirmation

$ runie "make it faster --yes"
  → Applies all changes, no prompts (for scripts)

# Keyboard shortcuts make sense
$ runie "deploy"   # Opens deploy confirmation
$ Ctrl+C           # Always cancels
$ Ctrl+G           # Go (confirm action)
$ Ctrl+D           # Diff (show changes)
$ Ctrl+H           # Help

# Errors are helpful
$ runie "deploy --invalid-flag"
  → ERROR: Unknown flag '--invalid-flag'
    Did you mean: --dry-run, --env, --pipeline
    Usage: runie deploy [flags]
```

---

## Summary: What Makes Runie a DevOps Engineer's Tool

| Principle | Implementation |
|-----------|---------------|
| **Reliability** | Deterministic mock mode, meaningful exit codes, no silent failures |
| **Composability** | stdin/stdout contracts, JSON output, headless mode |
| **Transparency** | Diff-first workflow, verbose logging, audit trails |
| **Unix Philosophy** | Do one thing well, pipe-friendly, config files over GUIs |
| **Speed** | Instant TUI response, async operations, no loading screens |
| **Trust** | Explicit confirmation, dry-run mode, rollback support |

---

## References

- [Coding Agents UX Research](../research/coding_agents_ux.md) — User trust, verification burden, transparency requirements
- [Unix Philosophy Research](../research/unix_philosophy.md) — Composability, exit codes, text streams
- [TUI Best Practices Research](../research/tui_best_practices.md) — Keyboard navigation, information density, help systems
- [Cognitive Load UX Research](../research/cognitive_load_ux.md) — Progressive disclosure, decision fatigue, context switching

---

*Document version: 1.0*
*Last updated: 2026-07-15*
*Research basis: User interviews, DevOps community feedback, Unix philosophy analysis*
