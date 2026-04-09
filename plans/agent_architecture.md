# Agent Architecture Implementation Plan

## Overview

This plan details the implementation of an autonomous security agent system for Slapper, enabling:
1. AI-guided security testing with full tool access
2. Longitudinal monitoring of self-owned infrastructure
3. Proactive first-alert capabilities for security teams

## Architecture Summary

```
┌─────────────────────────────────────────────────────────────────────┐
│                         User / TUI / CLI                             │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Commands (handlers/)                            │
│  scan, fuzz, recon, mcp-serve, serve, agent-run, etc.               │
└─────────────────────────────────────────────────────────────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          ▼                     ▼                     ▼
┌─────────────────────┐ ┌─────────────────────┐ ┌─────────────────────┐
│    MCP Server       │ │   Agent Loop         │ │   Direct Tool       │
│   (tool/protocol/)  │ │   (agent/)           │ │   Access            │
│                     │ │                     │ │                     │
│  - HTTP + STDIO     │ │  - Event-driven     │ │  - CLI commands     │
│  - JSON-RPC 2.0     │ │  - CronScheduler    │ │  - TUI              │
│  - Tool exposure    │ │  - Alert routing    │ │                     │
│  - External AI      │ │  - Memory system    │ │                     │
└─────────────────────┘ └─────────────────────┘ └─────────────────────┘
                                │
          ┌─────────────────────┼─────────────────────┐
          ▼                     ▼                     ▼
┌─────────────────────────────────────────────────────────────────────┐
│                      Tool Layer (tool/)                              │
│  ToolRegistry │ ToolDispatcher │ ChainPlanner │ SecurityTool         │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     AI Module (ai/)                                   │
│  AiClient │ AiPayloadGenerator │ SmartWafBypass │ AdaptiveScanEngine  │
└─────────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌─────────────────────────────────────────────────────────────────────┐
│                   External LLM / Local Ollama                         │
└─────────────────────────────────────────────────────────────────────┘
```

## Implementation Phases

### Phase 1: MCP Server (Complete Existing Implementation)

**Goal:** Wire up the existing MCP server implementation and add STDIO mode.

**Status:** Router and handlers exist but are stubbed out in `handle_mcp_serve()`.

#### 1.1 Replace Stub with Actual Server Startup

**File:** `crates/slapper/src/commands/handlers/notify.rs`

**Change:** Replace the `[STUB]` handler with actual Axum server startup.

```rust
#[cfg(feature = "rest-api")]
pub async fn handle_mcp_serve(_ctx: &CommandContext, args: crate::cli::McpServeArgs) -> Result<()> {
    use axum::Server;
    use std::net::SocketAddr;
    use crate::tool::create_default_registry;
    use crate::tool::protocol::mcp::routes::create_mcp_router;

    let registry = create_default_registry();
    let router = create_mcp_router(registry, args.api_key.clone());

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address: {}", e))?;

    tracing::info!("Starting MCP server on {}", addr);

    Server::bind(&addr)
        .serve(router)
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}
```

#### 1.2 Implement STDIO Mode

**Files:**
- `crates/slapper/src/commands/handlers/notify.rs` - Add stdio handler
- CLI already has `--stdio` flag in `McpServeArgs`

**Change:** When `args.stdio` is true, call `routes::run_stdio()` instead of HTTP server.

#### 1.3 Verify Authentication

**Note:** Authentication already exists in `auth.rs`:
- `validate_auth()` checks headers
- `validate_auth_params()` checks request params
- API key passed via CLI `--api-key`

**Verification:** Test that unauthenticated requests are rejected (except `initialize`).

#### 1.4 Testing

```bash
# HTTP mode
cargo run -- mcp-serve --port 8081 --api-key test-key

# STDIO mode (for testing)
echo '[]' | cargo run -- mcp-serve --stdio --api-key test-key
```

---

### Phase 2: Custom Agent Loop

**Goal:** Create an autonomous event-driven agent that monitors targets, executes scheduled scans, and alerts on findings.

#### 2.1 Agent Core (`agent/mod.rs`)

**New file:** `crates/slapper/src/agent/mod.rs`

