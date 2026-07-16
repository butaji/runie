# Persona: The Platform Engineer

> "I don't just build software — I build the foundation that lets other engineers build software. If your tool can't be extended, integrated, and composed into my platform, it doesn't belong in my stack."

---

## 1. Persona Profile

### Background

**Name:** Jordan Reyes  
**Age:** 32  
**Occupation:** Principal Platform Engineer at a Series C SaaS company  
**Location:** Austin, TX (hybrid)  
**Platform Experience:** 8 years in platform/infrastructure engineering  

Jordan leads the Developer Experience team, responsible for the internal developer platform (IDP), CI/CD infrastructure, and developer tooling that serves 200+ engineers. They spent the first five years of their career as a backend engineer before transitioning to platform work. This dual background gives them a unique perspective: they understand both the code developers write and the systems that deploy and operate it.

Their platform powers everything from local development environments to production deployments. They've built custom CLIs, internal developer portals (using Backstage), self-service infrastructure templates, and AI-assisted coding tools. When evaluating new tools, Jordan thinks in terms of **platforms**, not point solutions.

### Expertise Level

**Expert/Power User** — Jordan represents the top tier of platform engineers:

- Deep knowledge of distributed systems, Kubernetes, and cloud-native architecture
- Comfortable reading kernel source when debugging
- Writes Go, Python, and Rust for platform tooling
- Has built multiple internal platforms from scratch
- Understands API design, extensibility patterns, and plugin architectures
- Presented at internal conferences and written extensively about platform engineering

### Work Style

Jordan's day is a mix of deep work and collaborative problem-solving:

- **Mornings:** Reviews platform metrics, addresses overnight incidents, syncs with on-call engineers
- **Mid-day:** Architecture sessions with product teams, code reviews for platform changes
- **Afternoons:** Building new platform capabilities, integrating tools, writing internal documentation
- **Evenings:** Stays current with the platform engineering community, evaluates new tools

They use a combination of terminal tools (tmux, neovim, lazygit), a web-based Backstage portal, and occasionally VS Code for pair programming sessions. Everything must be keyboard-driven, scriptable, and composable.

### Tools in Their Stack

| Category | Tool | Why It Fits Their Philosophy |
|----------|------|----------------------------|
| **IDP** | Backstage | Extensible, plugin-based, opinionated defaults |
| **IaC** | Terraform + Pulumi | Code-as-infrastructure, composable modules |
| **Containers** | Kubernetes (EKS) + Helm | Cloud-native standard, templatable |
| **CI/CD** | GitHub Actions + ArgoCD | GitOps workflow, declarative |
| **Service Mesh** | Istio | Transparent, extensible |
| **Observability** | OpenTelemetry + Grafana | Vendor-neutral, composable |
| **Terminal** | tmux + neovim | Keyboard-driven, scriptable |
| **AI Tools** | Claude Code, Continue.dev | Terminal-friendly, extensible |
| **MCP** | Cursor, Continue.dev | Protocol-native AI integration |

---

## 2. Goals and Motivations

### Primary Goals

1. **Build internal platforms that other engineers love** — Jordan's success metric is developer satisfaction scores and platform adoption rates

2. **Enable self-service infrastructure** — Every ticket that requires platform team intervention is a failure

3. **Reduce cognitive load for application teams** — Platform should handle complexity so developers can focus on business logic

4. **Maintain security and compliance without friction** — Guardrails should be invisible when followed, clear when violated

5. **Create composable, extensible systems** — Build blocks that can be combined in unexpected ways

### Motivations

**PLATFORM SCALE (Primary)** — "When I build something, it affects 200 engineers. A 10% improvement in developer productivity translates to 20 engineers freed up daily. That's the real leverage."

**COMPOSABILITY (Secondary)** — "I don't want point solutions. I want tools that speak to each other through well-defined interfaces. Give me a great CLI with a solid API, and I'll integrate it into everything."

**INFRASTRUCTURE AS CODE (Tertiary)** — "Everything should be declarative, version-controlled, and reviewable. If it can't be git-diffed, it doesn't belong in production."

**TEACHING AND ENABLING (Quaternary)** — "My job isn't to be the hero who solves every problem. It's to build systems that let others solve their own problems."

---

## 3. Pain Points with Current Tools

