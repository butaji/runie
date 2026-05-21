# Charm.sh Ecosystem Pattern Research

**Date:** 2026-05-21
**Source:** charmbracelet/{lipgloss, bubbles, bubbletea, glamour, harmonica}
**Purpose:** Concrete API patterns and design decisions for Tidy adaptation

---

## 1. Lip Gloss: Fluent Styling API

### Core Philosophy
- **Value types, not pointers**: `Style` is a pure value type. Every method returns a new copy. Assignment creates true copies.
- **CSS-like shorthand**: `Padding(1, 4, 2)` follows CSS clockwise/shortand rules.
- **Fluent chaining**: All setters chain: `lipgloss.NewStyle().Bold(true).Foreground(c).Padding(2)`

### Borders API

```go
// Border style definition - pure data struct
type Border struct {
    Top, Bottom, Left, Right         string
    TopLeft, TopRight, BottomLeft, BottomRight string
    MiddleLeft, MiddleRight, Middle, MiddleTop, MiddleBottom string
}

// Predefined borders as factory functions (not vars) - ensures immutability
func NormalBorder() Border    // ─│┌┐└┘
func RoundedBorder() Border   // ─│╭╮╰╯
func ThickBorder() Border     // ━┃┏┓┗┛
func DoubleBorder() Border    // ═║╔╗╚╝
func HiddenBorder() Border    // spaces - preserves layout while invisible
func BlockBorder() Border     // █ full block

// Fluent application
style := lipgloss.NewStyle().
    BorderStyle(lipgloss.RoundedBorder()).
    BorderForeground(lipgloss.Color("63")).
    BorderBackground(lipgloss.Color("227")).
    BorderTop(true).        // selective sides
    BorderLeft(true)

// Shorthand with side flags (clockwise from top)
lipgloss.NewStyle().Border(lipgloss.ThickBorder(), true, false)           // top+bottom only
lipgloss.NewStyle().Border(lipgloss.DoubleBorder(), true, false, false, true) // top+left

// Gradient borders
s := lipgloss.NewStyle().
    Border(lipgloss.RoundedBorder()).
    BorderForegroundBlend(lipgloss.Color("#FF0000"), lipgloss.Color("#0000FF"))
```

**Key Design Decisions:**
1. `Border` is a struct of strings, not runes - supports multi-rune Unicode borders
2. Border size calculation uses `maxRuneWidth()` to handle wide characters
3. Corner rendering adapts to which sides are enabled (e.g., if `!hasLeft`, `TopLeft` becomes empty)
4. Hidden border preserves background color capability for layout shifts without visual borders

### Padding & Margins API

```go
// CSS shorthand - all sides
lipgloss.NewStyle().Padding(2)

// Two values: vertical, horizontal
lipgloss.NewStyle().Margin(2, 4)

// Three values: top, horizontal, bottom
lipgloss.NewStyle().Padding(1, 4, 2)

// Four values: clockwise from top
lipgloss.NewStyle().Margin(2, 4, 3, 1)

// Individual accessors
style.
    PaddingTop(2).PaddingRight(4).PaddingBottom(2).PaddingLeft(4).
    MarginTop(2).MarginRight(4).MarginBottom(2).MarginLeft(4)

// Custom fill characters
s := lipgloss.NewStyle().
    Padding(1, 2).
    PaddingChar('·').       // NBSP-like visual indicator
    Margin(1, 2).
    MarginChar('░')         // visible margin fill
```

**Implementation Detail:**
- Padding uses `\u00A0` (NBSP) by default so copied text preserves spacing
- Margins use regular space by default
- Both support background color inheritance via `marginBackgroundKey`

### Style Inheritance

```go
var styleA = lipgloss.NewStyle().
    Foreground(lipgloss.Color("229")).
    Background(lipgloss.Color("63"))

// Only unset rules are inherited. Foreground already set on B, so only
// Background is inherited from A.
var styleB = lipgloss.NewStyle().
    Foreground(lipgloss.Color("201")).
    Inherit(styleA)

// Margins and padding are NOT inherited by design
// Underlying string values are NOT inherited
```

### Unsetting Rules

```go
var style = lipgloss.NewStyle().
    Bold(true).
    UnsetBold().                       // remove rule entirely
    Background(lipgloss.Color("227")).
    UnsetBackground()
```

