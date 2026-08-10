# Persona: The Open Source Maintainer

> *"I maintain 12 projects in my spare time. Every minute spent on tooling overhead is a minute not spent on the actual code."*

---

## 1. Persona Profile

### Background

Alex maintains a portfolio of open source projects spanning from a popular CLI utility (2.3k GitHub stars) to several smaller libraries. By day, they're a mid-level software engineer at a Series B startup. Open source contribution is entirely volunteer work squeezed into evenings and weekends.

**Demographics:**
- **Age:** 28-38
- **Location:** Remote (US time zones preferred for community responsiveness)
- **Career stage:** Mid-level engineer, OSS as passion project
- **Available time:** 8-12 hours/week for OSS work, often in 1-2 hour blocks
- **Income from OSS:** $0 (all volunteer)

### Expertise Level

| Dimension | Level | Notes |
|-----------|-------|-------|
| Terminal fluency | Expert | Has `.bashrc` or `.zshrc` configured with dozens of aliases |
| Git mastery | Expert | Rebase workflow, bisect, stash stacks, worktrees |
| Language proficiency | Advanced | Primary language (Rust/Go/TypeScript) + secondary |
| Architecture skills | Intermediate | Can design systems but time-constrained |
| OSS governance | Learning |摸索ing through it as they go |

### Work Style

- **Deep work preference:** 90+ minute focus sessions when possible, but often interrupted by 15-30 minute quick tasks
- **Context switching costs hit hard:** With only 8-12 hours/week, a 23-minute refocus penalty (from cognitive load research) means they might lose 30-40% of an already-scarce session to context recovery
- **Asynchronous-first:** All project communication happens async via GitHub issues, PRs, and Discord. No real-time availability.
- **Batch processing:** Tends to group similar tasks (all code reviews together, all issues triaged together)

---

## 2. Goals and Motivations

### Primary Goals

1. **Maintain code quality with limited time** — Every commit must be something they're proud of, even if it took 20 minutes instead of a professional's 2 hours

2. **Reduce maintainer burden** — Currently spends ~40% of OSS time on "taxes" (triaging issues, reviewing obvious PRs, answering the same questions) instead of creative work

3. **Onboard contributors effectively** — A single good contributor who can handle issues independently is worth more than 10 drive-by PRs

4. **Ship meaningful features** — Their limited time must go toward features that matter, not repetitive implementation work

5. **Protect personal time** — OSS must not become a second job. They need to close the laptop and not think about it.

### Secondary Goals

- Build reputation and network in the developer community
- Learn new technologies through real-world application
- Create something useful that outlasts their current job

### What Motivates Them

- **Impact visibility:** Seeing their library mentioned on Hacker News or used in a notable project
- **Contributor success:** When a first-time contributor becomes a regular reviewer
- **Code elegance:** A clean abstraction that simplifies a complex problem
- **Acknowledgment:** Even a simple "thanks" in an issue makes the work feel worthwhile

### What Demotivates Them

- **Burnout from tedious tasks:** Reviewing 5 PRs that all fix the same typo
- **Entitlement behavior:** "Why hasn't this been merged yet?!" without any prior contribution
- **Scope creep pressure:** Feature requests that would "only take a few minutes" but would actually require months

---

## 3. Pain Points with Current Tools

### P0 — Critical Frustrations

#### 1. Context Switching Between Repos

Currently managing 12 repositories across 3 organizations. The context switching cost is brutal:

```
Switching from project-A (Rust, web scraping lib) to project-B (TypeScript, CLI tool):

1. Close project-A IDE/terminal context
2. Remember: "Was I in the middle of anything?"
3. cd to project-B directory
4. Recall: "What was I working on? Oh right, the --json flag for the export command"
5. Load mental model of project-B's API, recent changes, open PRs
6. Actually start productive work

Time lost: 23+ minutes (per cognitive load research)
```

With 8-12 hours/week, two such context switches can eat an entire session.

#### 2. "Almost Right" AI Suggestions

Per coding_agents_ux.md research: **66% frustration rate with AI solutions that are close but not quite correct.** For OSS work:

- AI suggests imports that don't exist in the crate version they're targeting
- Code looks correct but has subtle API drift from the project's conventions
- Debugging AI-generated code takes longer than writing it manually (45% experience this)

