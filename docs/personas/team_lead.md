# Persona Analysis: The Team Lead

**Persona Type:** Force Multiplier / Multiplier Persona  
**Confidence Level:** High technical, managing complexity at scale  
**Risk Profile:** Strategic — affects team productivity, onboarding velocity, and code quality consistency

---

## Executive Summary

The Team Lead is a senior engineer turned technical manager who must simultaneously maintain technical credibility, drive team productivity, and enforce quality standards across multiple contributors. Their relationship with AI coding tools is fundamentally different from individual contributors: they care less about personal speed and more about team-wide consistency, knowledge transfer, and reducing onboarding friction. Runie must be their lever for scaling senior expertise across the entire team—not another tool that creates chaos.

---

## 1. Persona Profile

### Background

| Attribute | Description |
|-----------|-------------|
| **Experience** | 8-15+ years, with 2-5 years in leadership |
| **Education** | Computer science or equivalent deep technical background |
| **Technical Foundation** | Deep expertise in multiple languages/frameworks, system design, architecture decisions |
| **Current Role** | Engineering Manager, Tech Lead, or Staff Engineer with people leadership |
| **Work Style** | Strategic multitasker; orchestrates rather than executes; proxy for team decisions |

### Expertise Level

**Technical State:**
- Can evaluate any code review with authority
- Makes architectural decisions that affect months of work
- Understands trade-offs between speed and maintainability
- Has battle scars from past technical debt decisions
- Recognizes patterns across languages and frameworks

**Leadership State:**
- Responsible for 4-12 engineers of varying skill levels
- Must enforce consistency without becoming a bottleneck
- Balances shipping pressure with quality standards
- Onboarding new team members is a recurring priority
- Performance reviews require objective code quality metrics

### Work Environment

- **Primary:** Combination of IDE, code review tools, and documentation
- **Secondary:** Run meetings, Slack/Teams, project management tools
- **Context:** Reviewing others' code more than writing their own
- **Pressure:** Delivery deadlines, team velocity, hiring pipeline, tech debt

---

## 2. Goals and Motivations

### Primary Goals

1. **Scale Senior Expertise** — One senior engineer can only review so much. They need tools that encode team standards so *every* developer can meet them.

2. **Reduce Onboarding Friction** — Every week a new hire spends learning conventions is a week not shipping. They want new team members productive in days, not weeks.

3. **Enforce Consistency Without Bottlenecks** — They can't personally review every PR or pair on every decision. They need automated guardrails that enforce standards autonomously.

4. **Maintain Technical Credibility** — As they spend more time in meetings, they risk falling behind. They need tools that help them stay technically sharp.

5. **Reduce Cognitive Load of Team Management** — They manage enough complexity. They don't want another tool that adds to the burden.

### Motivational Drivers

| Driver | Expression |
|--------|------------|
| **Leverage** | "If I can encode our standards into Runie, every developer gets senior-level guidance" |
| **Scale** | "I can't review 50 PRs a week myself, but Runie can help enforce consistency" |
| **Velocity** | "Faster onboarding means faster shipping" |
| **Quality** | "We ship features, not bugs" |
| **Teaching** | "I want juniors to learn from Runie, not just get code" |

---

## 3. Pain Points with Current Tools

### The Consistency Paradox

> "We have coding standards documented in Confluence. Nobody reads them. We have PR templates. Nobody fills them out. We have architecture decision records. They're written after the fact. Every developer interprets 'follow our conventions' differently."

**Problem:** Senior developers enforce standards in code review, but by then it's expensive to fix. Junior developers don't know what they don't know.

**Source:** [Unix Philosophy - Section 2.5]

### Onboarding Velocity Death

> "Our onboarding takes 3 months to full productivity. Half that time is learning conventions that could be automated. In a competitive hiring market, that's a liability."

**Problem:** Manual knowledge transfer is slow, inconsistent, and doesn't scale. Each senior developer "teaches" differently.

**Source:** [Cognitive Load UX - Section 7]

### The Code Review Bottleneck

> "I spend 40% of my day reviewing code. I want to add value there, not catch missing error handling or inconsistent naming. But if I don't catch it, it ships."

**Problem:** Senior time is consumed by tedious quality checks that could be automated. Runie should handle the mechanical enforcement.