---

## 2. Glamour: Markdown Rendering in Terminals

### Architecture
Glamour is a **stylesheet-driven** Markdown-to-ANSI renderer built on `goldmark` (CommonMark parser).

```go
// Simple usage
out, err := glamour.Render(markdownInput, "dark")  // style preset

// Custom renderer with options
r, _ := glamour.NewTermRenderer(
    glamour.WithWordWrap(40),     // wrap at 40 cols
)
out, err := r.Render(input)
```

### Style-Driven Rendering
Styles are JSON/YAML files defining `StylePrimitive` for each Markdown element:

```go
type StylePrimitive struct {
    Color           *string
    BackgroundColor *string
    Bold            *bool
    Italic          *bool
    Underline       *bool
    StrikeThrough   *bool
    Indent          *uint
    Margin          *uint
    Prefix          string
    BlockPrefix     string
    BlockSuffix     string
}
```

**Key insight:** Every Markdown node maps to an `Element` with optional `Entering`/`Exiting` strings and a `Renderer`/`Finisher`.

```go
type Element struct {
    Entering string          // prefix (e.g., "\n" before code block)
    Exiting  string          // suffix
    Renderer ElementRenderer // renders content
    Finisher ElementFinisher // cleanup after children
}
```

### Element Dispatch Pattern

```go
func (tr *ANSIRenderer) NewElement(node ast.Node, source []byte) Element {
    switch node.Kind() {
    case ast.KindHeading:
        n := node.(*ast.Heading)
        he := &HeadingElement{Level: n.Level, First: node.PreviousSibling() == nil}
        return Element{Renderer: he, Finisher: he}

    case ast.KindFencedCodeBlock:
        n := node.(*ast.FencedCodeBlock)
        code := extractLines(n, source)
        return Element{
            Entering: "\n",
            Renderer: &CodeBlockElement{
                Code:     code,
                Language: string(n.Language(source)),
            },
        }

    case ast.KindParagraph:
        return Element{
            Renderer: &ParagraphElement{First: node.PreviousSibling() == nil},
            Finisher: &ParagraphElement{},
        }
    // ... etc
    }
}
```

---

## 3. Syntax Highlighting in Glamour

### Chroma Integration

```go
// CodeBlockElement delegates to Chroma for syntax highlighting
type CodeBlockElement struct {
    Code     string
    Language string
}

func (e *CodeBlockElement) Render(w io.Writer, ctx RenderContext) error {
    // Style extraction from theme config
    theme := ctx.options.Styles.CodeBlock.Theme
    formatter := "terminal256"  // or "terminal", "terminal16m"

    // If a custom Chroma theme is defined, register it dynamically
    if rules.Chroma != nil {
        theme = "charm"
        styles.Register(chroma.MustNewStyle("charm", chroma.StyleEntries{
            chroma.Text:        chromaStyle(rules.Chroma.Text),
            chroma.Keyword:     chromaStyle(rules.Chroma.Keyword),
            chroma.String:      chromaStyle(rules.Chroma.LiteralString),
            chroma.Comment:     chromaStyle(rules.Chroma.Comment),
            // ... 30+ token types mapped
        }))
    }

    // Highlight and write
    err := quick.Highlight(w, e.Code, e.Language, formatter, theme)
    // Fallback to plain text if highlighting fails
}
```

**Design Decisions:**
1. Uses `chroma/quick` for one-shot highlighting
2. Custom themes registered at runtime via `chroma/styles.Register()`
3. Mutex-protected style registry (thread-safe)
4. Falls back to unhighlighted code if Chroma fails
5. Supports configurable formatter: `terminal256`, `terminal16m` (truecolor), `terminal` (8-color)

### Code Block with Syntax Highlighting (Adaptable Pattern)

```go
// For Tidy: simplified syntax highlighter interface
type SyntaxHighlighter interface {
    Highlight(code, language string) (string, error)
}

// Concrete implementation wrapping Chroma
type ChromaHighlighter struct {
    Formatter string // "terminal256", "terminal16m"
    Theme     string // "monokai", "dracula", etc.
}

func (c *ChromaHighlighter) Highlight(code, language string) (string, error) {
    if language == "" {
        return code, nil
    }
    var buf bytes.Buffer
    err := quick.Highlight(&buf, code, language, c.Formatter, c.Theme)
    return buf.String(), err
}
```

