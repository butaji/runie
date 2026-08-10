# Persona Analysis: The Junior Developer

**Persona Type:** Primary End User  
**Confidence Level:** Learning, Building Foundation  
**Risk Profile:** High — learning habits, forming tool preferences, building confidence

---

## Executive Summary

The Junior Developer is a recent graduate or 1-3 year professional who is actively learning programming concepts while contributing to real projects. They are eager but uncertain, capable but not confident. Their relationship with AI coding tools is complex: they need help but fear becoming dependent, they want speed but need understanding, they trust AI suggestions but verify everything. Runie must be their patient mentor, not their replacement brain.

---

## 1. Persona Profile

### Background

| Attribute | Description |
|-----------|-------------|
| **Experience** | 0-3 years professional experience, or bootcamp/college graduate within last 2 years |
| **Education** | Computer science degree, coding bootcamp, or self-taught with portfolio projects |
| **Technical Foundation** | Understands basic data structures, knows one language well, learning others, familiar with Git basics |
| **Current Role** | Individual contributor, likely in a team of mixed experience levels |
| **Work Style** | Explorer with guidance; prefers structured tasks with clear feedback loops |

### Expertise Level

**Current State:**
- Can read and understand existing code with effort
- Writes working code but may miss edge cases
- Familiar with debugging basics but not expert
- Learning design patterns and architectural thinking
- Often uncertain about "best practices" vs. "what works"

**Growth Trajectory:**
- Rapid skill acquisition in first 2 years
- Building mental models of how systems work
- Forming lasting opinions about tools and workflows
- Establishing habits that will persist throughout career

### Work Environment

- **Primary:** VS Code or JetBrains IDE
- **Secondary:** Browser documentation, Stack Overflow
- **Context:** Often working on features in unfamiliar codebases
- **Pressure:** Deadlines create stress; senior review creates anxiety about quality

---

## 2. Goals and Motivations

### Primary Goals

1. **Learn and Understand** — Primary motivation. They want to know *why* code works, not just *that* it works.

2. **Ship Working Code** — Contribute to projects without breaking existing functionality. Success = code that passes review.

3. **Build Confidence** — Move from "I think this might work" to "I know this is correct." Every successful task builds self-efficacy.

4. **Grow Efficiently** — Learn more in less time. They're competing with more experienced devs and feel constant pressure to level up.

5. **Avoid Looking Foolish** — Hidden goal that affects tool choices. They'd rather ask Runie than a colleague when stuck.

### Motivational Drivers

| Driver | Expression |
|--------|------------|
| **Curiosity** | "How does this framework handle async?" |
| **Mastery** | "I want to understand closures deeply" |
| **Competence** | "I want to write code that impresses my senior dev" |
| **Autonomy** | "I want to solve this myself, then verify" |
| **Belonging** | "I want to contribute meaningfully to my team" |

---

## 3. Pain Points with Current Tools

### The Trust Paradox

> "I use AI coding tools constantly, but I don't really trust them. I have to verify everything anyway, so sometimes I wonder if I'm saving any time at all."

**Problem:** 46% of developers distrust AI accuracy (up from 31% last year). Junior developers are hit hardest because they often can't identify when AI is wrong.

**Source:** [Coding Agent UX Research - Section 2.3]

### The "Almost Right" Trap

> "The AI gave me a solution that looks perfect, but there's a subtle bug that took me 2 hours to find. I would have been faster writing it myself."

**Statistics:**
- 66% cite "almost right" solutions as their #1 frustration
- 45% say debugging AI-generated code takes longer than writing it themselves

**Why it hurts juniors more:** Experienced devs can spot AI hallucinations faster. Juniors often accept plausible-sounding wrong answers.

**Source:** [Coding Agent UX Research - Section 2.1, 2.2]

### Context Window Confusion

> "I don't know what's in the AI's context. Did it see my other file? Did it read my test file? Why is it suggesting code that contradicts what I just wrote?"

**Problem:** Juniors lack the mental model to understand what AI can and can't see. They make mistakes assuming shared context.

**Source:** [Coding Agent UX Research - Section 2.6]

### The Explanation Deficit

> "It gave me code but didn't explain *why* this approach is better. I just copy-pasted it and moved on without learning anything."

**Problem:** AI tools optimize for output, not learning. Juniors miss the teaching moment because explanation takes longer than generation.

**Key Finding:** Code *understanding* (71.9%) is the #1 use case, not code *generation* (55.6%). Yet most tools focus on generation.