**Source:** [Coding Agent UX Research - Section 1.1]

### AI Tool Fragmentation

> "My team uses Claude Code, Cursor, Copilot, and Continue. Each has different context handling, different quality, different conventions. My codebase is a Frankenstein of AI-influenced patterns."

**Problem:** Without team-wide AI tool policies, developers fragment across tools, creating inconsistency in AI-generated code quality and style.

**Source:** [Coding Agent UX Research - Section 5.6]

### The "Works on My Machine" of AI

> "Developers run Claude Code locally, see great results, but the code doesn't match our patterns. It's technically correct but stylistically wrong. That's still wrong."

**Problem:** AI tools optimize for correctness, not team conventions. Runie should bridge this gap by encoding project-specific standards.

### Invisible Quality Degradation

> "I can't tell from a PR diff if the developer used AI. But I can tell from the patterns. AI-generated code often looks correct but violates subtle conventions that don't show up as errors."

**Problem:** AI-assisted code looks good in isolation but doesn't match team patterns. This is invisible unless you know what to look for.

### Knowledge Transfer is One-Way

> "When a senior dev leaves, they take years of context with them. Yes, we have documentation. But documentation is never complete. We need tools that capture implicit knowledge."

**Problem:** Team conventions exist in senior developers' heads, not in code. AI tools could encode this knowledge, but most don't.

---

## 4. What Would Delight This User

### The Senior Developer Proxy

> "Runie knows our conventions better than some of my senior devs. When a junior uses it, the code comes back matching our patterns. That's the leverage I've been looking for."

**Delight Trigger:** Tool as encoded senior expertise. Team standards visible to everyone.

**How Runie Delivers:**
- Project-specific configuration that encodes team conventions
- Automated enforcement of naming, error handling, documentation standards
- "In this project, we prefer..." language that reflects team voice
- Senior-level guidance available to every developer

### Onboarding in Days, Not Weeks

> "We onboarded a new React developer last month. With Runie configured to our patterns, they were productive in 3 days instead of 3 weeks. That's hiring pipeline velocity."

**Delight Trigger:** Configurable conventions that accelerate ramp-up.

**How Runie Delivers:**
- Pre-configured project standards loaded from repo
- Contextual hints that teach while coding
- "You've violated our convention: [link to explanation]"
- Progressive disclosure of team standards

### The Quality Filter

> "Runie caught a potential SQL injection in a PR that slipped past two reviewers. That's not just a code review tool—that's a safety net."

**Delight Trigger:** Automated quality enforcement that catches what humans miss.

**How Runie Delivers:**
- Pre-commit hooks that validate conventions
- Inline warnings when patterns violate team standards
- Security scanning integrated into the workflow
- Configurable rule severity levels

### Consistent AI Quality

> "Everyone on my team uses Runie. The code is consistent regardless of who generated it. That's the consistency I've always wanted."

**Delight Trigger:** Team-wide tool adoption that produces consistent output.

**How Runie Delivers:**
- Shared configuration across the team
- Centralized convention updates
- Consistent quality regardless of individual developer skill
- Team learning that accumulates in shared config

### Visibility Without Micromanagement

> "I can see that my team is using Runie and following conventions. I don't need to micromanage code style because Runie handles it. That's saved me hours of review time."

**Delight Trigger:** Oversight that doesn't require hands-on involvement.

**How Runie Delivers:**
- Team-level analytics on convention adherence
- Automated reminders for common violations
- Configuration that's auditable and version-controlled
- Escalation paths for edge cases

### The Knowledge Capturer

> "Our longest-tenured dev retired last quarter. But her patterns live on in our Runie config. New developers learn her conventions without ever meeting her."

**Delight Trigger:** Implicit knowledge made explicit and scalable.

**How Runie Delivers:**
- Convention files that capture team wisdom
- Rationale explanations for "why we do it this way"
- Historical context for architectural decisions
- Living documentation that updates with the team

---

## 5. Specific UI/UX Recommendations for Runie

### 5.1 Team Configuration Architecture

**Principle from Research:**
> "Separation of policy from mechanism; separate interfaces from engines."
> — [Unix Philosophy - Section 1.2]

**Implementation for Team Leads:**