```rust
pub struct Agent {
    registry: ToolRegistry,
    ai_client: Option<AiClient>,
    scheduler: CronScheduler,
    portfolio: TargetPortfolio,
    memory: LongitudinalMemory,
    alert_router: AlertRouter,
    event_handlers: Vec<Box<dyn EventHandler>>,
}

impl Agent {
    pub async fn run(&mut self) -> Result<()>;
    pub fn register_handler(&mut self, handler: Box<dyn EventHandler>);
    pub async fn execute_scan(&self, target: &str, scan_type: ScanType) -> Result<ToolResponse>;
    pub async fn analyze_findings(&self, findings: &[AgentFinding]) -> Result<AiAnalysisResult>;
}
```

#### 2.2 TargetPortfolio (`agent/portfolio.rs`)

**New file:** `crates/slapper/src/agent/portfolio.rs`

```rust
pub struct TargetPortfolio {
    targets: HashMap<String, TargetConfig>,
    memory_dir: PathBuf,
}

pub struct TargetConfig {
    target: String,
    target_type: TargetType,
    priority: Priority,
    schedule: Option<String>,  // cron expression
    alert_channels: Vec<AlertChannel>,
    last_scan: Option<DateTime<Utc>>,
    scan_history: Vec<ScanRecord>,
    baseline_findings: Vec<FindingId>,
}

pub struct ScanRecord {
    scan_id: String,
    scan_type: String,
    timestamp: DateTime<Utc>,
    findings_count: usize,
    severity_counts: HashMap<Severity, usize>,
}
```

**File location:** `~/.config/slapper/portfolio.json`

#### 2.3 Longitudinal Memory (`agent/memory.rs`)

**New file:** `crates/slapper/src/agent/memory.rs`

```rust
pub struct LongitudinalMemory {
    storage_dir: PathBuf,
    cache: AiCache,
}

impl LongitudinalMemory {
    pub fn new(storage_dir: PathBuf) -> Self;
    pub fn store_scan_results(&self, target: &str, results: &ToolResponse) -> Result<()>;
    pub fn get_target_history(&self, target: &str) -> Result<Vec<ScanRecord>>;
    pub fn compare_with_baseline(&self, target: &str, findings: &[AgentFinding]) -> Result<BaselineComparison>;
    pub fn detect_patterns(&self, target: &str) -> Result<Vec<Pattern>>;
    pub fn record_acknowledged_finding(&self, target: &str, finding_id: &str);
}
```

**File structure:**
```
~/.config/slapper/
├── memory/
│   ├── targets/
│   │   ├── example.com.json
│   │   ├── api.example.com.json
│   │   └── 192.168.1.0:8080.json
│   ├── patterns/
│   │   └── detected.json
│   └── baselines/
│       └── example.com_baseline.json
├── waf_bypasses.json
├── ai_cache.json
└── portfolio.json
```

#### 2.4 Alert Router (`agent/alerts.rs`)

**New file:** `crates/slapper/src/agent/alerts.rs`

```rust
pub struct AlertRouter {
    channels: Vec<AlertChannel>,
    rate_limiter: RateLimiter,
}

pub enum AlertChannel {
    Webhook(WebhookConfig),
    Email(EmailConfig),
    Slack(SlackConfig),
    PagerDuty(PagerDutyConfig),
}

pub struct Alert {
    severity: Severity,
    title: String,
    message: String,
    target: String,
    finding_id: Option<String>,
    recommended_actions: Vec<String>,
}
```

#### 2.5 Event System (`agent/events.rs`)

**New file:** `crates/slapper/src/agent/events.rs`

```rust
pub trait EventHandler: Send + Sync {
    fn event_type(&self) -> EventType;
    async fn handle(&self, event: &SecurityEvent, agent: &Agent) -> Result<()>;
}

pub enum EventType {
    ScanComplete,
    FindingDetected,
    ThresholdExceeded,
    ScheduleTriggered,
    ExternalWebhook,
    ManualTrigger,
}

pub enum SecurityEvent {
    ScanComplete(ScanCompleteEvent),
    FindingDetected(FindingDetectedEvent),
    ThresholdExceeded(ThresholdEvent),
    ScheduleTriggered(ScheduleEvent),
    ExternalWebhook(WebhookEvent),
}
```

#### 2.6 Agent CLI Command

**File:** `crates/slapper/src/cli/mod.rs`

```rust
#[cfg(feature = "rest-api")]
#[command(about = "Run autonomous security agent")]
AgentRun(AgentRunArgs),
```

**New file:** `crates/slapper/src/cli/agent.rs`