**Source:** [Coding Agent UX Research - Section 1.2]

### Tool Overwhelm

> "Should I use Claude Code or Cursor? GitHub Copilot or Continue? Each one does things slightly differently and I'm already overwhelmed."

**Problem:** The average knowledge worker uses 9.4 applications daily. Juniors lack the experience to evaluate competing tools and end up with fragmented workflows.

**Source:** [Cognitive Load UX - Section 8]

### Imposter Syndrome Amplification

> "I already feel like I don't belong. When AI gives me code I don't understand, I feel even more like a fraud."

**Problem:** 20% of developers feel less confident in problem-solving due to AI tools. This effect is amplified for juniors who are already building confidence.

**Source:** [Coding Agent UX Research - Section 5.4]

### Verification Burden

> "I still don't have enough confidence to blindly trust the responses. I spend as much time checking AI output as I would have spent writing it."

**Problem:** The verification burden undermines the time-saving benefit. For juniors, this is especially true because verification requires understanding they may not have.

**Source:** [Coding Agent UX Research - Section 5.2]

---

## 4. What Would Delight This User

### Uncovering the "Why"

> "Wait, so Runie didn't just give me the code—it explained *why* this SQL query is faster? That's the first time an AI tool actually taught me something."

**Delight Trigger:** Tool as teacher, not just code generator. Explaining trade-offs, not just presenting solutions.

**How Runie Delivers:**
- Natural language explanations of generated code
- "This approach is better because..." not just "Here's the code"
- Links to relevant documentation sections
- Reasoning visible before action taken

### Getting Unstuck Without Shame

> "I could have asked my senior dev for help, but I didn't want to bother them for something basic. Runie helped me figure it out myself."

**Delight Trigger:** Help that builds confidence, not dependency. "I solved it" feeling, not "the tool solved it."

**How Runie Delivers:**
- Socratic prompting: "What have you tried?" before answers
- Progressive hints rather than immediate solutions
- Celebrating small victories, not just final outputs

### Safe Experimentation

> "I was afraid to refactor that module because I didn't understand all the dependencies. Runie showed me exactly what would break before I changed anything."

**Delight Trigger:** Bold action with visible safety net. "What if I tried...?" answered with clarity.

**How Runie Delivers:**
- Clear preview of changes before application
- Impact analysis: "This change affects X files"
- One-click rollback when experiments fail
- No penalty for trying

### Learning Alongside

> "I didn't just get code—I got code with a mini-lesson on the pattern I was using wrong."

**Delight Trigger:** Every session is a micro-learning opportunity. Knowledge accumulates.

**How Runie Delivers:**
- Contextual explanations tied to specific code
- Suggestion to "explain this pattern to me" always available
- Progress tracking on concepts learned
- "You've asked about async/await 5 times—here's a comprehensive guide"

### Transparent Limitations

> "Instead of failing silently or giving wrong answers, Runie told me exactly what it couldn't see in my codebase. That honesty made me trust it more."

**Delight Trigger:** Tools that know their limits and communicate them clearly.

**How Runie Delivers:**
- Explicit "I don't have context for..." messages
- Rate limits visible before hitting them
- Clear signaling when outside knowledge cutoff
- "I'm about to do something that needs your approval"

---

## 5. Specific UI/UX Recommendations for Runie

### 5.1 Progressive Disclosure Architecture

**Principle from Research:**
> "30-50% faster task completion can be achieved with progressive interfaces versus full-exposure alternatives."
> — [Cognitive Load UX - Section 3]

**Implementation for Juniors:**