```
runie/
├── config/
│   ├── base.toml           # Company-wide defaults
│   ├── team.toml          # Team-specific overrides
│   └── project.toml       # Project-specific conventions
└── rules/
    ├── naming.toml         # Naming conventions
    ├── error_handling.toml # Error patterns
    ├── security.toml       # Security rules
    └── documentation.toml  # Doc requirements
```

**Key Principles:**
- Hierarchical configuration (company → team → project)
- Version-controlled configuration in repo
- Merge strategy for conflicting rules
- Override mechanism for legitimate exceptions

### 5.2 Convention Configuration UI

**Principle from Research:**
> "Configuration via text files (dotfiles, YAML, TOML) — human-readable, version-controllable."
> — [Unix Philosophy - Section 3.4]

**Team Lead Configuration Interface:**

```
┌─────────────────────────────────────────────────────────────┐
│  TEAM CONVENTIONS                                          │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Naming Conventions                              [Edit ▼]   │
│  ─────────────────────────────────────────────────────────  │
│  ✓ snake_case for functions                                │
│  ✓ PascalCase for types                                     │
│  ✓ SCREAMING_SNAKE_CASE for constants                      │
│  ✓ Prefix errors with "err_"                               │
│                                                             │
│  Error Handling                                  [Edit ▼]   │
│  ─────────────────────────────────────────────────────────  │
│  ✓ Always handle Result<T, E> explicitly                   │
│  ✓ Chain error context with map_err                        │
│  ✓ Log errors at appropriate level                         │
│  ✓ Never swallow errors silently                           │
│                                                             │
│  Documentation                                 [Edit ▼]   │
│  ─────────────────────────────────────────────────────────  │
│  ✓ Doc comments on public APIs                             │
│  ✓ README required for new modules                        │
│  ✓ CHANGELOG entry for breaking changes                   │
│  ✓ Inline comments for complex logic (>5 lines)           │
│                                                             │
│  Security                                         [Edit ▼]  │
│  ─────────────────────────────────────────────────────────  │
│  ⚠ Never commit secrets (hardcoded API keys)               │
│  ⚠ Validate all user input                                │
│  ⚠ Use parameterized queries (no string concatenation)     │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│  [Import from GitHub]  [Share with Team]  [Export Config]   │
└─────────────────────────────────────────────────────────────┘
```

### 5.3 Project Initialization Wizard

**Principle from Research:**
> "Works out of the box without configuration."
> — [TUI Best Practices - Section 7]

**Team Lead Setup Flow:**

```
┌─────────────────────────────────────────────────────────────┐
│  NEW PROJECT SETUP                                          │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Step 1: Select your stack                                 │
│  ┌───────────────────────────────────────────────────────┐│
│  │ [X] Rust   [ ] Python   [ ] TypeScript   [ ] Go        ││
│  └───────────────────────────────────────────────────────┘│
│                                                             │
│  Step 2: Team conventions                                  │
│  ┌───────────────────────────────────────────────────────┐│
│  │ Use company-wide conventions? [Y/n]                   ││
│  │ → Loading company/Rust.toml...                        ││
│  │ → 47 rules loaded                                      ││
│  │                                                      ││
│  │ Add project-specific rules?                           ││
│  │ [ ] Yes, I'll configure now                          ││
│  │ [ ] No, use defaults only                             ││
│  └───────────────────────────────────────────────────────┘│
│                                                             │
│  Step 3: Integration setup                                  │
│  ┌───────────────────────────────────────────────────────┐│
│  │ [X] Add pre-commit hook                              ││
│  │ [X] Configure editor plugin                           ││
│  │ [ ] Set up CI integration                             ││
│  │ [ ] Add to CI/CD pipeline                             ││
│  └───────────────────────────────────────────────────────┘│
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│  [Back]  [Next]  [Finish]  [Save Config for Later]        │
└─────────────────────────────────────────────────────────────┘
```

### 5.4 Convention Violation Dashboard

**Principle from Research:**
> "Design for visibility to make inspection and debugging easier."
> — [Unix Philosophy - Section 1.2]

**Team-Level Visibility:**