### Pain Point #1: The "Vendor Lock-in" Trap

Jordan has been burned by tools that seemed great initially but became dead ends:

- Tools with no plugin system, no API, no way to extend them
- "Platforms" that required their own proprietary infrastructure
- Vendors who changed pricing/behavior after teams were locked in

**Quote:** *"I won't adopt a tool unless I can extend it. If you don't have an MCP server, a plugin API, and an open configuration format, you're not a platform — you're a feature."*

**Impact:** Wasted evaluation cycles, migration costs, team frustration

### Pain Point #2: Fragile AI Tool Integrations

Current AI tools don't play well with platform infrastructure:

- Claude Code works great standalone but can't access internal MCP servers
- Cursor's extension system is limited to VS Code ecosystem
- No tool provides a proper plugin API for custom capabilities
- AI suggestions ignore internal libraries and conventions

**Quote:** *"We have internal code generators, internal API clients, internal best practices. The AI doesn't know any of this. It generates generic code that doesn't match our patterns."*

**Impact:** Reduced productivity, inconsistent code, manual corrections

### Pain Point #3: Context Isolation

AI tools exist in isolation from platform context:

- Don't understand our service catalog or dependencies
- Can't read our architecture decision records (ADRs)
- Ignore internal RFCs and design documents
- Suggest patterns that conflict with our infrastructure

**Quote:** *"I want AI that knows our codebase, our conventions, our internal packages. Not generic patterns that I've already told it not to use."*

**Impact:** Constant re-explaining, manual corrections, eroded trust

### Pain Point #4: Testing AI Output is Hard

Verifying AI-generated code requires the same effort as writing it:

- No fixture/recording system for deterministic testing
- Can't replay API responses in CI
- "Magic" behavior makes debugging impossible
- No way to audit what the AI did and why

**Quote:** *"If I can't test it, I can't trust it in production. Every AI tool needs a way to record sessions, replay fixtures, and verify behavior."*

**Impact:** Unable to use AI in production environments, manual verification overhead

### Pain Point #5: The Monolith Anti-Pattern

Modern "AI platforms" violate everything Jordan believes:

- All-in-one solutions that can't be decomposed
- Proprietary formats that prevent inspection
- Features built on features, each adding dependencies
- No clear separation between interface and engine

**Quote:** *"I see tools pulling in 400 dependencies before generating a single token. That's not a platform — that's complexity masquerading as capability. Where's the Unix philosophy?"*

**Impact:** Bloated systems, security vulnerabilities, unmaintainable codebases

---

## 4. What Would Delight This User

### Delight #1: True Extensibility

Jordan wants a tool with clear extension points:

```go
// Example: Custom MCP server for internal packages
type PlatformPlugin struct {
    Name    string
    Version string
    Handlers []Handler
}

// Plugin lifecycle
func (p *PlatformPlugin) Initialize(cfg Config) error
func (p *PlatformPlugin) Serve(ctx context.Context) error
func (p *PlatformPlugin) Shutdown() error
```

**What impresses Jordan:**
- Well-documented plugin API
- Clear interface contracts
- Multiple extension mechanisms (plugins, MCP, webhooks)
- No vendor lock-in

### Delight #2: Internal Platform Integration

AI that knows and respects internal systems:

```
Platform Context Available:
├── Service Catalog (Backstage)
│   ├── Dependencies
│   ├── SLOs
│   └── On-call rotation
├── Internal Packages
│   ├── @acme/api-client (v2.3.1)
│   ├── @acme/logger (v1.0.0)
│   └── @acme/config (v0.9.0)
├── Architecture Standards
│   ├── ADR-023: Service Mesh Pattern
│   └── ADR-045: API Design Guidelines
└── Platform Capabilities
    ├── Deployment: ArgoCD
    ├── Secrets: Vault
    └── Observability: OpenTelemetry
```

**What impresses Jordan:** "It already knows our conventions. It generates code that matches our patterns, uses our internal packages, and follows our architecture."

### Delight #3: Composable Architecture

Following Eric Raymond's 17 rules:

> **Rule #1: Modularity** — Write simple parts connected by clean interfaces  
> **Rule #3: Composition** — Design programs to be connected to other programs  
> **Rule #5: Simplicity** — Design for simplicity; add complexity only where you must