---

## 4. Bubbles: Lists, Modals, Dropdowns

### List Component Architecture

```go
// Item interface - minimal contract
type Item interface {
    FilterValue() string  // value used for fuzzy filtering
}

// ItemDelegate - separates rendering logic from data
type ItemDelegate interface {
    Render(w io.Writer, m Model, index int, item Item)
    Height() int
    Spacing() int
    Update(msg tea.Msg, m *Model) tea.Cmd
}
```

**Key Pattern:** The list uses a **delegate pattern** where rendering is externalized. This allows the same data to render differently (compact vs. fancy) without changing the list model.

### List Initialization & Configuration

```go
items := []list.Item{
    item("Ramen"), item("Tomato Soup"), /* ... */
}

l := list.New(items, itemDelegate{}, width, height)
l.Title = "What do you want for dinner?"
l.SetShowStatusBar(false)
l.SetFilteringEnabled(false)     // disable built-in fuzzy filter
l.SetShowPagination(true)
l.SetShowHelp(true)
l.SetShowTitle(true)

// Styles are mutable after creation
l.Styles.Title = lipgloss.NewStyle().MarginLeft(2)
l.Styles.PaginationStyle = lipgloss.NewStyle().PaddingLeft(4)
```

### Model Selector Dropdown Pattern

```go
// From list-simple example - adapted for model selection
type modelSelector struct {
    list     list.Model
    choice   string
    styles   styles
}

type modelItem string
func (i modelItem) FilterValue() string { return string(i) }

type modelDelegate struct{ styles *styles }

func (d modelDelegate) Render(w io.Writer, m list.Model, index int, listItem list.Item) {
    i, ok := listItem.(modelItem)
    if !ok { return }

    str := fmt.Sprintf("%d. %s", index+1, i)
    fn := d.styles.item.Render

    // Selection indicator
    if index == m.Index() {
        fn = func(s ...string) string {
            return d.styles.selectedItem.Render("> " + strings.Join(s, " "))
        }
    }
    fmt.Fprint(w, fn(str))
}

func (d modelDelegate) Height() int  { return 1 }
func (d modelDelegate) Spacing() int { return 0 }
func (d modelDelegate) Update(_ tea.Msg, _ *list.Model) tea.Cmd { return nil }
```

### Filter State Machine

```go
type FilterState int
const (
    Unfiltered    FilterState = iota
    Filtering                  // user actively typing filter
    FilterApplied              // filter set, user not editing
)

// Update loop branches based on filter state
func (m Model) Update(msg tea.Msg) (Model, tea.Cmd) {
    if m.filterState == Filtering {
        return m.handleFiltering(msg)
    }
    return m.handleBrowsing(msg)
}
```

---

## 5. Permission/Confirmation Modal Pattern

From `bubbletea/examples/prevent-quit/main.go` - the canonical confirmation modal:

```go
type model struct {
    textarea   textarea.Model
    hasChanges bool
    quitting   bool  // modal state flag
}

// Intercept quit events at the program level
func filter(teaModel tea.Model, msg tea.Msg) tea.Msg {
    if _, ok := msg.(tea.QuitMsg); !ok {
        return msg
    }
    m := teaModel.(model)
    if m.hasChanges {
        return nil  // swallow quit, trigger modal instead
    }
    return msg
}

// Program setup with filter
p := tea.NewProgram(initialModel(), tea.WithFilter(filter))

// Update branches based on modal state
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    if m.quitting {
        return m.updatePromptView(msg)  // modal interaction
    }
    return m.updateTextView(msg)       // normal editing
}

// Modal interaction handler
func (m model) updatePromptView(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyPressMsg:
        if msg.String() == "y" {
            m.hasChanges = false
            return m, tea.Quit  // confirmed
        }
        m.quitting = false   // cancelled, return to app
    }
    return m, nil
}

// View switches based on state
func (m model) View() tea.View {
    if m.quitting && m.hasChanges {
        // Modal overlay
        text := lipgloss.JoinHorizontal(lipgloss.Top,
            "You have unsaved changes. Quit without saving?",
            choiceStyle.Render("[yN]"))
        return tea.NewView(quitViewStyle.Render(text))
    }
    // Normal view...
}
```