The worst part: the solution *looks* right, so they trust it, but it doesn't work. This multiplies the context switching cost.

#### 3. Tedious Contribution Tax

**Current workflow for a simple doc PR:**
```
1. Fork repo (if not already)
2. Clone fork
3. Create branch
4. Make change
5. Commit (with conventional commit format)
6. Push
7. Open PR
8. Wait for CI
9. Address review comments (often just formatting)
10. Merge
11. Sync upstream
```

For a one-line documentation fix, steps 1-3 and 9-11 are pure overhead. Yet they can't skip them because maintainers need the audit trail.

### P1 — Significant Frustrations

#### 4. Inconsistent Terminal Behavior

Each tool has different conventions:
- `q` quits in lazygit but `Esc` in htop
- `j/k` navigation in some tools, arrow keys in others
- No consistent help key (`?` vs `F1` vs `H`)

This constant mental translation adds up when switching between tools 50+ times per session.

#### 5. CI/CD Pipeline Blindness

When a PR breaks CI, they have to:
1. Click through to CI provider (CircleCI/GitHub Actions)
2. Wait for page load
3. Parse cryptic error logs
4. Try to reproduce locally
5. Fix
6. Push
7. Wait for CI again

No in-terminal visibility into CI state. No ability to quickly debug in the same environment.

#### 6. Code Review Asymmetry

**Ideal:** Review takes 15 minutes, provides valuable feedback, contributor learns something

**Reality:** Review takes 15 minutes, feedback is "please run `cargo fmt`", contributor feels patronized

AI could handle the mechanical review (formatting, obvious nits) and flag the substantive issues for human attention.

---

## 4. What Would Delight This User

### High-Impact Delighters

#### 1. Intelligent Project Switching

Something that understands their multi-repo context:

```
When they open Runie:
┌─────────────────────────────────────────────────────┐
│  Recent Projects                        [3 repos]  │
├─────────────────────────────────────────────────────┤
│  ▶ runie (runie-tests)              2m ago [active]│
│    ↳ Review: @user/pr-234 "fix auth bug"            │
│                                                     │
│    cargo-claude                         1d ago     │
│    ↳ Open: 3 issues, 1 draft PR                      │
│                                                     │
│    dotfiles                                3d ago   │
│    ↳ Last: synced config                          │
└─────────────────────────────────────────────────────┘
```

Not just "recent directories" but *project context*: what was I doing, what's pending, what's urgent.

#### 2. "Review My Code" Mode

An AI-assisted review that handles the mechanical issues:

```
┌─────────────────────────────────────────────────────┐
│  AI Pre-Review: pr-234 "add rate limiting"          │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ✓ Formatting: clean                                │
│  ✓ Tests: 4 new tests, all passing                 │
│  ✓ Documentation: function docs present             │
│                                                     │
│  ⚠ Semantic Issues (2):                             │
│  ├─ L45: Could panic on empty vec [medium]          │
│  └─ L112: Unused import `Debug` [nit]              │
│                                                     │
│  🔍 Design Questions (1):                            │
│  └─ L89: Consider exponential backoff instead of   │
│          fixed 100ms delay? (see similar pattern    │
│          in src/auth.rs:234)                        │
│                                                     │
│  [Approve] [Request Changes] [Comment Only]        │
└─────────────────────────────────────────────────────┘
```

They spend their 15 minutes on the 🔍 section, not chasing down missing imports.

#### 3. Contributor Onboarding Acceleration

When a new contributor submits their first PR:

```
┌─────────────────────────────────────────────────────┐
│  First PR from @newcontributor                      │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Welcome! This looks like your first contribution.  │
│                                                     │
│  Quick checklist I auto-ran:                       │
│  ✓ Matches project's coding style                  │
│  ✓ Tests included                                 │
│  ✓ CHANGELOG updated                              │
│  ✓ commit msg follows conventional commits         │
│                                                     │
│  The code looks good! I've left one suggestion     │
│  about error handling, but overall this is ready   │
│  to merge once CI passes.                          │
│                                                     │
│  Thanks for contributing! 🎉                        │
└─────────────────────────────────────────────────────┘
```

They didn't have to write this. The AI wrote it based on their project conventions. The contributor gets a warm welcome; they don't have to spend emotional labor.

#### 4. Issue Triage Assistance