**What impresses Jordan:**
- Engine separable from interface
- Multiple frontends (TUI, CLI, API)
- Headless mode for CI
- Exit codes as contracts

### Delight #4: First-Class MCP Support

Jordan uses MCP everywhere. They want Runie to be an MCP-native tool:

```json
// Example MCP server manifest
{
  "name": "runie",
  "version": "1.0.0",
  "capabilities": [
    "code-generation",
    "code-review", 
    "refactoring",
    "test-generation"
  ],
  "tools": [
    {
      "name": "generate_code",
      "description": "Generate code from natural language",
      "inputSchema": { ... }
    },
    {
      "name": "refactor",
      "description": "Refactor existing code",
      "inputSchema": { ... }
    }
  ]
}
```

### Delight #5: Transparent, Auditable Behavior

Following Unix philosophy Rule #7: **Transparency**

> "Design for visibility to make inspection and debugging easier."

**What impresses Jordan:**
- Every action logged with context
- Diff-first for all changes
- Clear reasoning visible before execution
- Session recordings for replay/debugging

---

## 5. Specific UI/UX Recommendations for Runie

### 5.1 Keyboard-First Navigation

Following TUI best practices, Runie must be fully keyboard-driven:

| Key | Action | Context |
|-----|--------|---------|
| `j` / `k` | Move down/up | All lists |
| `h` / `l` | Back/forward | Navigation |
| `gg` / `G` | Top/bottom | Lists |
| `/` | Search | Search mode |
| `:` | Command palette | Command input |
| `Esc` | Cancel/back | Universal abort |
| `Ctrl+P` | Quick open | Files, commands |
| `Ctrl+b` | Toggle sidebar | Panels |
| `mm` | Model switcher | Model selection |

### 5.2 High-Density Information Display

Jordan prefers information-dense interfaces. The status bar should show:

```
[Platform Context] │ Provider: anthropic │ Model: opus-4 │ Mock: ON │ Session: platform-eng-2026-07-15 │ 3 pending changes
```

**Principles from Cognitive Load research:**
- Show critical information at a glance
- Use progressive disclosure for details
- Group related information
- Never hide information users need

### 5.3 Diff-First Workflow

Following Unix philosophy Rule #12: **Repair**

> "When you must fail, fail noisily and as soon as possible."

Before any file modification:

```
REVIEW CHANGES (4 files)

src/services/user_service.go
- func GetUser(id string) *User {
+ func GetUser(ctx context.Context, id string) (*User, error) {

src/services/user_service_test.go
+ func TestGetUserSuccess(t *testing.T) { ... }

config/platform.yaml
+ database_pool_size: 25

internal/api/client.go
- import "github.com/acme/logger"
+ import "github.com/acme/logger/v2"

[a] Accept all  [n] Reject all  [1-4] File-specific  [e] Edit hunk  [?] Help
```

### 5.4 Plugin-Enabled Architecture

Following the Elm Architecture pattern:

```
┌─────────────────────────────────────────────────────────────┐
│                    Runie Architecture                        │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   ┌───────────┐  ┌───────────┐  ┌───────────┐  ┌──────────┐ │
│   │   TUI    │  │   CLI    │  │   API    │  │   MCP   │ │
│   └─────┬─────┘  └─────┬─────┘  └─────┬─────┘  └────┬─────┘ │
│         └──────────────┼──────────────┼─────────────┘        │
│                        ▼                                    │
│              ┌─────────────────┐                            │
│              │     Engine      │                            │
│              │   (Core Logic)  │                            │
│              └────────┬────────┘                            │
│                       │                                     │
│         ┌─────────────┼─────────────┐                       │
│         ▼             ▼             ▼                       │
│   ┌──────────┐ ┌──────────┐ ┌──────────┐                     │
│   │ Plugins  │ │  MCP    │ │ Fixtures│                     │
│   │  API    │ │ Servers │ │  System │                     │
│   └──────────┘ └──────────┘ └──────────┘                     │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 5.5 Command Palette with Extensible Commands

```
> ▌

> :plugin
► :plugin list
  :plugin install
  :plugin uninstall
  :plugin configure

> :mcp
  :mcp list-servers
  :mcp add-server
  :mcp invoke tool-name

> :platform
  :platform sync-catalog
  :platform validate-adr
  :platform check-dependencies