```
┌─────────────────────────────────────────────────────────────┐
│  TEAM QUALITY METRICS                                       │
│  ┌───────────────────────────────────────────────────────┐│
│  │ Convention Adherence          This Week  vs Last Week ││
│  │ ─────────────────────────────────────────────────────  ││
│  │ Naming conventions            94%         ↑ +3%       ││
│  │ Error handling                87%         ↑ +5%       ││
│  │ Documentation                 76%         ↓ -2%       ││
│  │ Security rules               100%         → stable    ││
│  │                                                       ││
│  │ Overall Score                 89%         ↑ +2%       ││
│  └───────────────────────────────────────────────────────┘│
│                                                             │
│  TOP VIOLATIONS (Requiring Attention)                      │
│  ─────────────────────────────────────────────────────────  │
│  1. "Use ? operator instead of match"      12 occurrences │
│  2. "Missing doc comments on public fn"     8 occurrences │
│  3. "Consider using this constant"          5 occurrences │
│                                                             │
│  [View Details]  [Dismiss]  [Add to Onboarding]             │
└─────────────────────────────────────────────────────────────┘
```

### 5.5 Knowledge Transfer Documentation

**Principle from Research:**
> "Fold knowledge into data so program logic can be stupid and robust."
> — [Unix Philosophy - Section 1.2]

**Conventions with Rationale:**

```toml
# conventions/error_handling.toml

[[rules]]
id = "no_silent_errors"
description = "Never use expect() in production code"
severity = "error"

rationale = """
This rule enforces explicit error handling.

Background:
- Silent failures cause production incidents that are hard to debug
- .expect() crashes the program; .unwrap() is worse
- Our incident post-mortems show 3 outages from unwrap() in 2024

Exceptions:
- Tests may use unwrap() for obvious values
- Prototyping code marked with #[allow(unused)]
"""

examples = [
    { bad = 'let value = parse(input).unwrap();', good = 'let value = parse(input)?;' },
]

references = [
    "RFC-0042: Error handling policy",
    "Our incident post-mortem from 2024-03-15",
]
```

### 5.6 Senior Developer Encoded Wisdom

**Principle from Research:**
> "Use tools in preference to unskilled help to lighten a programming task."
> — [Unix Philosophy - Section 1.1]

**Contextual Guidance:**

```
┌─────────────────────────────────────────────────────────────┐
│  CONTEXTUAL CONVENTION REMINDER                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  You wrote:                                                 │
│  ┌───────────────────────────────────────────────────────┐│
│  │ fn process_user_data(data: Vec<u8>) -> Result<String> ││
│  │     // ... implementation                              ││
│  │ }                                                      ││
│  └───────────────────────────────────────────────────────┘│
│                                                             │
│  In this project, we prefer:                                │
│  ┌───────────────────────────────────────────────────────┐│
│  │ fn process_user_data(                                ││
│  │     data: &[u8],                                      ││
│  │     ctx: &ProcessingContext,                          ││
│  │ ) -> Result<String, ProcessError>                     ││
│  │                                                       ││
│  │ Why? We always pass context for observability.       ││
│  │ See: docs/architecture/context-propagation.md         ││
│  │                                                      ││
│  │ [Accept Suggestion]  [Explain More]  [Disable Rule]  ││
│  └───────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## 6. Default Behaviors That Would Impress Them

### 6.1 Smart Team Defaults

**Default Behavior:**
- Runie auto-detects project conventions from existing code
- "I noticed your codebase uses [convention]. Should I follow it?"
- Learns from accepted/rejected suggestions over time

**Why It Impresses:**
Shows Runie adapts to the team, not the other way around.

### 6.2 Configurable Strictness Levels

**Default Behavior:**
```
Runie Mode: [Standard ○──●──○ Strict]
```

- **Standard:** Suggestions and gentle reminders
- **Standard+:** Inline hints with explanations
- **Strict:** Blocks commits that violate critical rules

**Why It Impresses:**
Respects team diversity—some teams want guardrails, others want suggestions.

### 6.3 Cross-Repo Consistency

**Default Behavior:**
- Shared team configuration loaded from central location
- Updates propagate to all team members
- "Your Runie config is 3 versions behind. Update? [Y/n]"

**Why It Impresses:**
Ensures consistency without micromanagement.

### 6.4 Escalation Paths

**Default Behavior:**
```
┌─────────────────────────────────────────────────────────────┐
│  AMBIGUOUS VIOLATION                                        │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  This code violates your naming convention, but it's        │
│  following an external API that uses camelCase.              │
│                                                             │
│  Option A: Ignore this instance                             │
│  Option B: Add to project exceptions list                   │
│  Option C: Ask team lead (notified for review)               │
│                                                             │
│  [A]  [B]  [C]                                              │
└─────────────────────────────────────────────────────────────┘
```

**Why It Impresses:**
Provides graceful escalation for edge cases without blocking work.

### 6.5 Audit Trail Visibility

**Default Behavior:**
```
$ runie audit --last-month
Conventions enforced: 847
Violations caught: 23
Auto-fixed: 18
Escalated to human: 5