**Design Decisions:**
1. **State machine approach**: `quitting` boolean branches both `Update` and `View`
2. **Event filtering**: `tea.WithFilter` intercepts quit at the framework level before it reaches Update
3. **Modal as view mode**: Not a separate component, just a conditional render path
4. **Style**: `lipgloss.NewStyle().Padding(1, 3).Border(lipgloss.RoundedBorder())` for modal frame

---

## 6. Harmonica: Spring Animations

### Physics-Based Animation API

```go
// Spring initialization - precomputes coefficients
type Spring struct {
    posPosCoef, posVelCoef float64
    velPosCoef, velVelCoef float64
}

// NewSpring(deltaTime, angularFrequency, dampingRatio)
//   deltaTime: time step (use FPS(60) for 60fps)
//   angularFrequency: speed (higher = faster)
//   dampingRatio: springiness (0-1 underdamped, 1 critical, >1 overdamped)
func NewSpring(deltaTime, angularFrequency, dampingRatio float64) Spring

// Update position and velocity toward equilibrium
func (s Spring) Update(pos, vel, equilibriumPos float64) (newPos, newVel float64)
```

### Usage Pattern

```go
// Initialize once
spring := harmonica.NewSpring(harmonica.FPS(60), 6.0, 0.5)

// In animation loop
sprite.x, sprite.xVelocity = spring.Update(sprite.x, sprite.xVelocity, targetX)
sprite.y, sprite.yVelocity = spring.Update(sprite.y, sprite.yVelocity, targetY)
```

### Damping Categories

| Ratio | Behavior |
|-------|----------|
| < 1.0 | Under-damped: fastest reach, overshoots, oscillates with decay |
| = 1.0 | Critically damped: fastest without oscillation |
| > 1.0 | Over-damped: no oscillation, slower than critical |

**Algorithm:** Ryan Juckett's damped simple harmonic oscillator (ported from C++). Precomputes 4 coefficients so `Update()` is just 4 multiplications and 2 additions - extremely fast.

### Adaptation for TUI Layout Animations

```go
// Tidy adaptation: animated panel width
type AnimatedPanel struct {
    targetWidth float64
    currentWidth float64
    velocity float64
    spring harmonica.Spring
}

func NewAnimatedPanel() AnimatedPanel {
    return AnimatedPanel{
        spring: harmonica.NewSpring(harmonica.FPS(60), 8.0, 0.7), // snappy, slight overshoot
    }
}

func (p *AnimatedPanel) SetTarget(width float64) {
    p.targetWidth = width
}

func (p *AnimatedPanel) Update() {
    p.currentWidth, p.velocity = p.spring.Update(
        p.currentWidth, p.velocity, p.targetWidth)
}

func (p *AnimatedPanel) Width() int {
    return int(p.currentWidth)
}
```

---

## 7. Common Layout Patterns

### Split Pane Layout

From `bubbletea/examples/split-editors/main.go`:

```go
type model struct {
    width  int
    height int
    inputs []textarea.Model  // multiple panes
    focus  int               // active pane index
}

// Distribute width evenly
func (m *model) sizeInputs() {
    for i := range m.inputs {
        m.inputs[i].SetWidth(m.width / len(m.inputs))
        m.inputs[i].SetHeight(m.height - helpHeight)
    }
}

// Horizontal join of all pane views
func (m model) View() tea.View {
    views := make([]string, len(m.inputs))
    for i := range m.inputs {
        views[i] = m.inputs[i].View()
    }
    return tea.NewView(
        lipgloss.JoinHorizontal(lipgloss.Top, views...) + "\n\n" + help)
}

// Focus management
func (m *model) nextPane() {
    m.inputs[m.focus].Blur()
    m.focus = (m.focus + 1) % len(m.inputs)
    m.inputs[m.focus].Focus()
}
```

**Key Decisions:**
1. Equal division: `width / len(inputs)`
2. Focus state managed externally (in parent model), not by individual panes
3. `lipgloss.JoinHorizontal(lipgloss.Top, views...)` handles alignment
4. Each pane is a full Bubble Tea model receiving all messages

### Join Utilities