```

### 5.6 Error Messages with Context

Following the TUI best practices for error presentation:

```
✗ ERROR: Internal Platform Integration

  Failed to load service catalog from Backstage:
  → Connection refused: http://backstage.internal:7000
  
  This may indicate:
  • Backstage is down
  • Network connectivity issues
  • Invalid API token

  Context available:
  • Platform mode: enabled
  • Backstage URL: from config
  • Last successful sync: 2 hours ago

  [r] Retry   [c] Continue without platform context   [?] Help
```

---

## 6. Default Behaviors That Would Impress Them

### 6.1 Platform-Aware Defaults

On first launch, Runie should auto-detect platform context:

```
Detected Platform Environment:
├── Backstage: http://backstage.internal:7000 (connected)
├── Service Catalog: 127 services loaded
├── Internal Registry: npm.acme.internal (connected)
├── MCP Servers: 3 configured
│   ├── acme-packages (internal package docs)
│   ├── platform-tools (infra utilities)
│   └── incident-response (PagerDuty, Slack)
└── Architecture Standards: 23 ADRs loaded

Platform-aware mode enabled. Generating context-aware suggestions.
```

### 6.2 Sensible Defaults That "Just Work"

Following the zero-decision UX principle from cognitive load research:

- **Mock mode ON** — No accidental real API calls
- **Diff-first ON** — Must explicitly accept changes
- **Platform context enabled** — Auto-load internal knowledge
- **Structured logging** — JSON to stdout, human to stderr
- **Exit code contracts** — Meaningful, documented codes

### 6.3 Transparent Rate Limiting

When approaching limits:

```
⚠ Rate limit warning (claude-sonnet-4)
├── Used: 45/50 requests
├── Resets in: 23 seconds
├── Queued requests: 2
│
├── Options:
│   [1] Switch to claude-haiku (100 req/min)
│   [2] Wait 23 seconds
│   [3] Continue (may fail)
│   [4] Queue for later
│
└── Note: Platform mode active. Internal rate limits may apply separately.
```

### 6.4 Fixture/Recording Mode for Testing

For CI and deterministic testing:

```bash
# Record a session for replay
RUNIE_RECORD=./fixtures/platform-eng-001.runie \
  runie "generate user service with repository pattern"

# Replay in CI without real API calls
RUNIE_MOCK=./fixtures/platform-eng-001.runie \
  runie test --pipeline generate-service

# Verify fixture coverage
runie fixture-coverage --report
# → Fixtures: 94% coverage
# → Missing: delete operations, error paths
```

### 6.5 Persistent Session State

Following memory aids from cognitive load research:

```
Session restored: platform-eng-2026-07-15

├── Context: user service generation (in progress)
├── Pending changes: 4 files
├── Last action: 45 minutes ago
└── Model: claude-opus-4

[r] Resume session  [n] New session  [?] Help
```

---

## 7. API and Integration Requirements

### 7.1 RESTful API for Platform Integration

Jordan needs programmatic access for internal tooling:

```bash
# List available capabilities
GET /api/v1/capabilities

# Generate code
POST /api/v1/generate
{
  "prompt": "generate a user repository with CRUD operations",
  "context": {
    "language": "go",
    "framework": "standard-library",
    "pattern": "repository"
  },
  "platform": {
    "packages": ["github.com/acme/db"],
    "conventions": "go-standard"
  }
}

# Validate against platform standards
POST /api/v1/validate
{
  "code": "...",
  "rules": ["acme-go-style", "security-scan"]
}

# Get platform context
GET /api/v1/platform/context
```

### 7.2 Streaming Responses

For real-time feedback:

```bash
# SSE streaming for long operations
POST /api/v1/generate/stream
Content-Type: text/event-stream

event: thinking
data: {"type": "analyzing", "message": "Reading existing code patterns..."}

event: thinking
data: {"type": "generating", "message": "Applying repository pattern..."}

event: diff
data: {"file": "user_repository.go", "changes": [...]}

event: complete
data: {"files": 4, "tokens": 2341, "duration_ms": 4523}
```

### 7.3 Webhook System

For event-driven integrations:

```yaml
# Example webhook configuration
webhooks:
  - name: "platform-sync"
    url: "http://backstage.internal/webhooks/runie"
    events:
      - code.generated
      - code.refactored
      - session.completed
    filter:
      context.platform: true
    retry:
      max_attempts: 3
      backoff: exponential