```
┌─────────────────────────────────────────────────────┐
│  New Issue: "doesn't work on macOS"                 │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Auto-triaged: [Needs Info]                         │
│                                                     │
│  Missing:                                           │
│  □ macOS version                                   │
│  □ Runie version                                   │
│  □ Error output                                    │
│  □ Steps to reproduce                              │
│                                                     │
│  [Draft Reply: Request Info] [Add Labels] [Close]  │
└─────────────────────────────────────────────────────┘
```

With one keystroke, they request the necessary information in a friendly, pre-written way that matches their project's tone.

---

## 5. Specific UI/UX Recommendations for Runie

### 5.1 Multi-Repo Context Management

**Recommendation:** Implement a "project registry" concept that goes beyond directory switching.

| Feature | Rationale | Reference |
|---------|----------|----------|
| Named project shortcuts (`proj use rustlib`) | Cognitive load research: recognition over recall | Show options, don't require memorization |
| Context preservation per project | 23-min refocus cost; if they switch away and back, restore their place | Persistent state across interruptions |
| Visual project dashboard | Current time is fragmented; give them an at-a-glance view | Status bar as memory aid |
| Background sync indicators | Show when repos are out of date without forcing action | Information density principles |

**Implementation guidance:**
- Store project metadata in `~/.config/runie/projects.toml`
- Support `project.add`, `project.list`, `project.switch`
- Preserve per-project state: last branch, open PRs, recent tasks

### 5.2 Terminal-Native CI Visibility

**Recommendation:** Bring CI status into the terminal rather than forcing browser context switches.

```
┌─────────────────────────────────────────────────────┐
│  CI Status: runie (pr-234)                         │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ✓ cargo build        0:42                          │
│  ✓ cargo test        1:23                          │
│  ● clippy            running...  ██████░░ 67%     │
│  ○ cargo doc          pending                         │
│  ○ integration test  pending                         │
│                                                     │
│  [View Logs] [Retry Failed] [Cancel]               │
└─────────────────────────────────────────────────────┘
```

**Rationale:** Every browser tab switch costs ~23 minutes in focus recovery. Keep them in the terminal.

### 5.3 Keyboard-First Workflows

**Recommendation:** Follow established TUI conventions while adding OSS-specific shortcuts.

| Key | Action | Rationale |
|-----|--------|-----------|
| `gr` | Git review (open pending PRs) | "gr" for "git review" is natural |
| `gi` | Git issues (open issues dashboard) | Consistency with `gr` pattern |
| `gc` | Git commits (recent commits) | Quick history without leaving context |
| `gp` | Git pr (PR creation flow) | `gp` as git-push-adjacent |
| `/` | Search across project | Universal search pattern |
| `?` | Context-sensitive help | Industry standard |

**Rationale:** Unix philosophy: "Do the least surprising thing." These map to vim/git conventions most developers already know.

### 5.4 Diff-First AI Interactions

**Recommendation:** Always show AI changes before applying, with clear diffs.

```
┌─────────────────────────────────────────────────────┐
│  AI Suggestion: Add rate limiting                   │
├─────────────────────────────────────────────────────┤
│                                                     │
│  diff --git a/src/lib.rs b/src/lib.rs               │
│  @@ -45,6 +45,8 @@ pub struct Client {               │
│  +    rate_limiter: RateLimiter,                    │
│       }                                             │
│                                                       │
│  [View Full Diff] [Accept] [Edit] [Reject]          │
└─────────────────────────────────────────────────────┘
```

**Rationale:** Research shows **66% frustration** with "almost right" AI solutions. Diff visibility lets them catch issues before the cognitive cost of debugging.

### 5.5 Progressive Disclosure for Complex Actions

**Recommendation:** Don't show all options upfront; reveal as needed.

```
┌─────────────────────────────────────────────────────┐
│  PR Actions                                         │
├─────────────────────────────────────────────────────┤
│                                                     │
│  [Create PR] [Review PR] [Merge]                    │
│                                                     │
│  ▼ Advanced (press Enter or 'a')                    │
│    [Squash Merge] [Rebase Merge] [Close w/o Merge]  │
│    [Create Milestone] [Add to Project]              │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Rationale:** Cognitive load research shows 30-50% faster task completion with progressive disclosure.

---

## 6. Default Behaviors That Would Impress Them

### 6.1 Smart Defaults Based on Project Type

**Current behavior of most tools:** Generic defaults that require configuration

**What would impress them:** Intelligent detection

```
Auto-detected:
- Language: Rust
- Build tool: cargo
- Package registry: crates.io
- CI provider: GitHub Actions
- License: MIT