```rust
pub struct AgentRunArgs {
    #[arg(long, help = "Portfolio file path")]
    pub portfolio: Option<String>,
    #[arg(long, help = "AI provider (openai, ollama)")]
    pub ai_provider: Option<String>,
    #[arg(long, help = "API key for AI provider")]
    pub api_key: Option<String>,
    #[arg(long, help = "Base URL for AI API")]
    pub ai_base_url: Option<String>,
    #[arg(long, help = "Poll interval in seconds")]
    pub poll_interval: Option<u64>,
}
```

---

### Phase 3: Skill System

**Goal:** Enable loading of YAML+Markdown skill files that define agent capabilities.

#### 3.1 SkillLoader (`agent/skills.rs`)

**New file:** `crates/slapper/src/agent/skills.rs`

```rust
pub struct Skill {
    pub name: String,
    pub description: String,
    pub triggers: Vec<String>,
    pub metadata: SkillMetadata,
    pub content: String,
}

pub struct SkillMetadata {
    pub category: String,
    pub tools: Vec<String>,
    pub scope: String,
    pub requires: Option<String>,
}

pub struct SkillLoader {
    skill_dirs: Vec<PathBuf>,
}

impl SkillLoader {
    pub fn new(skill_dirs: Vec<PathBuf>) -> Self;
    pub fn load_skills(&self) -> Result<Vec<Skill>>;
    pub fn load_skill(&self, path: &Path) -> Result<Skill>;
    pub fn watch_for_changes(&self) -> Result<()>;
}

impl Skill {
    pub fn parse(content: &str) -> Result<Self>;
    pub fn matches_trigger(&self, input: &str) -> bool;
    pub fn to_prompt(&self) -> String;
}
```

**File format (YAML frontmatter + Markdown body):**

```yaml
---
name: slapper-recon
description: "Comprehensive reconnaissance for security assessment"
triggers:
  - recon
  - reconnaissance
  - dns
  - subdomain
  - whois
  - ssl
metadata:
  category: recon
  tools: [recon, scan-ports, scan-endpoints, fingerprint]
  scope: targets
---

## Overview
Performs comprehensive reconnaissance including DNS enumeration,...

## Capabilities
- DNS zone transfer testing
- Subdomain enumeration via multiple sources
- SSL/TLS certificate analysis
- Technology stack detection
- CVE correlation

## Usage
Invoke via MCP tools or CLI:
- `slapper recon <target>`
- Use `scan-ports` for port scanning
- Use `fingerprint` for service detection
```

#### 3.2 SkillRegistry

**Integration point:** `agent/mod.rs`

```rust
pub struct SkillRegistry {
    skills: HashMap<String, Skill>,
    skills_by_trigger: HashMap<String, Vec<String>>,
}

impl SkillRegistry {
    pub fn register(&mut self, skill: Skill) -> Result<()>;
    pub fn find_by_trigger(&self, trigger: &str) -> Vec<&Skill>;
    pub fn find_by_tool(&self, tool_id: &str) -> Vec<&Skill>;
    pub fn get_prompts_for_context(&self, context: &str) -> Vec<String>;
}
```

---

### Phase 4: Search Integration

**Goal:** Create a unified search tool that can query multiple sources for vulnerability research.

#### 4.1 SearchTool Implementation

**New file:** `crates/slapper/src/tool/implementations/search.rs`

```rust
#[derive(Clone)]
pub struct SearchTool {
    searxng_url: String,
    engines: Vec<SearchEngine>,
    cache: Arc<AiCache>,
}

pub enum SearchEngine {
    SearXNG(String),  // instance URL
    OSV,
    NVD,
    GitHubAdvisories,
    ExploitDB,
}

impl SecurityTool for SearchTool {
    fn id(&self) -> &'static str { "search" }
    fn name(&self) -> &'static str { "Web Search" }
    fn category(&self) -> ToolCategory { ToolCategory::Recon }
    fn description(&self) -> &'static str {
        "Search web, CVE databases, and security research sources"
    }
    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse>;
    fn capabilities(&self) -> Vec<ToolCapability>;
}
```

#### 4.2 Search Configuration

**File:** `crates/slapper/src/config/settings.rs`

```rust
pub struct SearchConfig {
    pub enabled: bool,
    pub searxng_url: Option<String>,
    pub engines: Vec<String>,
    pub cache_ttl_seconds: u64,
}
```