```

### 7.4 gRPC Interface

For high-performance internal communication:

```protobuf
service RunieService {
  rpc Generate(GenerateRequest) returns (GenerateResponse);
  rpc GenerateStream(GenerateRequest) returns (stream GenerateEvent);
  rpc Validate(ValidateRequest) returns (ValidateResponse);
  rpc GetContext(ContextRequest) returns (ContextResponse);
  rpc ListPlugins(Empty) returns (PluginList);
}

message GenerateRequest {
  string prompt = 1;
  GenerationContext context = 2;
  PlatformContext platform = 3;
  GenerationOptions options = 4;
}

message PlatformContext {
  repeated string service_catalog_ids = 1;
  repeated string package_registry_urls = 2;
  repeated string architecture_documents = 3;
  map<string, string> internal_config = 4;
}
```

### 7.5 OpenAPI Documentation

Auto-generated from code:

```bash
# Get OpenAPI spec
curl http://localhost:8080/openapi.json

# Interactive docs
open http://localhost:8080/docs
```

### 7.6 Exit Code Contracts

Following Unix philosophy, exit codes are contracts:

| Code | Meaning | Use Case |
|------|---------|----------|
| `0` | Success | Operation completed as expected |
| `1` | General error | Something went wrong |
| `2` | Misuse | Invalid arguments or usage |
| `3` | Configuration error | Invalid config file |
| `4` | Execution error | Command failed during execution |
| `5` | Timeout | Operation exceeded time limit |
| `6` | Platform error | Internal platform unavailable |
| `7` | Plugin error | Plugin failed or unavailable |
| `130` | Interrupted | Ctrl+C pressed |

---

## 8. Extensibility and Plugin System Expectations

### 8.1 Multiple Extension Mechanisms

Jordan expects flexibility in how Runie can be extended:

```
┌─────────────────────────────────────────────────────────────┐
│                   Extension Mechanisms                       │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│   1. MCP Servers (Model Context Protocol)                   │
│      ├── Native MCP support                                 │
│      ├── Register custom tools                              │
│      └── Compose with existing servers                      │
│                                                              │
│   2. Plugin API (First-class plugins)                       │
│      ├── Go/Rust/Python SDK                                │
│      ├── Lifecycle management                              │
│      └── Sandboxed execution                               │
│                                                              │
│   3. Webhook System (Event-driven)                         │
│      ├── HTTP callbacks on events                          │
│      ├── Retry with backoff                                │
│      └── Event filtering                                   │
│                                                              │
│   4. Scripting (Lightweight automation)                    │
│      ├── Lua/JavaScript embeddings                         │
│      ├── Hooks for events                                  │
│      └── Custom commands                                   │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 8.2 MCP Server Implementation

Runie should be both an MCP client and server:

```go
// Runie as MCP server (expose Runie capabilities to other tools)
type MCPServer struct {
    engine    *Engine
    tools     []Tool
    resources []Resource
}

func (s *MCPServer) HandleToolCall(ctx context.Context, req ToolCallRequest) (*ToolCallResponse, error) {
    switch req.Name {
    case "runie.generate":
        return s.handleGenerate(ctx, req)
    case "runie.refactor":
        return s.handleRefactor(ctx, req)
    case "runie.explain":
        return s.handleExplain(ctx, req)
    default:
        return nil, fmt.Errorf("unknown tool: %s", req.Name)
    }
}

// Runie as MCP client (use other MCP servers)
type MCPClient struct {
    servers map[string]*MCPServerConnection
}

func (c *MCPClient) UseServer(name string, server *ServerConfig) error {
    conn, err := mcp.Dial(server.Address)
    if err != nil {
        return fmt.Errorf("failed to connect to %s: %w", name, err)
    }
    c.servers[name] = conn
    return nil
}
```

### 8.3 Plugin SDK Example

Simple plugin implementation:

```go
// Internal packages plugin
package acme_packages

import (
    "github.com/runie/runie/sdk"
)

type AcmePackagesPlugin struct {
    registry string
    cache    *PackageCache
}

func (p *AcmePackagesPlugin) Initialize(cfg sdk.PluginConfig) error {
    p.registry = cfg.GetString("registry", "npm.acme.internal")
    p.cache = NewPackageCache()
    return nil
}

func (p *AcmePackagesPlugin) ProvideContext(ctx *sdk.Context) error {
    // Inject internal package knowledge into context
    packages, err := p.cache.GetAll()
    if err != nil {
        return err
    }
    ctx.AddKnowledge("internal_packages", packages)
    return nil
}

func (p *AcmePackagesPlugin) Validate(code string) []sdk.ValidationError {
    // Ensure code uses internal packages
    return validateInternalImports(code, p.cache)
}

func (p *AcmePackagesPlugin) Shutdown() error {
    return p.cache.Close()
}

// Register the plugin
var Plugin = &AcmePackagesPlugin{}
func init() {
    sdk.RegisterPlugin("acme-packages", Plugin)
}
```

### 8.4 Plugin Registry

```bash
# List available plugins
runie plugin list

# Install from registry
runie plugin install acme-packages

# Install from URL
runie plugin install https://plugins.internal/acme-platform-1.0.0.so

# Configure plugin
runie plugin configure acme-packages --set registry=npm.acme.internal

# Update plugin
runie plugin update acme-packages

# Uninstall
runie plugin uninstall acme-packages
```

### 8.5 Hook System

For customizing behavior:

```yaml
# .runie/hooks.yaml
hooks:
  before_generate:
    - script: "scripts/validate-context.sh"
      timeout: 5s
    
  after_generate:
    - script: "scripts/format-code.sh"
    - webhook: "http://backstage.internal/webhooks/code-generated"
    
  on_error:
    - script: "scripts/report-error.sh"
    - webhook: "http://slack.internal/webhooks/errors"
    
  before_refactor:
    - script: "scripts/backup.sh"
      timeout: 30s
```

### 8.6 Platform Template System

For internal platform templates:

```bash
# Define templates
runie template init --name "acme-service"
cat > ~/.runie/templates/acme-service.yaml << 'EOF'
name: acme-service
description: Standard microservice template for Acme
context:
  packages:
    - github.com/acme/logger/v2
    - github.com/acme/config
    - github.com/acme/db
  patterns:
    - error-handling: standard
    - logging: structured
    - config: yaml-viper
  linting:
    - acme-go-style
    - security-scan
EOF

# Use template
runie generate --template acme-service "user service with repository"
```

---

## 9. How Runie Can Exceed Their Expectations (Wow Factors)

### Wow Factor #1: Internal Platform Awareness

Not just "aware of internal code" but truly integrated:

```
┌─────────────────────────────────────────────────────────────┐
│ Platform Intelligence Active                                 │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Service: user-service                                       │
│ Owner: platform-team@acme.com                              │
│ SLO: 99.9% uptime (current: 99.95%)                        │
│ On-call: Jordan Reyes                                       │
│                                                              │
│ Dependencies:                                               │
│ ├── database (postgres, managed)                           │
│ ├── cache (redis, 3 replicas)                            │
│ ├── auth (oauth2-proxy)                                    │
│ └── notifications (sns-sqs)                               │
│                                                              │
│ Recent ADRs relevant to this service:                       │
│ ├── ADR-023: Service Mesh Pattern (implemented)           │
│ └── ADR-051: Observability Standards (in progress)        │
│                                                              │
│ Generating code with platform awareness...                  │
└─────────────────────────────────────────────────────────────┘
```

### Wow Factor #2: Self-Documenting Platform

Generate and maintain documentation automatically:

```bash
$ runie document --service user-service

Generated: docs/services/user-service/README.md

# User Service

## Overview
Standard microservice following Acme platform patterns.

## API Endpoints

### GET /users/{id}
Returns user by ID.

**Request:**
```json
{
  "id": "uuid"
}
```

**Response:** `200 OK`
```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "email": "user@acme.com",
  "created_at": "2024-01-15T10:30:00Z"
}
```

## Dependencies
| Service | Type | Connection |
|---------|------|-----------|
| database | PostgreSQL | Primary |
| cache | Redis | Read-through |
| auth | OAuth2 | Bearer token |

## Observability
- Traces: OpenTelemetry enabled
- Metrics: custom.googleapis.com/user_service/
- Logs: structured JSON → Loki
- Alerts: PagerDuty #platform-alerts

## Deployment
- ArgoCD Application: user-service
- Image: gcr.io/acme/user-service
- Replicas: 3 (HPA: 2-10)
- Resources: 500m CPU, 512Mi memory
```