Applied smart defaults:
- fmt command: cargo fmt
- lint command: cargo clippy
- test command: cargo test --all-features
- doc command: cargo doc --open
```

**Why it matters:** Every minute spent configuring is a minute not writing code. They have 8 hours/week; defaults should work on day one.

### 6.2 Context-Aware Command Palette

**Current behavior:** Generic command search

**What would impress them:** Commands that adapt to current state

```
When on main branch:
> "branch" → [Create Branch] [Switch Branch] [Delete Branch]

When on feature branch with uncommitted changes:
> "branch" → [Stash & Create Branch] [Commit & Create Branch] [Switch Branch]

When in PR review context:
> "branch" → [Checkout PR] [Review PR] [Merge PR]
```

**Why it matters:** Shows the tool understands their workflow, not just their directory.

### 6.3 Non-Intrusive Background Operations

**Current behavior:** Modal loading spinners blocking all interaction

**What would impress them:** Async operations with minimal intrusion

```
[Background] Fetching upstream... done
[Background] Checking 12 open PRs... 3 need review
[Background] CI running for pr-234... ████░░░░ 60%

User can continue working; status bar shows progress
```

**Why it matters:** They're context-switching constantly. Don't add artificial blocking.

### 6.4 Fail Loud, Fail Informatively

**Current behavior:** Silent failures or cryptic error codes

**What would impress them:**

```
Error: Failed to push to origin
├─ Reason: Network unreachable
├─ Tried: 3 attempts over 10s
└─ Suggestions:
   ├─ Check your internet connection
   ├─ Verify SSH key is added to GitHub
   └─ Run: ssh -T git@github.com to test auth
```

**Why it matters:** Unix philosophy: "When you must fail, fail noisily and as soon as possible." Every cryptic error is a context switch to debugging.

### 6.5 Remember and Respect Their Time

**Current behavior:** Same onboarding flow every time

**What would impress them:** Persistent session awareness

```
Welcome back! Resuming project: cargo-claude
├─ Branch: feature/auth-refactor
├─ Recent: 3 new comments on your PR
└─ CI: pr-234 passed ✓

[Continue where I left off] [Start fresh] [Switch project]
```

**Why it matters:** 8-12 hours/week. Don't make them re-establish context every session.

---

## 7. Multi-Project Context Switching Requirements

### 7.1 The Problem

The average OSS maintainer manages 5-15 projects. The cognitive cost of switching:

| Switch Type | Time Cost | Frequency | Weekly Impact |
|-------------|-----------|-----------|---------------|
| Between active PR reviews | 5 min | 20x | 100 min |
| Between active projects | 15 min | 10x | 150 min |
| Between deep work sessions | 23 min | 5x | 115 min |
| **Total context switching tax** | | | **~6 hours/week** |

With 8-12 hours available, **up to 50%** is consumed by context switching overhead.

### 7.2 Runie's Opportunity

Runie can reduce this by:

1. **Project state persistence** — When they switch projects, preserve state
   - Last viewed file
   - Open PRs/Issues they were looking at
   - Recent commands run
   - AI conversation context

2. **Lightweight project switching** — `proj use rustlib` should be <1 second
   - No need to `cd` anywhere
   - No need to re-establish mental context manually
   - Visual indicator of current project

3. **Aggregated dashboards** — See all projects in one view
   ```
   ┌─────────────────────────────────────────────────────┐
   │  All Projects Overview                    [3 active]│
   ├─────────────────────────────────────────────────────┤
   │  ● cargo-claude     2 issues, 1 PR awaiting review  │
   │  ○ rust-scraper     stale (14d)                     │
   │  ● dotfiles         sync pending                    │
   └─────────────────────────────────────────────────────┘
   ```

4. **Smart notifications** — "While you're in rust-scraper, there's a new comment on cargo-claude #45"

### 7.3 Implementation Considerations

| Requirement | Approach |
|-------------|----------|
| State storage | `~/.local/state/runie/projects/` with TOML per project |
| Fast switching | Lazy-load project context; don't load everything on switch |
| Conflict handling | Last-write-wins for state; explicit merge for conversation context |
| Privacy | Local storage only; no cloud sync by default |
| Portability | `~/.config/runie/projects.toml` defines the project registry |

---

## 8. Contribution Workflow Optimization Needs

### 8.1 The Contributor Funnel

```
                    ┌─────────────┐
                    │  Visitors   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ First Issue │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ First PR    │ ← Drop-off point #1
                    └──────┬──────┘   (hard to get started)
                           │
                    ┌──────▼──────┐
                    │ Regular     │ ← Drop-off point #2
                    │ Contributor │   (burnout, no recognition)
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
                    │ Core        │
                    │ Maintainer  │ ← Very rare
                    └─────────────┘