```go
// Horizontal join - align along vertical axis
lipgloss.JoinHorizontal(lipgloss.Top, blockA, blockB, blockC)
lipgloss.JoinHorizontal(lipgloss.Bottom, blockA, blockB)
lipgloss.JoinHorizontal(0.2, blockA, blockB)  // 20% from top

// Vertical join - align along horizontal axis
lipgloss.JoinVertical(lipgloss.Left, blockA, blockB)
lipgloss.JoinVertical(lipgloss.Center, blockA, blockB)

// Both normalize all blocks to same height/width by padding with spaces
```

### Placement Utilities

```go
// Place in a box (centered, right-aligned, etc.)
lipgloss.Place(30, 80, lipgloss.Right, lipgloss.Bottom, paragraph)

// Horizontal placement only
lipgloss.PlaceHorizontal(80, lipgloss.Center, paragraph)

// Vertical placement only
lipgloss.PlaceVertical(30, lipgloss.Bottom, paragraph)

// With styled whitespace
lipgloss.Place(width, height, hPos, vPos, content,
    lipgloss.WithWhitespaceChars("~"),
    lipgloss.WithWhitespaceForeground(lipgloss.Color("241")),
)
```

### Viewport (Scrollable Panel)

```go
type Model struct {
    width  int
    height int
    yOffset int  // vertical scroll position
    xOffset int  // horizontal scroll position
    lines   []string
    Style   lipgloss.Style  // borders, padding applied here
}

// Set content
vp.SetContent(markdownString)     // auto-splits on \n
vp.SetContentLines([]string{})    // direct line array

// Navigation
vp.ScrollDown(n)
vp.ScrollUp(n)
vp.PageDown()
vp.PageUp()
vp.HalfPageDown()
vp.GotoTop()
vp.GotoBottom()

// Cursor-aware visibility
vp.EnsureVisible(line, colStart, colEnd)

// View applies style frame and cuts visible region
func (m Model) View() string {
    contentWidth := m.width - m.Style.GetHorizontalFrameSize()
    contentHeight := m.height - m.Style.GetVerticalFrameSize()
    contents := lipgloss.NewStyle().
        Width(contentWidth).Height(contentHeight).
        Render(strings.Join(m.visibleLines(), "\n"))
    return m.Style.Render(contents)
}
```

**Key Features:**
- Soft wrap mode (`SoftWrap bool`)
- Horizontal scrolling with configurable step
- Left gutter support (`LeftGutterFunc`) for line numbers/indicators
- Highlight ranges with `SetHighlights()` + navigation
- Mouse wheel support

---

## 8. Multi-View State Machine

From `bubbletea/examples/views/main.go`:

```go
type model struct {
    Choice   int      // selected option
    Chosen   bool     // which view is active
    Progress float64  // animation state
    Loaded   bool
}

// Branch update based on view state
func (m model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    if msg, ok := msg.(tea.KeyPressMsg); ok {
        if msg.String() == "q" || msg.String() == "esc" {
            return m, tea.Quit
        }
    }
    if !m.Chosen {
        return updateChoices(msg, m)
    }
    return updateChosen(msg, m)
}

// Branch view based on view state
func (m model) View() tea.View {
    if !m.Chosen {
        return tea.NewView(choicesView(m))
    }
    return tea.NewView(chosenView(m))
}

// Sub-update for choices view
func updateChoices(msg tea.Msg, m model) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyPressMsg:
        switch msg.String() {
        case "j", "down": m.Choice++
        case "k", "up": m.Choice--
        case "enter":
            m.Chosen = true
            return m, frame()  // start animation
        }
    }
    return m, nil
}
```

**Pattern:** Use boolean/enum state in parent model. Branch both `Update` and `View`. Each sub-view has its own update function. No nested `tea.Model` required for simple cases.

---

## 9. Keybinding & Help System