Top improvement areas:
- Documentation (improved 12%)
- Error handling (improved 8%)
```

**Why It Impresses:**
Demonstrates measurable ROI on convention enforcement.

---

## 7. Onboarding and Consistency Requirements

### 7.1 New Developer Onboarding

**Principle from Research:**
> "Every week a new hire spends learning conventions is a week not shipping."
> — [Pain Point Section]

**Onboarding Configuration:**

```
┌─────────────────────────────────────────────────────────────┐
│  NEW TEAM MEMBER ONBOARDING                                 │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Welcome to the team! Runie is configured with our         │
│  conventions. As you code, you'll see hints that explain   │
│  why we do things a certain way.                            │
│                                                             │
│  Quick Start:                                               │
│  1. [Get up to speed] Your first week with Runie           │
│  2. [See conventions] Review our coding standards          │
│  3. [Configure editor] VS Code / JetBrains integration      │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Convention Quick Reference:                                 │
│  • Naming: snake_case functions, PascalCase types           │
│  • Errors: Always use Result<T, E>, never unwrap()          │
│  • Docs: doc comments on all public APIs                   │
│  • Tests: 80% coverage minimum, integration tests required │
│                                                             │
│  [Start Coding]  [Take Tour]  [View Full Docs]              │
└─────────────────────────────────────────────────────────────┘
```

### 7.2 Convention Discovery

**Principle from Research:**
> "Progressive disclosure manages complexity — show 80% of what 80% of users need; reveal the rest on demand."
> — [Cognitive Load UX - Section 4]

**Learning Flow:**

```
Phase 1: Implicit Learning (First Week)
────────────────────────────────────────
Runie shows inline hints as violations occur:
"Tip: In this project, we use snake_case for function names.
See: team/runbook.md#naming"

Phase 2: Explicit Reference (On Demand)
───────────────────────────────────────
runie conventions list
runie conventions show error_handling

Phase 3: Deep Understanding (When Curious)
───────────────────────────────────────────
runie conventions explain no_silent_errors
→ Full rationale, examples, exceptions
```

### 7.3 Consistent Tooling Across Team

**Principle from Research:**
> "Consistent quality > occasional brilliance."
> — [Coding Agent UX Research - Section 8]

**Version-Controlled Configuration:**

```bash
# .github/runie/
├── conventions.toml      # Committed to repo
├── rules/
│   └── *.toml           # Modular rules
└── .runieignore         # Files/dirs to ignore
```

**Benefits:**
- Configuration lives with the code
- PRs include convention changes when needed
- New team members clone config with repo
- Rollback is git-based

### 7.4 Enforcement Flexibility

**Principle from Research:**
> "Beginner-friendly defaults, power-user shortcuts available."
> — [TUI Best Practices - Section 6]

**Enforcement Levels:**

| Level | Behavior | Use Case |
|-------|----------|----------|
| **Learn** | Hints only, no blocking | Junior developers learning |
| **Suggest** | Inline suggestions, soft warnings | Standard development |
| **Enforce** | Block violations, require override | CI/CD pipelines |
| **Audit** | Report violations, no blocking | Leadership visibility |

---

## 8. Team Productivity and Knowledge Transfer

### 8.1 Scaling Senior Expertise

**Principle from Research:**
> "Programmer time is expensive; conserve it in preference to machine time."
> — [Unix Philosophy - Section 1.2]

**The Leverage Equation:**

```
Senior Engineer Hours:
├── Code Review (high value)
│   └── Runie handles mechanical checks
├── Architectural decisions (high value)
│   └── Senior engineers make, Runie enforces
├── Convention enforcement (low value)
│   └── Runie automates entirely
└── Teaching juniors (medium value)
    └── Runie provides context-rich guidance