```

### 8.2 Where Runie Can Help

#### Reduce Barrier to First Contribution

**Current friction:**
1. Fork repo
2. Clone fork
3. Create branch
4. Make change
5. Commit
6. Push
7. Open PR

**Runie's opportunity:** Streamline step 4-7

```
# In Runie, on an issue they're assigned to:
/claim "implement rate limiting"

AI generates a branch with:
- Skeleton implementation based on issue description
- Test stubs
- CHANGELOG entry
- Commit message following conventional commits

[Review Skeleton] [Accept & Start] [Edit First]
```

#### Reduce Maintainer Review Burden

**Current friction:**
- 50% of PRs need only mechanical fixes (formatting, typos, CHANGELOG)
- Reviewer spends time on nits instead of substantive feedback
- Contributors feel patronized by "please run cargo fmt" comments

**Runie's opportunity:** Pre-review that handles the mechanical

```
┌─────────────────────────────────────────────────────┐
│  Pre-Review Summary for pr-234                      │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Ready for human review:                            │
│  ✓ Follows code style                              │
│  ✓ Tests pass                                      │
│  ✓ No obvious performance issues                   │
│                                                     │
│  Author should address before merge:               │
│  ⚠ Missing CHANGELOG entry                        │
│  ⚠ Test coverage dropped 0.2%                     │
│                                                     │
│  Questions for reviewer:                           │
│  ? L89: Is the 100ms timeout intentional?         │
│                                                     │
│  [Request Changes] [Approve with Nits] [Comment]   │
└─────────────────────────────────────────────────────┘
```

### 8.3 Workflow Templates

**For the maintainer's own contributions to other OSS projects:**

```
Runie can record and replay contribution workflows:

1. "I want to contribute to tokio"
2. Runie detects similar project structure
3. Offers to scaffold based on their past patterns:
   - Branch naming: `{type}/{description}` (e.g., `fix/memory-leak`)
   - Commit format: conventional commits
   - PR template: auto-fills from their boilerplate
   - CI-aware: shows real-time CI status
```

### 8.4 Code Review Quality Metrics

Track but don't pressure:

| Metric | Purpose |
|--------|---------|
| Time to first review | How quickly do they respond? |
| Review thoroughness | Are they catching real issues? |
| Contributor satisfaction | Did the contributor learn something? |
| Rework rate | How often does PR need major revision? |

---

## 9. How Runie Can Exceed Their Expectations (Wow Factors)

### 9.1 The "Maintainer Assistant" Mode

A mode that specifically assists maintainers, not just users:

```
/maintain

Running Maintainer Assistant:
├─ Scanning 12 projects for activity...
├─ 3 new issues need triage
├─ 2 PRs need review
├─ 1 stale PR can be closed
└─ 0 security advisories

Actions:
[Review 2 PRs] [Triage Issues] [Close Stale PRs] [Dismiss]
```

**Wow factor:** It thinks like a maintainer. It understands the "taxes" of open source and automates the tedious parts.

### 9.2 Contributor Reputation Memory

```
┌─────────────────────────────────────────────────────┐
│  @rustacean-first contribution!                    │
├─────────────────────────────────────────────────────┤
│                                                     │
│  Welcome! This looks like your first PR to cargo-claude │
│                                                     │
│  Quick notes:                                       │
│  • We use conventional commits (cargo commit)      │
│  • Run `cargo test --all-features` before PR       │
│  • Check CHANGELOG.md for format                   │
│                                                     │
│  I've auto-checked your PR against these:           │
│  ✓ Formatting clean                                │
│  ✓ Tests included                                 │
│  ✓ CHANGELOG follows format                        │
│                                                     │
│  The code looks good! 🎉                            │
└─────────────────────────────────────────────────────┘
```

**Wow factor:** The AI wrote the welcome message *for them*, in their project's voice, based on their conventions.

### 9.3 "Time Boxing" Mode

Given their limited time (8-12 hours/week), help them be intentional:

```
/timebox 2h