```go
// Define keymap as struct
type KeyMap struct {
    Up   key.Binding
    Down key.Binding
}

// Create bindings with help text
var DefaultKeyMap = KeyMap{
    Up: key.NewBinding(
        key.WithKeys("k", "up"),
        key.WithHelp("↑/k", "move up"),
    ),
    Down: key.NewBinding(
        key.WithKeys("j", "down"),
        key.WithHelp("↓/j", "move down"),
    ),
}

// Match in update loop
func (m Model) Update(msg tea.Msg) (tea.Model, tea.Cmd) {
    switch msg := msg.(type) {
    case tea.KeyPressMsg:
        switch {
        case key.Matches(msg, m.KeyMap.Up):
            // handle up
        case key.Matches(msg, m.KeyMap.Down):
            // handle down
        }
    }
}

// Auto-generate help view
help := help.New()
helpView := help.ShortHelpView([]key.Binding{
    m.keymap.save, m.keymap.quit,
})
```

---

## 10. Adaptation Recommendations for Tidy

### 1. Style System
- **Adopt Lip Gloss fluent API**: `NewStyle().Chain().Chain()`
- **Value types**: Styles should be immutable copies, not mutable references
- **Inheritance model**: `Inherit()` for theme propagation without override
- **CSS shorthand**: `Padding(1, 2, 3, 4)` is intuitive

### 2. Component Architecture
- **Delegate pattern** for lists: separate data (`Item`) from rendering (`ItemDelegate`)
- **State machine** for modals: boolean flag branches `Update` and `View`
- **Event filtering** for global shortcuts: intercept at program level

### 3. Layout Engine
- **Join utilities**: `JoinHorizontal`/`JoinVertical` with position alignment
- **Place utilities**: center content in fixed-size boxes
- **Viewport component**: reusable scrollable panel with gutter support
- **Split panes**: divide width among N components, manage focus externally

### 4. Animation
- **Spring physics**: Harmonica's precomputed coefficients for zero-allocation updates
- **Separate animation state**: `current`, `target`, `velocity` + spring per animated property
- **60fps default**: `harmonica.FPS(60)` as standard timestep

### 5. Markdown Rendering
- **Stylesheet-driven**: JSON/YAML style definitions map to ANSI codes
- **Element dispatch**: AST node -> Element{Renderer, Finisher}
- **Chroma integration**: `quick.Highlight()` with configurable formatter
- **Fallback**: unhighlighted plain text if highlighter fails

### 6. Syntax Highlighting
```go
// Suggested Tidy interface
type Highlighter interface {
    Highlight(code, language string, width int) (string, error)
}

// Configuration
highlight := &ChromaHighlighter{
    Formatter: "terminal256",  // or "terminal16m" for truecolor
    Theme:     "dracula",
}
```

---

## Appendix: Concrete Code Snippets Ready for Adaptation

### Modal Overlay
```go
var modalStyle = lipgloss.NewStyle().
    Padding(1, 3).
    Border(lipgloss.RoundedBorder()).
    BorderForeground(lipgloss.Color("170"))

func renderModal(title, body string) string {
    return modalStyle.Render(
        lipgloss.JoinVertical(lipgloss.Left,
            lipgloss.NewStyle().Bold(true).Render(title),
            "",
            body,
        ),
    )
}
```

### Model Selector Dropdown
```go
type Selector struct {
    items  []string
    cursor int
    styles struct {
        item     lipgloss.Style
        selected lipgloss.Style
    }
}

func (s *Selector) View() string {
    var b strings.Builder
    for i, item := range s.items {
        if i == s.cursor {
            fmt.Fprintln(&b, s.styles.selected.Render("> "+item))
        } else {
            fmt.Fprintln(&b, s.styles.item.Render("  "+item))
        }
    }
    return b.String()
}
```

### Split Pane
```go
func splitPanes(panes []string, totalWidth, totalHeight int) string {
    paneWidth := totalWidth / len(panes)
    styled := make([]string, len(panes))
    for i, p := range panes {
        styled[i] = lipgloss.NewStyle().
            Width(paneWidth).
            Height(totalHeight).
            Border(lipgloss.NormalBorder()).
            Render(p)
    }
    return lipgloss.JoinHorizontal(lipgloss.Top, styled...)
}
```

### Animated Panel
```go
type AnimatedWidth struct {
    target, current, velocity float64
    spring harmonica.Spring
}

func (a *AnimatedWidth) SetTarget(w float64) { a.target = w }
func (a *AnimatedWidth) Update() {
    a.current, a.velocity = a.spring.Update(a.current, a.velocity, a.target)
}
func (a *AnimatedWidth) Value() int { return int(a.current) }
```