#### 4.3 Search Result Types

```rust
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
    pub relevance_score: Option<f32>,
}

pub struct CveSearchResult {
    pub cve_id: String,
    pub description: String,
    pub cvss_score: Option<f32>,
    pub cvss_vector: Option<String>,
    pub affected_products: Vec<String>,
    pub references: Vec<String>,
    pub published: DateTime<Utc>,
    pub last_modified: DateTime<Utc>,
}
```

---

### Phase 5: Alerting & Notification

**Goal:** Route alerts to security team via configured channels.

#### 5.1 Alert Integration with Agent

**File:** `crates/slapper/src/agent/alerts.rs`

```rust
impl Agent {
    pub async fn alert(&self, alert: Alert) -> Result<()> {
        // Check rate limits
        // Route to appropriate channels
        // Log alert to memory
        // Potentially trigger additional scans
    }
}
```

#### 5.2 Integration with Existing Webhook System

**Note:** Webhook system already exists in `commands/webhook.rs`. Reuse `send_webhook_notifications()`.

---

## File Structure

```
crates/slapper/src/
├── agent/
│   ├── mod.rs              # Agent core
│   ├── portfolio.rs        # TargetPortfolio
│   ├── memory.rs           # LongitudinalMemory
│   ├── alerts.rs           # AlertRouter
│   ├── events.rs           # Event system
│   ├── skills.rs           # SkillLoader
│   └── tests/
│       └── mod.rs
├── tool/
│   └── implementations/
│       ├── mod.rs          # Add SearchTool export
│       └── search.rs       # NEW: SearchTool
├── commands/
│   ├── handlers/
│   │   ├── mod.rs          # Add agent handler
│   │   └── agent.rs        # NEW: AgentRun handler
│   └── webhook.rs          # Reuse existing
├── cli/
│   ├── mod.rs              # Add AgentRun command
│   └── agent.rs            # NEW: AgentRunArgs
├── config/
│   └── settings.rs         # Add SearchConfig
└── skills/
    ├── README.md
    ├── recon.md
    ├── fuzzing.md
    ├── waf.md
    └── search.md
```

---

## Dependencies (Cargo.toml additions)

```toml
# Already present
axum = { version = "0.23", features = ["ws"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-stream = "0.3"

# New for Phase 2
notify = "7"                      # File watching for skill hot-reload
toml = "0.8"                      # Portfolio config parsing
chrono = { version = "0.4", features = ["serde"] }

# New for Phase 4
reqwest = { version = "0.12", features = ["json"] }
```

---

## Testing Strategy

### Unit Tests
- `cargo test --lib -p slapper` for core functionality
- Test skill parsing with various YAML+Markdown formats
- Test longitudinal memory storage/retrieval
- Test alert routing logic

### Integration Tests
- Test MCP server with Claude Desktop / other MCP clients
- Test STDIO mode with local AI process
- Test search tool with actual SearXNG instance

### Manual Testing
```bash
# Start MCP server
cargo run -- mcp-serve --port 8081 --api-key test-key

# Test with curl
curl -X POST http://localhost:8081/mcp \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer test-key" \
  -d '{"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}}'

# Test STDIO mode
echo '[{"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}]' | \
  cargo run -- mcp-serve --stdio --api-key test-key
```

---

## Checklist

- [ ] Phase 1.1: Wire up MCP HTTP server
- [ ] Phase 1.2: Implement MCP STDIO mode
- [ ] Phase 1.3: Verify authentication
- [ ] Phase 1.4: Test MCP server
- [ ] Phase 2.1: Create Agent core
- [ ] Phase 2.2: Implement TargetPortfolio
- [ ] Phase 2.3: Build LongitudinalMemory
- [ ] Phase 2.4: Implement AlertRouter
- [ ] Phase 2.5: Create Event system
- [ ] Phase 2.6: Add Agent CLI command
- [ ] Phase 3.1: Create SkillLoader
- [ ] Phase 3.2: Build SkillRegistry
- [ ] Phase 4.1: Implement SearchTool
- [ ] Phase 4.2: Add SearchConfig
- [ ] Phase 4.3: Search result types
- [ ] Phase 5.1: Alert integration
- [ ] Phase 5.2: Webhook reuse
- [ ] Update AGENTS.md with agent documentation