Remaining this week: 6h 23m
├─ In progress: cargo-claude auth refactor (2h left)
├─ Queued: 3 issues triaging (~30m)
└─ Backlog: 12 issues, 5 PRs

Suggestions for next 2 hours:
├─ [A] Finish auth refactor (2h) → ship feature
├─ [B] Review 2 waiting PRs (1h) + triage (1h) → community care
└─ [C] Triage 12 issues (1h) + 1 PR (1h) → maintenance

What matters most this week?
```

**Wow factor:** Respects their time constraint. Helps them make intentional choices instead of just reacting to notifications.

### 9.4 "Merge Confidence" Score

Before they click merge, give them confidence:

```
┌─────────────────────────────────────────────────────┐
│  Merge Confidence: 87%                              │
├─────────────────────────────────────────────────────┤
│                                                     │
│  ✓ CI: All checks passed                           │
│  ✓ Tests: 4 new, 12 existing pass                  │
│  ✓ Coverage: 94.2% (+0.3%)                         │
│  ✓ Breaking changes: none detected                 │
│                                                     │
│  ⚠ Review: 1 approval, 1 pending                   │
│  ⚠ Dependencies: 1 new, unmaintained (3 stars)    │
│                                                     │
│  [Merge Anyway] [Wait for 2nd Review] [Message Author]│
└─────────────────────────────────────────────────────┘
```

**Wow factor:** Takes the anxiety out of merging. They can see at a glance whether this PR is safe.

### 9.5 Offline-First Architecture

As a volunteer working in coffee shops, airports, and anywhere with flaky wifi:

```
[Offline] Last synced: 23 minutes ago
├─ Cached: Current branch, recent commits, open PRs
├─ Available: Read issues, read code, read PRs
├─ Queued: 1 comment, 1 review draft
└─ Will sync: When connection restored

[Write Offline] [View Cached] [Queue Actions]
```

**Wow factor:** Doesn't punish them for being in the real world. Their limited time shouldn't be wasted waiting for a spinner.

---

## 10. Summary: Design Principles for Runie

Based on research and this persona's needs:

| Principle | Rationale | Source |
|-----------|----------|--------|
| **Speed is sacred** | Every second of waiting is a context-switch opportunity | TUI Best Practices |
| **23-minute rule** | Design to minimize focus recovery time | Cognitive Load UX |
| **Recognition over recall** | Show options, don't require memorization | Cognitive Load UX |
| **Diff-first AI** | Trust requires visibility; 66% frustrated by "almost right" | Coding Agents UX |
| **Progressive disclosure** | 30-50% faster task completion | Cognitive Load UX |
| **Fail loudly** | Every cryptic error is wasted time | Unix Philosophy |
| **Tool composability** | Output should flow to other tools | Unix Philosophy |
| **Context persistence** | Every context switch costs 23 minutes | Research (OSS Maintainer specific) |

---

## Appendix: Persona Validation Checklist

Use this to validate Runie's design decisions against this persona:

- [ ] Can they switch between 3 projects in under 10 seconds?
- [ ] Does the command palette adapt to current context?
- [ ] Are AI suggestions shown as diffs before applying?
- [ ] Does the status bar answer: where am I, what's pending, what's happening?
- [ ] Can they triage an issue without leaving the terminal?
- [ ] Does the app work offline with queued sync?
- [ ] Are defaults smart enough to work without configuration?
- [ ] Can they review a PR in under 15 minutes with full confidence?
- [ ] Does the help system show only relevant keys?
- [ ] Do errors include actionable suggestions?

---

*Document version: 1.0*
*Created: 2026-07-15*
*Last updated: 2026-07-15*
*Research sources: coding_agents_ux.md, unix_philosophy.md, tui_best_practices.md, cognitive_load_ux.md*