```
┌─────────────────────────────────────────────────────────────┐
│  INITIAL VIEW (Minimal)                                      │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                                                          ││
│  │  What would you like help with?                         ││
│  │  ▌                                                       ││
│  │                                                          ││
│  └─────────────────────────────────────────────────────────┘│
│  [Explain this code] [Help me write] [Debug this] [Learn]    │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  ON "EXPLAIN THIS CODE" (Progressive Detail)                │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                                                          ││
│  │  This function handles user authentication...            ││
│  │                                                          ││
│  │  [Show me more details ▼]                                ││
│  │                                                          ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│  ON "SHOW ME MORE DETAILS" (Full Disclosure)                 │
│  ┌─────────────────────────────────────────────────────────┐│
│  │                                                          ││
│  │  Line-by-line breakdown:                                 ││
│  │  Line 12: bcrypt.compare() validates password hash        ││
│  │  Line 15: JWT signed with secret from environment        ││
│  │  Line 23: Token expires in 24 hours (configurable)       ││
│  │                                                          ││
│  │  Related concepts:                                       ││
│  │  • JWT authentication flow                               ││
│  │  • Password hashing best practices                      ││
│  │                                                          ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

**Key Guidelines:**
- Start with one sentence summaries
- Always show "There's more" affordances
- Let users control depth, don't force it
- Remember depth preferences for future sessions

### 5.2 Verification-Friendly Diff Display

**Principle from Research:**
> "Show, don't tell — Diff-first, visible changes before applying."
> — [Coding Agent UX Research - Section 8]

**Junior-Specific Design:**

```
┌─────────────────────────────────────────────────────────────┐
│  REVIEW CHANGES BEFORE APPLYING                              │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  auth/login.rs                          │    auth/login.rs   │
│  ─────────────────────────────────────  │  ─────────────────│
│  1  fn login(user: &str, pass: &str) { │  1  fn login(...)  │
│  2      // Basic validation            │  2  + hash password │
│  3      let hash = bcrypt(pass)?;      │  3  + validate JWT │
│  4      let token = jwt::encode(...)?; │  4  + error handle │
│  5      Ok(token)                      │  5  + Ok(token)   │
│                                        │                    │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Changed 3 lines • Added 8 lines • 0 deleted               │
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  What this does: Adds SHA-256 hashing before JWT      ││
│  │  Why: Prevents timing attacks on password validation   ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  [Apply Changes]  [Edit Before Apply]  [Ask About Changes] │
└─────────────────────────────────────────────────────────────┘
```

**Junior Features:**
- **"What does this change?"** always visible
- **"Why this approach?"** expandable section
- **"What could go wrong?"** risks highlighted
- **"Undo"** always available after apply
- **Color coding:** Green = new concepts to learn, Yellow = review carefully, Red = significant change

### 5.3 Context Visibility Panel

**Problem:** Juniors don't know what AI can see.

**Solution:** Always-visible context indicator

```
┌─────────────────────────────────────────────────────────────┐
│  CONTEXT                            [Configure Context ▼]   │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  ✓ auth/login.rs         ✓ auth/mod.rs        ✓ Cargo.toml   │
│  ✓ tests/auth_tests.rs   ✓ .env.example      ✓ README.md    │
│                                                             │
│  📁 Currently viewing: src/handlers/user.rs (line 45)      │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│  [+] Add file to context  [~] Refresh context  [?] Help    │
└─────────────────────────────────────────────────────────────┘
```

**Benefits:**
- Always know what Runie sees
- Easy to add missing files
- Visual confirmation of context state
- Reduces "why didn't it see my changes?" questions

### 5.4 Learning Mode Toggle

**Principle from Research:**
> "Users want to learn alongside code generation, not just get answers."
> — [Coding Agent UX Research - Section 1.2]

**Implementation:**

```
┌─────────────────────────────────────────────────────────────┐
│  MODE: [● Learning ○ Balanced ○ Speed]                      │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Learning mode provides:                                    │
│  • More detailed explanations                               │
│  • Links to documentation                                   │
│  • Alternative approaches shown                            │
│  • "Why not this instead?" prompts                         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**Behavior Differences by Mode:**

| Feature | Learning | Balanced | Speed |
|---------|----------|----------|-------|
| Code explanations | Extended | Summary | None |
| Alternative approaches | Shown | Optional | Hidden |
| Documentation links | Always | On request | Hidden |
| Confirmation dialogs | More | Standard | Fewer |
| Response time | Slower | Normal | Fastest |

### 5.5 Confidence Indicator

**Principle from Research:**
> "Only 3% of developers 'highly trust' AI outputs. Developers spend more time reviewing AI code than writing it."
> — [Coding Agent UX Research - Section 2.3]

**Solution:** Make confidence visible so juniors know what to scrutinize

```
┌─────────────────────────────────────────────────────────────┐
│  ┌─────────────────────────────────────────────────────────┐│
│  │  HIGH CONFIDENCE (green)                                ││
│  │  "This is idiomatic Rust for error handling. Well-      ││
│  │   tested pattern used in 50,000+ crates."             ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  MEDIUM CONFIDENCE (yellow)                             ││
│  │  "This SQL query should work, but edge cases around    ││
│  │   NULL values depend on your database config."         ││
│  └─────────────────────────────────────────────────────────┘│
│                                                             │
│  ┌─────────────────────────────────────────────────────────┐│
│  │  LOW CONFIDENCE (red)                                   ││
│  │  "I'm not certain about the React hook order here.     ││
│  │   Please review carefully."                            ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

**Benefits:**
- Junior knows what to focus review on
- Reduced anxiety from uncertainty
- Gradual trust building with evidence

### 5.6 Socratic Help Mode

**Principle from Research:**
> "Help that builds confidence, not dependency."
> — [Cognitive Load UX - Section 6]

**Implementation:**

```
┌─────────────────────────────────────────────────────────────┐
│  You asked: "How do I handle errors in async Rust?"        │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  Let's think through this together.                       │
│                                                             │
│  What have you tried so far?                              │
│  ▌ ________________________________________________        │
│                                                             │
│  ─────────────────────────────────────────────────────────  │
│  [I don't know where to start]                            │
│  [Show me a hint]  [Show me similar code in this repo]     │
└─────────────────────────────────────────────────────────────┘
```

**Progression:**
1. Ask what they've tried
2. Offer hints before solutions
3. Show similar patterns in their codebase
4. Provide solution with full explanation
5. Ask if they want to understand the pattern deeper

---

## 6. Default Behaviors That Would Impress Them

### 6.1 Explain First, Generate Second

**Default Behavior:**
When a junior asks for code, Runie should respond with:

1. **Understanding check** — "I see you're trying to [summarize understanding]. Is that right?"
2. **Approach explanation** — "I'll solve this by [high-level approach]"
3. **Code with annotations** — Comments explaining *why* each part exists
4. **Learning prompt** — "Want me to explain any part of this?"

**Why It Impresses:**
Addresses the #1 AI tool complaint: "I just copy-pasted without understanding."

### 6.2 Conservative Changes by Default

**Default Behavior:**
- Smaller, safer changes first
- "I could refactor all 50 files, but let's start with this one to test the approach"
- Visible impact analysis before any action
- Always show what *didn't* change

**Why It Impresses:**
Builds trust through transparency. Juniors feel safe experimenting.

### 6.3 Anticipate Edge Cases

**Default Behavior:**
```
"I've written the code, but here are 3 edge cases to verify:
1. What should happen if the user is already logged in?
2. Should we handle rate limiting here or at the gateway?
3. This assumes the database is reachable—want me to add retry logic?"
```

**Why It Impresses:**
Shows thinking beyond the immediate problem. Models how seniors approach code review.

### 6.4 Remember Context Across Sessions

**Default Behavior:**
- Recall previous questions about similar code
- "Last time we worked on auth, you asked about JWT validation. Want me to apply that same pattern here?"
- Build a visible "things I've learned" list

**Why It Impresses:**
Demonstrates Runie is a learning partner, not just a tool.

### 6.5 Graceful Uncertainty

**Default Behavior:**
```
"I'm not certain about the best approach for this React state management.
Here are two options with trade-offs:
Option A: useState (simpler, good for local state)
Option B: useReducer (better for complex state logic)
Which approach fits your current needs?"
```

**Why It Impresses:**
Models expert decision-making. Junior learns how to evaluate options.

---

## 7. Learning Curve Expectations

### Phase 1: First 10 Minutes (Onboarding)

**Expectation:** "I want to get started without reading documentation."

**Runie Response:**
- Single welcome message explaining purpose
- One clear call to action: "What would you like help with?"
- Inline hints for keyboard shortcuts
- Optional quick tour (skippable)

**Success Metric:** Junior can ask their first question within 2 minutes of opening Runie.

### Phase 2: First Day

**Expectation:** "I want to understand what Runie can and can't do."

**Runie Response:**
- Contextual suggestions: "I see you're working on [file type]. Here's how Runie can help with [relevant features]."
- Progressive discovery of features
- No overwhelming options upfront

**Success Metric:** Junior has used 2-3 features and understands when to use Runie vs. other tools.

### Phase 3: First Week

**Expectation:** "I want to use Runie efficiently without thinking about it."

**Runie Response:**
- Keyboard shortcuts become natural
- Common workflows feel intuitive
- Unclear moments trigger helpful prompts
- Confidence grows through successful interactions

**Success Metric:** Junior reaches for Runie for appropriate tasks without prompting.

### Phase 4: First Month

**Expectation:** "I want to customize Runie to my workflow."

**Runie Response:**
- Advanced features available but not required
- Personalization options introduced contextually
- Continued learning prompts

**Success Metric:** Junior has opinion about Runie preferences and can articulate what they like.

---

## 8. Guidance and Help System Requirements

### 8.1 Help Hierarchy (Three Levels)

**Level 1: Inline Context Hints**
```
┌─────────────────────────────────────────────────────────────┐
│  ↑↓ Navigate │ Enter Select │ Esc Back │ ? Help             │
└─────────────────────────────────────────────────────────────┘
```
- Always visible at bottom of screen
- Context-sensitive: changes based on current mode
- Minimal: 5-7 actions maximum
- Scannable: action + description format

**Level 2: On-Demand Quick Help**
```
┌─────────────────────────────────────────────────────────────┐
│  QUICK HELP (press ?)                                       │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  NAVIGATION                                                 │
│  j/k       Move down/up                                    │
│  gg/G      Jump to top/bottom                              │
│  /         Search forward                                  │
│                                                             │
│  ACTIONS                                                    │
│  Enter     Select / Confirm                                │
│  Space     Toggle selection                                 │
│                                                             │
│  MODES                                                     │
│  e         Enter explanation mode                         │
│  g         Enter generation mode                          │
│  l         Enter learning mode                            │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```
- Triggered by `?` or `F1`
- Shows only keys relevant to current context
- Grouped by category
- Short descriptions, not tutorials

**Level 3: Full Documentation**

```bash
# Command line help
runie help

# Topic-specific help
runie help explain
runie help generate
runie help config
```

- Comprehensive man-page style
- Examples for every command
- Conceptual explanations
- Searchable

### 8.2 Junior-Specific Help Features

**"I'm Stuck" Flow**
```
┌─────────────────────────────────────────────────────────────┐
│  🔒 STUCK DETECTED                                          │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  You've been on this screen for a while. Want help?        │
│                                                             │
│  [Show me hints]          [I want to try myself]           │
│  [Explain this section]   [Start over]                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

**"What does this mean?" Tooltip**
```
┌─────────────────────────────────────────────────────────────┐
│  [JWT Token] ──────────────────────────────────────── [?]  │
│  │                                                       │ │
│  │  A JSON Web Token is an open standard for...         │ │
│  │                                                       │ │
│  │  Learn more: [Link to documentation]                 │ │
│  │  See example: [Show in codebase]                     │ │
│  └───────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

**Error Messages That Teach**

❌ **Bad:** "Invalid token"
❌ **Worse:** "Error: authentication failure at module::auth::validate_token line 145"
✅ **Good:**
```
"This error means the JWT token couldn't be verified.

Common causes:
1. Token has expired (check the 'exp' claim)
2. Secret key mismatch (verify JWT_SECRET in .env)
3. Token was modified after signing

Your token's 'exp' claim shows: 2024-01-15T10:30:00Z
Current time: 2024-01-15T14:45:00Z

The token expired 4 hours ago. You'll need to log in again."
```

### 8.3 Learning Resources Integration

**Contextual Learning Prompts**
- After generating code: "Want me to explain how this works?"
- After errors: "This is a common mistake. Want to learn more?"
- After patterns: "You've used this pattern 3 times. Here's a deeper dive."

**External Resources**
```
┌─────────────────────────────────────────────────────────────┐
│  RELATED LEARNING                                           │
│  ─────────────────────────────────────────────────────────  │
│                                                             │
│  📚 Rust Book: Error Handling                               │
│     https://doc.rust-lang.org/book/ch09-00-error-handling/  │
│                                                             │
│  🎥 Video: Understanding Result types in Rust               │
│     https://rust-lang.org/videos/result-types               │
│                                                             │
│  📁 Examples in your codebase:                              │
│     src/utils/errors.rs                                     │
│     tests/test_error_handling.rs                           │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## 9. How Runie Can Exceed Expectations (Wow Factors)

### Wow Factor 1: The Code Mentor Effect

> "I've been using Runie for a month, and I realized I don't just use it to write code—I use it to *understand* code. Last week my senior dev explained something during review, and I already knew it because Runie had taught me. That's when I knew Runie was different."

**What Makes It Happen:**
- Explanations that connect to concepts, not just code
- "Why" before "what"
- Consistent educational tone
- Building on previous learning

**How to Measure:**
Track repeat questions about the same concept. If frequency decreases, learning is happening.

### Wow Factor 2: The Confidence Builder

> "Before Runie, I'd spend 30 minutes stuck before asking for help. Now I use Runie and figure it out myself. My PR count is up, and my questions to the team chat are down. I feel like a real developer."

**What Makes It Happen:**
- Socratic prompting that guides, not gives
- Safe experimentation with visible previews
- Success feedback that celebrates effort, not just output
- No judgment for "basic" questions

**How to Measure:**
Survey confidence levels over time. Track ratio of independent resolutions vs. escalations.

### Wow Factor 3: The Time Machine

> "I asked Runie to show me all the places where we're doing password handling wrong. It found 5 instances across the codebase that I'd never have found manually. That's 2 hours of security work in 2 minutes."

**What Makes It Happen:**
- Proactive pattern detection
- Cross-file analysis
- "What could go wrong?" over "What do you want?"
- Security and best-practice scanning

**How to Measure:**
Track "discovered issues" vs. "reported bugs" ratio.

### Wow Factor 4: The Translation Layer

> "I came from Python and everything in Rust felt foreign. Runie doesn't just translate my Python code to Rust—it explains why Rust does it differently. Now I actually understand ownership."

**What Makes It Happen:**
- Cross-language explanations
- Concept mapping between ecosystems
- "Here's the Rust equivalent of [Python concept] and why it's designed differently"
- Bridge building from existing knowledge

**How to Measure:**
Track satisfaction scores for developers new to a language.

### Wow Factor 5: The Team Voice

> "Runie knows our codebase conventions. When I write code that violates our patterns, it tells me in our team's voice, with references to our docs. It's like having a mentor who never gets tired of explaining."

**What Makes It Happen:**
- Project-specific context awareness
- Convention documentation parsing
- "In this project, we typically..."
- Links to project documentation

**How to Measure:**
Survey "team fit" perception over time.

---

## 10. Anti-Patterns to Avoid

### Forgetting They're Learning

❌ **Don't:** Provide solutions without context
✅ **Do:** Explain every suggestion

### Assuming Too Much

❌ **Don't:** "As you know, Result<T, E> handles..."
✅ **Do:** Check knowledge level before diving deep

### Overwhelming with Options

❌ **Don't:** Show 10 alternative approaches upfront
✅ **Do:** Start with best guess, offer alternatives on request

### Silent Failures

❌ **Don't:** Timeout with no explanation
✅ **Do:** "This is taking longer than expected. Here's why..."

### Confidence Mismatches

❌ **Don't:** Present uncertain suggestions as facts
✅ **Do:** Clearly indicate confidence level

### Forgetting the Human

❌ **Don't:** Pure code generation without acknowledgment
✅ **Do:** "This is tricky! Here's how I'd approach it..."

---

## 11. Success Metrics for This Persona

### Quantitative

| Metric | Target | Measurement |
|--------|--------|-------------|
| Task completion rate | >85% | % of questions resulting in useful output |
| Time to first success | <5 min | Time from onboarding to first helpful response |
| Feature discovery rate | 3+ features/day | New features used in first week |
| Return rate | >70% | % returning after first session |

### Qualitative

| Dimension | Questions to Ask |
|-----------|------------------|
| **Learning** | "What did you learn today?" |
| **Confidence** | "How do you feel about your coding skills now vs. before?" |
| **Trust** | "Would you trust Runie's suggestions without review?" |
| **Value** | "What's the one thing you couldn't do without Runie?" |
| **Comparison** | "How does Runie compare to [other tools]?" |

### Behavior Indicators

Positive signals:
- Asking "why" questions instead of "how" questions
- Voluntarily using advanced features
- Recommending Runie to peers
- Reduced escalation to human help

Warning signals:
- Blind acceptance of suggestions
- Always asking for "the code" without wanting explanation
- Avoidance of Runie despite inability to solve problems
- Impatient responses ("just give me the code")

---

## Appendix: Research Sources

| Finding | Source |
|---------|--------|
| 71.9% use AI for code understanding | IBM Case Study, 2025 |
| 66% frustrated by "almost right" solutions | Stack Overflow Survey, 2025 |
| 46% distrust AI accuracy | Stack Overflow Survey, 2025 |
| 30-50% faster with progressive disclosure | Number Analytics |
| 4-7 chunks working memory limit | Cognitive Load Theory (Sweller) |
| 23 min context switch recovery | UC Irvine Research |
| "Explain code" > "Generate code" | IBM watsonx Study |

---

*Document Version: 1.0*  
*Last Updated: 2026-07-15*  
*Research Foundation: coding_agents_ux.md, unix_philosophy.md, tui_best_practices.md, cognitive_load_ux.md*