### Wow Factor #3: Platform Health Dashboard

Real-time platform status in the TUI:

```
┌─────────────────────────────────────────────────────────────┐
│ Platform Health                              [Auto-refresh] │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Build System                   ████████████████████ 100%   │
│ ├── GitHub Actions             ████████████████████ OK    │
│ ├── ArgoCD                     ████████████████████ OK    │
│ └── Container Registry         ███████████████░░░░ 95%   │
│                                                              │
│ Developer Tools                  ██████████████████░░ 90%   │
│ ├── Backstage                   ████████████████████ OK    │
│ ├── Runie                       ████████████████████ OK    │
│ ├── Internal Registry          ███████████████░░░░ 95%   │
│ └── MCP Servers                 ████████████████████ OK    │
│                                                              │
│ Infrastructure                   ████████████████████ 100% │
│ ├── Kubernetes                 ████████████████████ OK    │
│ ├── Service Mesh                ████████████████████ OK    │
│ └── Vault                       ████████████████████ OK    │
│                                                              │
│ ⚠ Warning: Internal registry at 95% storage               │
│ [r] Refresh  [d] Details  [?] Help                        │
└─────────────────────────────────────────────────────────────┘
```

### Wow Factor #4: Incident Response Mode

Special mode for on-call engineers:

```
┌─────────────────────────────────────────────────────────────┐
│ 🚨 INCIDENT MODE — PagerDuty #INC-2024-0847                 │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ Alert: High Latency — user-service                          │
│ Severity: P2                                                │
│ Started: 14:32 (23 minutes ago)                            │
│ On-call: Jordan Reyes                                       │
│                                                              │
│ ┌────────────────────────────────────────────────────────┐ │
│ │ P99 Latency: ███████████████████░░░░ 2.3s (thr: 200ms) │ │
│ │ Error Rate:  ████████░░░░░░░░░░░░░░ 8.2% (thr: 0.1%)  │ │
│ │ CPU:         ████████████░░░░░░░░░░ 62%                │ │
│ └────────────────────────────────────────────────────────┘ │
│                                                              │
│ Likely Causes:                                              │
│ [1] Database connection pool exhausted (78% confidence)    │
│ [2] Upstream dependency slow (45% confidence)              │
│ [3] Cache miss rate increased (23% confidence)             │
│                                                              │
│ Quick Actions:                                              │
│ [s] Scale replicas    [c] Check connections    [l] Logs    │
│ [r] Run diagnostics   [x] Execute runbook    [?] Help      │
│                                                              │
│ Runie Analysis:                                             │
│ "Connection pool exhaustion detected. Pool config: 10 conn, │
│  current usage: 10/10. Recommendation: Increase pool size │
│  to 25 or add connection recycling."                        │
│                                                              │
│ [a] Apply recommendation  [d] Detailed analysis  [x] Exit  │
└─────────────────────────────────────────────────────────────┘
```

### Wow Factor #5: Architecture Validation

Validate code against organizational standards:

```bash
$ runie validate --service user-service --rules all

Validating against Acme Platform Standards...

✓ GO-STYLE-001: Error handling follows standard pattern
✓ GO-STYLE-002: Logging uses structured logger
✓ GO-STYLE-003: Configuration via viper
✓ GO-STYLE-004: Context propagation present
⚠ SEC-001: SQL injection risk in query builder (line 142)
⚠ OBS-002: Missing span for database operation (line 156)
✓ OBS-003: Structured logging present
✓ OBS-004: Metrics exported to OpenTelemetry

Validation Result: 2 warnings, 0 errors

[✓] Passed  [w] View warnings  [f] Full report  [?] Help
```

### Wow Factor #6: Multi-Team Collaboration

Platform engineering for multiple teams:

```bash
$ runie platform teams

┌─────────────────────────────────────────────────────────────┐
│ Acme Platform Teams                                          │
├─────────────────────────────────────────────────────────────┤
│ TEAM              │ SERVICES │ STATUS │ ENG LEAD           │
│───────────────────┼──────────┼────────┼────────────────────│
│ payments          │ 12       │ ● OK   │ Alex Chen           │
│ identity          │ 8        │ ● OK   │ Sam Park            │
│ notifications     │ 15       │ ⚠ WARN │ Jordan Reyes       │
│ analytics         │ 23       │ ● OK   │ Chris Lee           │
└─────────────────────────────────────────────────────────────┘

$ runie platform cross-team --from identity --to payments

Cross-Team Dependency Analysis:
├── identity → payments
│   ├── /api/v1/payments/authorize (sync)
│   └── /api/v1/payments/verify (async)
│
├── Potential Issues:
│   └── Circular dependency risk if payments imports identity
│
└── Recommendations:
    └── Use event-driven communication for async operations
```

### Wow Factor #7: The "Platform Contract"

A living document of what the platform guarantees:

```
┌─────────────────────────────────────────────────────────────┐
│ Platform Contract: user-service                             │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│ This service is guaranteed:                                 │
│                                                              │
│ ✓ Uptime: 99.9% (P2 incident SLA)                          │
│ ✓ Latency: P99 < 200ms                                     │
│ ✓ Availability: Multi-AZ deployment                        │
│ ✓ Observability: Metrics, traces, logs                      │
│ ✓ Security: mTLS, secrets via Vault                        │
│ ✓ Deployments: Blue-green via ArgoCD                       │
│ ✓ Rollbacks: One-command, < 5 minutes                      │
│                                                              │
│ This service must:                                          │
│                                                              │
│ → Use platform logger (github.com/acme/logger)             │
│ → Export OpenTelemetry spans                               │
│ → Follow naming conventions (ADR-023)                      │
│ → Register in Backstage service catalog                    │
│ → Define SLOs in platform config                          │
│ → Include integration tests for dependencies               │
│                                                              │
│ Compliance: 8/10 requirements met                           │
│ [u] Update contract  [v] View full requirements  [?] Help  │
└─────────────────────────────────────────────────────────────┘
```

---

## Summary: What Runie Must Do for Platform Engineers

### MUST HAVE

| Requirement | Implementation |
|-------------|----------------|
| **Plugin API** | SDK for Go/Rust/Python, lifecycle management, sandboxed execution |
| **MCP Support** | Native client AND server, custom tool registration |
| **Platform Context** | Service catalog integration, internal package awareness |
| **API-first** | REST + gRPC, OpenAPI docs, webhooks |
| **Headless Mode** | Full functionality via CLI, CI-ready |
| **Fixture System** | Record/replay for testing, deterministic output |
| **Exit Code Contracts** | Meaningful codes, documented behavior |
| **Composable Output** | stdout/stderr separation, structured formats |

### SHOULD HAVE

| Enhancement | Impact |
|-------------|--------|
| Template System | Standardized code generation for organization |
| Architecture Validation | Automated ADR compliance checking |
| Multi-Team Support | Platform engineering for multiple teams |
| Incident Response Mode | On-call optimization |
| Self-Documenting Code | Automatic README/API docs from code |

### WOW FACTORS

| Feature | Why It Impresses |
|---------|------------------|
| Platform Intelligence | Knows internal systems, generates context-aware code |
| Service Catalog Integration | References real dependencies, owners, SLOs |
| Platform Health Dashboard | Unified view of developer tool health |
| Architecture ADRs | Validates against organizational decisions |
| Incident Response Mode | Dramatically reduces MTTR |

---

## References

- [Coding Agents UX Research](../research/coding_agents_ux.md) — Trust, verification, platform-scale requirements
- [Unix Philosophy Research](../research/unix_philosophy.md) — Composability, modularity, extensibility, text streams
- [TUI Best Practices Research](../research/tui_best_practices.md) — Keyboard navigation, information density, Elm architecture
- [Cognitive Load UX Research](../research/cognitive_load_ux.md) — Progressive disclosure, decision fatigue, context switching
- [MCP Documentation](https://modelcontextprotocol.io) — Model Context Protocol specification
- [Backstage](https://backstage.io) — Internal developer platform reference
- [Platform Engineering Best Practices](https://platformengineering.org) — Platform engineering community

---

*Document version: 1.0*  
*Created: 2026-07-15*  
*Research basis: Platform engineering community analysis, internal developer platform design, MCP ecosystem, Unix philosophy application*
