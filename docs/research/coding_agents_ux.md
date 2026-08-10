# Coding Agent UX Research Report

*Research compiled from user feedback, surveys, and academic studies on AI coding assistants*

## Executive Summary

This report synthesizes findings from multiple sources including the 2025 Stack Overflow Developer Survey (49,000+ respondents), IBM's case study on watsonx Code Assistant (669 users), academic research, and community discussions to understand what users appreciate and frustrates about AI coding tools.

**Key Insight:** 84% of developers now use AI tools, but trust is declining (46% distrust accuracy vs 33% who trust). The biggest pain point: AI solutions that are "almost right" (66%), leading to debugging that takes longer than writing code manually (45%).

---

## 1. What Users Appreciate Most

### 1.1 Speed and Productivity Gains
- **30-75% time savings** on coding, testing, and documentation tasks
- **52% of developers** report positive productivity impact from AI tools/agents
- 70% report reduced time on specific development tasks
- Quick answers to programming questions without leaving the editor

### 1.2 Code Understanding and Learning
- **Top use case is understanding code** (71.9%), not generating it (55.6%)
  - Explaining functions and unfamiliar code
  - Answering general programming questions (68.5%)
  - Exploring new codebases without disturbing colleagues
- "I like its ability to explain functions of code which could take a bit to understand. It can save a lot of time."
- Helping recall forgotten concepts and learning new technologies

### 1.3 Boilerplate and Routine Tasks
- Autocomplete for repetitive patterns
- Unit test generation (35.7% use this feature)
- Documentation generation (39.6%)
- Translation between languages
- Code refactoring suggestions

### 1.4 Exploration and Ideation
- Discovering alternative approaches to problems
- "It gives me new ideas"
- "Recommends approaches I haven't thought of"
- Brainstorming when unsure how to proceed ("exploration mode")

### 1.5 Seamless IDE Integration
- Tools that feel native to the editor
- Inline diffs showing changes before accepting
- Quick access via keyboard shortcuts
- Minimal context switching required

---

## 2. What Frustrates Users Most

### 2.1 "Almost Right" Solutions
**The #1 frustration (66%):** AI solutions that are close but not quite correct.
- Forces developers to mentally parse what needs fixing anyway
- Often faster to write from scratch than debug AI output
- Requires understanding the problem to verify the solution

### 2.2 Debugging AI-Generated Code
**Second biggest frustration (45%):** Debugging takes longer than writing code manually.
- "You have to spend time to verify it"
- "It is a burden to have to double check answers"
- Errors can be subtle and hard to catch

### 2.3 Lack of Trust and Verification Burden
- **46% distrust AI accuracy** (up from 31% last year)
- Only **3% "highly trust"** AI outputs
- Experienced developers most cautious: only 2.6% highly trust
- Developers spend more time reviewing AI code than writing it

### 2.4 Transparency Issues
- **"Black box" problem:** Hard to understand why AI generated specific code
- Claude Code's `--verbose` flag described as "firehose of debug output"
- Cursor's usage limits hidden until you hit them
- Opaque rate limits causing silent failures

### 2.5 Context Window Limitations
- AI can't grasp large codebases
- Suggestions break architectural patterns
- Multi-file changes require careful orchestration
- Performance degrades on large monorepos

### 2.6 Security and Privacy Concerns
- **87% concerned about accuracy** of AI agents
- **81% concerned about security/privacy**
- Risk of reproducing copyrighted code
- Hardcoded secrets in AI suggestions
- SQL injection vulnerabilities in generated queries

---

## 3. Delightful vs Painful Experiences

### 3.1 Delightful Patterns

| Pattern | Description |
|---------|-------------|
| **Immediate feedback** | Inline suggestions appear as you type, no waiting |
| **Visible diffs** | See exactly what changed before accepting |
| **Respectful interruption** | Tool pauses when you start typing, resumes after |
| **Clear explanations** | Why did it suggest this? Can it explain unfamiliar code? |
| **Consistent quality** | Reliable enough for routine tasks |
| **Good autocomplete timing** | Appears when helpful, not intrusive |
| **Learning alongside** | Helps understand code, not just generate it |

### 3.2 Painful Patterns

| Pattern | Description |
|---------|-------------|
| **Wrong but confident** | Plausible code that doesn't work |
| **Silent failures** | Hit rate limit, session dies with no warning |
| **Breaking flow** | Autocomplete interrupts typing mid-thought |
| **Magic behavior** | Changes files without showing what/why |
| **Context loss** | Forgets conversation, repeats previous mistakes |
| **Inconsistent quality** | Great one moment, useless the next |
| **Privacy black box** | Sending code somewhere, unclear what happens |

---

## 4. Most Appreciated Default Behaviors

### 4.1 Autocomplete Behavior
- Suggestions appear **after a brief pause**, not on every keystroke
- Grayed-out or distinct styling from typed code
- Easy to dismiss (Tab to accept, keep typing to dismiss)
- Respects cursor position and indentation

### 4.2 Control and Transparency
- **Diff-first approach:** Show changes, don't auto-apply
- Clear indicators of AI-generated vs. human code
- Ability to undo AI actions
- Permission dialogs for destructive operations

### 4.3 Context Awareness
- Understands current file and project structure
- Doesn't suggest code that conflicts with existing patterns
- Respects `.gitignore` and project conventions
- Good defaults that "just work" without configuration