RESULT: Senior engineers freed for high-value work
```

### 8.2 Knowledge Capture System

**Principle from Research:**
> "Design for the future, because it will arrive sooner than you think."
> — [Unix Philosophy - Section 1.2]

**Conventions as Documentation:**

```rust
// convention: documentation_on_public_apis
//
// RATIONALE:
// Our API docs are generated from doc comments.
// Without them, our OpenAPI spec is incomplete.
// This affects developer experience for API consumers.
//
// WHO DECIDED THIS:
// Tech lead Sarah Chen, Q3 2024
// After API consumer feedback that docs were out of sync.
//
// EXCEPTIONS:
// Internal utilities marked #[doc(hidden)]
// Test helpers
//
// LAST UPDATED: 2026-01-15 by tech-lead
```

### 8.3 Team Learning Metrics

**Visibility Dashboard:**

```
┌─────────────────────────────────────────────────────────────┐
│  TEAM LEARNING INSIGHTS                                     │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Convention Mastery (Last 30 Days)                         │
│  ┌───────────────────────────────────────────────────────┐│
│  │ Alice  ████████████████░░░░░  78%                     ││
│  │ Bob    ██████████████████░░░  88%                     ││
│  │ Carol  ████████████████████  95%                      ││
│  │ Dave   ████████████░░░░░░░░  52%  ← Needs attention ││
│  └───────────────────────────────────────────────────────┘│
│                                                             │
│  Most Improved:                                            │
│  • Error handling: +15% adherence this month               │
│  • Documentation: +8% adherence this month                 │
│                                                             │
│  Hotspots (New Violations This Week):                       │
│  • 3x: "Use Result instead of panicking"                   │
│  • 2x: "Missing doc comment"                               │
│                                                             │
│  [View Full Report]  [Export for 1:1s]  [Set Goals]        │
└─────────────────────────────────────────────────────────────┘
```

### 8.4 Handoff Documentation

**Principle from Research:**
> "Expect the output of every program to become the input to another, as yet unknown, program."
> — [Unix Philosophy - Section 1.1]

**Automated Context for Handoffs:**

```bash
$ runie context export --for dev-onboarding
{
  "project": "payment-service",
  "language": "Rust",
  "conventions_version": "2.4.1",
  "key_patterns": [
    "error handling via thiserror",
    "async with tokio",
    "structured logging with tracing"
  ],
  "common_mistakes": [
    "Don't use unwrap() - see conventions/error_handling.toml",
    "Remember to add spans for async operations"
  ],
  "links": {
    "architecture": "docs/architecture.md",
    "onboarding": "docs/onboarding.md",
    "conventions": "runie/conventions.toml"
  }
}
```

---

## 9. How Runie Can Exceed Their Expectations (Wow Factors)

### Wow Factor 1: The Senior Shadow

> "My new hire committed code on their second day. It followed every convention perfectly. I asked how they learned so fast. They said 'Runie kept telling me what to do and why.' That's when I realized—our conventions have never been this consistent."

**What Makes It Happen:**
- Configured conventions with rich explanations
- Contextual hints that teach while coding
- No learning curve beyond "accept/reject suggestions"
- Same guidance for every developer, every time

### Wow Factor 2: The Code Review Multiplier

> "I used to spend 2 hours a day on code review, mostly catching style issues. Now I spend 30 minutes because Runie catches the mechanical stuff. I can focus on architecture and design—the things only humans should review."

**What Makes It Happens:**
- Automated convention enforcement before review
- Pre-commit hooks that block common issues
- Developer-facing feedback before PR submission
- Reviewer time redirected to high-value analysis

### Wow Factor 3: The Institutional Memory

> "We had a senior engineer leave last year. They'd been here 8 years and knew everything about our patterns. When I looked at our Runie config, I realized their knowledge was encoded there. Their patterns live on in every code review."

**What Makes It Happens:**
- Conventions files that capture decisions, not just rules
- Rationale attached to every standard
- Historical context for why things are done
- Living documentation that updates with team

### Wow Factor 4: The Onboarding Time Machine

> "We cut our onboarding from 3 months to 3 weeks. New developers are productive faster because Runie teaches them our conventions as they code. They don't have to wait for code review to learn what we expect."

**What Makes It Happens:**
- Convention education integrated into workflow
- Progressive disclosure of team standards
- Contextual explanations with examples
- No separate "read the docs" step required

### Wow Factor 5: The Quality Flywheel

> "Six months ago, our convention adherence was 60%. Now it's 92%. We didn't add more reviewers or have more meetings. We just configured Runie and let it do the work. Quality improved because every developer had a senior developer in their editor."

**What Makes It Happens:**
- Consistent enforcement across all developers
- Measurable improvement over time
- Data-driven quality conversations
- Positive feedback loops that encourage adoption

### Wow Factor 6: The Exception Handler

> "We had a legitimate exception to our rule about no panicking. Runie let us document it, explain it, and track it. Six months later, another developer hit the same situation. Runie said 'This has an exception: [link to rationale].' They were unblocked in seconds instead of waiting for my decision."

**What Makes It Happens:**
- Exception documentation with reasoning
- Approved exceptions visible to all developers
- Automatic exception application in similar contexts
- Audit trail for all exceptions granted

---

## 10. Anti-Patterns to Avoid

### One-Size-Fits-All Configuration

❌ **Don't:** Force company-wide rules that ignore project differences  
✅ **Do:** Support hierarchical config: company → team → project

### Silent Violations

❌ **Don't:** Catch convention violations but don't report them  
✅ **Do:** Make violations visible with clear remediation paths

### Config Overload

❌ **Don't:** 200 rules by default that overwhelm new users  
✅ **Do:** Sensible defaults, add rules as team grows

### One-Way Enforcement

❌ **Don't:** Block developers without explanation  
✅ **Do:** Explain *why* a rule exists, link to rationale

### Static Documentation

❌ **Don't:** Conventions docs that never update  
✅ **Do:** Living docs that evolve with the codebase

### Hidden State

❌ **Don't:** Runie config that lives only locally  
✅ **Do:** Config in repo, versioned with code

### Blanket Blocking

❌ **Don't:** Hard errors that prevent any work  
✅ **Do:** Configurable severity levels, escalation paths

---

## 11. Success Metrics for This Persona

### Team-Level Metrics

| Metric | Target | Measurement |
|--------|--------|------------|
| Convention adherence | >90% | % of PRs passing all rules |
| Onboarding velocity | <4 weeks | Time to first PR without convention failures |
| Review time reduction | >30% | Senior hours on mechanical review |
| Tool adoption | >80% | % of team using Runie conventions |

### Quality Metrics

| Metric | Target | Measurement |
|--------|--------|------------|
| Bug reduction | >20% | Convention-related bugs in prod |
| Tech debt | Declining | Convention violations over time |
| Documentation | >80% | Public APIs with doc comments |

### Leadership Metrics

| Metric | Target | Measurement |
|--------|--------|------------|
| Knowledge transfer | Complete | Senior patterns encoded in config |
| Escalations | Declining | Convention questions to tech lead |
| Consistency | Improving | Code style variance across team |

### Behavioral Indicators

Positive signals:
- Team members cite Runie as learning resource
- Convention discussion in PRs declining
- New developers contributing convention improvements
- Senior devs using saved review time for architecture

Warning signals:
- Runie config diverging from actual team practice
- Developers working around Runie instead of with it
- Exception requests for fundamental rules
- Adoption declining over time

---

## Appendix: Research Sources

| Finding | Source |
|---------|--------|
| Senior expertise scaling | Unix Philosophy - Section 1.1, 1.2 |
| Configuration via text files | Unix Philosophy - Section 3.4 |
| Progressive disclosure | Cognitive Load UX - Section 4 |
| Consistent quality > occasional brilliance | Coding Agent UX - Section 8 |
| 30-50% faster with progressive interfaces | Cognitive Load UX - Section 3 |
| Knowledge transfer as multiplier | Team Lead Persona Analysis |
| Configurable strictness | TUI Best Practices - Section 6 |

---

*Document Version: 1.0*  
*Last Updated: 2026-07-15*  
*Research Foundation: coding_agents_ux.md, unix_philosophy.md, tui_best_practices.md, cognitive_load_ux.md*