### 4.4 Agency Calibration
- Agent mode should be **opt-in or clearly indicated**
- Power users want autonomy; newcomers want guardrails
- Adjustable "autonomy slider" is valued (as in Cursor)
- Clear feedback on what the agent is doing

### 4.5 Privacy Controls
- Enterprise-grade privacy options
- Clear data handling policies
- Ability to use offline or local models
- Transparent about when code is sent to servers

---

## 5. Cognitive Load Issues

### 5.1 The Productivity Paradox
Research shows context switching can require **23 minutes** to regain full focus, with developers losing **20% cognitive capacity** per interruption.

AI tools can either:
- **Reduce cognitive load:** By handling routine tasks
- **Increase cognitive load:** By requiring constant verification

### 5.2 Verification Burden
- Developers must understand problems to verify AI solutions
- This undermines the "time savings" benefit
- "I still don't have enough confidence to blindly trust the responses"
- Increased cognitive load from double-checking everything

### 5.3 Mental Model Mismatch
- AI behavior doesn't match developer expectations
- "Why did it suggest that?" - lack of explanation
- Hard to predict what the AI will do next
- Uncertainty about context window contents

### 5.4 Deskilling Concerns
- **20% of developers** feel less confident in their problem-solving
- "Makes people lazy" / "promotes not to think"
- Risk of over-reliance on AI
- Loss of deep understanding when AI handles details

### 5.5 Split Attention
- Autocomplete suggestions distract from writing code
- Chat panels take focus from editor
- Terminal vs. IDE interface creates context split
- Multi-tool workflows (Claude Code + Cursor) require mental state management

### 5.6 Accountability Ambiguity
- Who is responsible for AI-generated code?
- "If it doesn't have close to 100% correctness, I cannot trust it"
- Legal/IP concerns about generated code
- Shared authorship creates attribution challenges

---

## 6. Key UX Findings by Tool

### 6.1 Claude Code
**Strengths:**
- Exceptional context depth (200K+ tokens)
- Best for complex, multi-file refactoring
- Terminal-first appeals to power users
- Strong reasoning about trade-offs

**Weaknesses:**
- Opaque rate limits (silent failures)
- Terminal interface lacks visual feedback
- Verbose mode is overwhelming, not helpful
- Less IDE-native than alternatives

### 6.2 Cursor
**Strengths:**
- Best IDE integration ("AI-first editor")
- Inline diffs before accepting changes
- Visual feedback throughout
- Good for VS Code users

**Weaknesses:**
- Context retrieval degrades on large codebases
- Free tier has usage restrictions
- Some features behind paywall

### 6.3 GitHub Copilot
**Strengths:**
- Best enterprise integration (GitHub, Azure, Microsoft)
- Widest IDE support (VS Code, JetBrains, Visual Studio)
- Best privacy controls for enterprise
- Generous free tier

**Weaknesses:**
- Weaker reasoning for complex tasks
- Less sophisticated context handling
- Feels like "autocomplete" not "agent"

---

## 7. User Sentiment Trends (2023-2025)

| Metric | 2023 | 2024 | 2025 |
|--------|------|------|------|
| Favorable sentiment | 70%+ | 70%+ | **60%** |
| Daily AI users | ~35% | ~43% | **51%** |
| Trust AI accuracy | ~38% | ~38% | **33%** |
| Distrust AI accuracy | ~31% | ~31% | **46%** |

**Key trend:** Adoption up, sentiment down. Users are using AI more but trusting it less.

---

## 8. Recommendations for Runie

Based on this research, users appreciate coding tools that:

1. **Prioritize code understanding over generation** - This was the #1 use case in the IBM study
2. **Show, don't tell** - Diff-first, visible changes before applying
3. **Respect developer flow** - Non-intrusive suggestions, resume after interruption
4. **Be transparent about limitations** - Clear rate limits, no silent failures
5. **Provide good defaults** - Work out of the box without configuration
6. **Support verification** - Help users understand what was generated and why
7. **Calibrate agency appropriately** - Let users control how autonomous the tool is
8. **Minimize context switching** - Keep the developer in their flow state
9. **Enable hybrid workflows** - Users often combine tools (Claude Code + Cursor)
10. **Build trust through reliability** - Consistent quality > occasional brilliance

---

## Sources

- [2025 Stack Overflow Developer Survey - AI Section](https://survey.stackoverflow.co/2025/ai)
- [IBM Case Study: AI Code Assistant Impact on Productivity (arXiv)](https://arxiv.org/html/2412.06603v2)
- [Claude Code vs Cursor vs Copilot: Honest 2026 Comparison](https://www.artifilog.com/posts/claude-code-vs-cursor-vs-copilot)
- [AI Coding Assistants Statistics & Trends 2025](https://www.secondtalent.com/resources/ai-coding-assistant-statistics/)
- [ShiftMag: Stack Overflow Survey Analysis](https://shiftmag.dev/stack-overflow-survey-2025-ai-5653/)
- [Cursor vs GitHub Copilot 2026 Comparison](https://misar.io/blogs/cursor-vs-github-copilot-2026)
- [AI Agent Security Risks (Kaspersky)](https://www.kaspersky.com/blog/vibe-coding-2025-risks/54584/)
- [Understanding Security Risks in AI-Generated Code (CSA)](https://cloudsecurityalliance.org/blog/2025/07/09/understanding-security-risks-in-ai-generated-code)
