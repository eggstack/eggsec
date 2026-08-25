# Integrations Module

## Purpose

Issue tracker connectors for Jira, GitHub, and GitLab. Provides a common `IssueTracker` trait for creating, updating, searching, and commenting on issues in external trackers. Feature-gated: `external-integrations`.

## Location & Feature Gating

| Path | Feature Gate |
|------|-------------|
| `crates/eggsec/src/integrations/mod.rs` | `external-integrations` (`lib.rs:116`) |
| `crates/eggsec/src/integrations/common.rs` | `external-integrations` |
| `crates/eggsec/src/integrations/jira.rs` | `external-integrations` |
| `crates/eggsec/src/integrations/github.rs` | `external-integrations` |
| `crates/eggsec/src/integrations/gitlab.rs` | `external-integrations` |

When the feature is disabled, `lib.rs:118-120` compiles the module as `#[allow(dead_code)] mod integrations` (private, unused). Config integration is gated at `config/settings.rs:136-138`: `pub integrations: IntegrationConfig` is only present with the feature enabled.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `IntegrationConfig` | `integrations/mod.rs:20` | Top-level config: `Option<JiraConfig>`, `Option<GitHubConfig>`, `Option<GitLabConfig>` |
| `IssueTracker` | `integrations/mod.rs:27` | Async trait: `create_issue`, `update_issue`, `add_comment`, `get_issue`, `search_issues` |
| `Issue` | `integrations/mod.rs:36` | Universal issue: id, title, description, labels, severity, assignees, status, url, created_at |
| `IssueUpdate` | `integrations/mod.rs:49` | Partial update: title, description, status, labels (all `Option`) |
| `JiraConfig` | `integrations/jira.rs:8` | Jira: url, username, api_token (`SensitiveString`), project_key |
| `GitHubConfig` | `integrations/github.rs:8` | GitHub: owner, repo, api_token (`SensitiveString`) |
| `GitLabConfig` | `integrations/gitlab.rs:8` | GitLab: url, project_id, api_token (`SensitiveString`) |
| `JiraClient` | `integrations/jira.rs:15` | Jira REST API client |
| `GitHubClient` | `integrations/github.rs:14` | GitHub Issues API client |
| `GitLabClient` | `integrations/gitlab.rs:14` | GitLab Issues API client |

## Architecture

### File structure

| File | Lines | Role |
|------|-------|------|
| `integrations/mod.rs` | 77 | `IntegrationConfig`, `IssueTracker` trait, `Issue`, `IssueUpdate`, tests |
| `integrations/common.rs` | 159 | `send_with_retry`, `handle_response_error`, `truncate_utf8`, tests |
| `integrations/jira.rs` | 416 | `JiraClient`, Jira REST API v3, issue parsing, transition logic, tests |
| `integrations/github.rs` | 298 | `GitHubClient`, GitHub Issues API, issue parsing, tests |
| `integrations/gitlab.rs` | 282 | `GitLabClient`, GitLab Issues API v4, issue parsing, tests |

### IssueTracker trait (`integrations/mod.rs:27-34`)

```rust
#[async_trait]
pub trait IssueTracker: Send + Sync {
    async fn create_issue(&self, issue: &Issue) -> Result<String>;
    async fn update_issue(&self, id: &str, update: &IssueUpdate) -> Result<()>;
    async fn add_comment(&self, issue_id: &str, comment: &str) -> Result<()>;
    async fn get_issue(&self, id: &str) -> Result<Issue>;
    async fn search_issues(&self, query: &str) -> Result<Vec<Issue>>;
}
```

### Retry logic (`common.rs:41-103`)

Shared across all three backends via `send_with_retry()`:
- **Max 3 retries** (`MAX_RETRIES = 3`, line 5).
- **Exponential backoff**: 500ms base (`BASE_BACKOFF_MS = 500`, line 6), doubled per attempt.
- **Respects `Retry-After` header**: Parsed as seconds, used as backoff when present (`common.rs:54-59`).
- **Retried on**: HTTP 429 (rate limit), 5xx (server errors), network errors.
- **Not retried on**: 4xx (except 429), serialization errors.
- **Error body truncation**: `truncate_utf8()` (`common.rs:26-39`) limits error body logging to 200 bytes (`MAX_ERROR_BODY_LEN`, line 4).

### API endpoints

| Backend | Base URL | Endpoints Used |
|---------|----------|---------------|
| Jira | `{config.url}/rest/api/3/` | `issue` (GET/POST/PUT), `issue/{id}/transitions` (GET/POST), `issue/{id}/comment` (POST), `search?jql=` (GET) |
| GitHub | `https://api.github.com/repos/{owner}/{repo}` | `issues` (GET/POST/PATCH), `issues/{number}` (GET/PATCH), `issues/{number}/comments` (POST) |
| GitHub Search | `https://api.github.com/search/issues` | `?q=repo:{owner}/{repo} {query}` (GET) |
| GitLab | `{config.url}/api/v4/projects/{project_id}` | `issues` (GET/POST/PUT), `issues/{iid}` (GET/PUT), `issues/{iid}/notes` (POST) |

## Behavior & Flow

### Issue creation payloads

**Jira** (`jira.rs:176-230`):
```json
{ "fields": {
    "project": { "key": "<project_key>" },
    "summary": "<title>",
    "description": { "type": "doc", "version": 1, "content": [{
        "type": "paragraph", "content": [{ "type": "text", "text": "<description>" }]
    }]},
    "issuetype": { "name": "Task" },
    "labels": ["<labels>"],
    "priority": { "name": "<severity>" }
}}
```
Severity mapping: Critical/Blocker/Highest → "Critical", High → "High", Medium/Major/Normal → "Medium", Low/Minor/Trivial → "Low". Uses Jira Atlassian Document Format for description.

**GitHub** (`github.rs:96-138`):
```json
{ "title": "<title>", "body": "<description>", "labels": ["<labels>"] }
```
Returns `#{number}` format. Bearer token auth. GitHub API version header: `2022-11-28`.

**GitLab** (`gitlab.rs:96-133`):
```json
{ "title": "<title>", "description": "<description>", "labels": "<labels>" }
```
Returns `!{iid}` format. `PRIVATE-TOKEN` header auth.

### Issue update payloads

**Jira** (`jira.rs:232-278`): Updates fields via PUT, then transitions via `transition_issue()` if status is provided. Transition matching is case-insensitive (`jira.rs:55-64`).

**GitHub** (`github.rs:140-173`): PATCH with title/body/labels/state. State is passed directly as-is.

**GitLab** (`gitlab.rs:135-173`): PUT with title/description/labels + `state_event`. State mapping: `closed`/`close` → `"close"`, `opened`/`open`/`reopen`/`reopened` → `"reopen"`.

### Search pagination

| Backend | Page Size | Max Pages | Max Results |
|---------|-----------|-----------|-------------|
| Jira | 100 (`jira.rs:331`) | N/A (total-based) | 1000 (`jira.rs:332`) |
| GitHub | 100 (`github.rs:225`) | 10 (`github.rs:226`) | 1000 |
| GitLab | 20 (`gitlab.rs:215`) | 50 (`gitlab.rs:216`) | 1000 |

### Issue parsing

Each backend has a `parse_issue()` method that maps provider-specific JSON to the universal `Issue` type:
- **Jira** (`jira.rs:98-171`): Parses `fields.description.content[0].content[0].text` (Atlassian Document Format) with fallback to `fields.description` as string. Maps `priority.name` to `Severity`.
- **GitHub** (`github.rs:43-91`): Parses `labels[].name`, `assignees[].login`, `state`, `number`, `html_url`. Severity is always `None` (GitHub issues have no severity).
- **GitLab** (`gitlab.rs:43-91`): Parses `labels[]` (strings), `assignees[].username`, `state`, `iid`, `web_url`. Severity is always `None`.

## Credential Handling

| Backend | Auth Method | Token Type | Location |
|---------|------------|------------|----------|
| Jira | HTTP Basic Auth | `api_token: SensitiveString` | `jira.rs:11`, sent via `basic_auth()` at lines 38, 88, 208, 264, 300, 314, 343 |
| GitHub | Bearer Token | `api_token: SensitiveString` | `github.rs:11`, sent via `Authorization: Bearer {token}` at lines 114-117, 162-164, 188-190, 208-210, 241-243 |
| GitLab | Private Token | `api_token: SensitiveString` | `gitlab.rs:11`, sent via `PRIVATE-TOKEN: {token}` at lines 114, 167, 187, 201, 229 |

All API tokens use `SensitiveString` and are accessed only via `expose_secret()` at the point of HTTP request construction. Credentials are loaded from `IntegrationConfig` in `EggsecConfig` (`config/settings.rs:136-138`). No environment variable fallback — config file only.

## Public API

### Trait methods

| Method | Signature | Returns |
|--------|-----------|---------|
| `create_issue` | `async (&self, &Issue) -> Result<String>` | Issue ID (Jira: key, GitHub: `#number`, GitLab: `!iid`) |
| `update_issue` | `async (&self, &str, &IssueUpdate) -> Result<()>` | Unit on success |
| `add_comment` | `async (&self, &str, &str) -> Result<()>` | Unit on success |
| `get_issue` | `async (&self, &str) -> Result<Issue>` | Parsed `Issue` |
| `search_issues` | `async (&self, &str) -> Result<Vec<Issue>>` | Parsed issues |

### Client constructors

| Constructor | Signature |
|------------|-----------|
| `JiraClient::new` | `(JiraConfig) -> Self` |
| `GitHubClient::new` | `(GitHubConfig) -> Self` |
| `GitLabClient::new` | `(GitLabConfig) -> Self` |

All constructors build a `reqwest::Client` with 30-second timeout, falling back to default client on builder error.

## Integration Points

### Dispatch (`dispatch/mod.rs:289-299`, `dispatch/security.rs:330-434`)

`TaskKind::Integrations` dispatches to `run_integrations_task()` which:
1. Selects tracker from `IntegrationConfig` (Jira > GitHub > GitLab priority).
2. Dispatches by `mode` string: `"configure"` → `TaskResult::Integrations`, `"create_issue"` → `TaskResult::IntegrationsCreateIssue`, `"search_issues"` → `TaskResult::IntegrationsSearchIssues`.
3. 60-second timeout wrapper.

### Dispatch result types (`dispatch/types.rs:130-139`)

```rust
#[cfg(feature = "external-integrations")]
TaskResult::Integrations,                           // configure/list mode
TaskResult::IntegrationsCreateIssue { issue: Issue }, // created issue with ID
TaskResult::IntegrationsSearchIssues { issues: Vec<Issue> }, // search results
```

### TUI

- **Integrations tab** (`tabs/integrations.rs`): Gated behind `external-integrations` feature (`tabs/mod.rs:17,81`).
- **Tab spec** (`tabs/spec.rs:769`): Present when feature enabled.
- **Task dispatch** (`app/task_dispatcher.rs:174-180`): Handles `Integrations`, `IntegrationsCreateIssue`, `IntegrationsSearchIssues` results.
- **State update** (`app/state_update.rs:391-414`): Updates integrations tab state with results.
- **Settings** (`tabs/settings/main.rs:569-570, 1610, 1690`): Integration config fields in settings.

### Config

`IntegrationConfig` is embedded in `EggsecConfig` at `config/settings.rs:136-138`, feature-gated. Only present when `external-integrations` is enabled.

## Testing

- **Unit tests** (`integrations/mod.rs:57-77`): 1 test for `Issue` creation.
- **Unit tests** (`integrations/common.rs:105-159`): 7 tests for `truncate_utf8` (boundary, multibyte, 4-byte, empty, zero-max).
- **Unit tests** (`integrations/jira.rs:373-416`): 2 tests for client config and transition matching.
- **Unit tests** (`integrations/github.rs:271-298`): 2 tests for client config and API URL construction.
- **Unit tests** (`integrations/gitlab.rs:255-282`): 2 tests for client config and API URL construction.
- **Total**: 14 tests. No integration tests (all tests are unit-level, no live API calls).

## Invariants & Gotchas

1. **Tracker selection priority**: When multiple backends are configured in `IntegrationConfig`, Jira takes precedence over GitHub, which takes precedence over GitLab (`security.rs:346-358`). Only one tracker is used per dispatch.
2. **Issue ID format varies**: Jira returns a project key (e.g., `SEC-123`), GitHub returns `#123`, GitLab returns `!123`. The `update_issue` and `add_comment` methods accept string IDs and strip prefixes (`trim_start_matches('#')`, `trim_start_matches('!')`).
3. **Jira transition matching**: `transition_issue()` matches target status case-insensitively against available transitions (`jira.rs:55-64`). If no match is found, returns an error with available transition names.
4. **GitLab state events**: GitLab uses `state_event` field with values `"close"` or `"reopen"` — not arbitrary status strings (`gitlab.rs:151-162`).
5. **Search pagination limits**: Each backend caps results differently (Jira: 1000, GitHub: 1000, GitLab: 1000). Large result sets may be truncated.
6. **No severity mapping for GitHub/GitLab**: These backends always set `severity: None` on parsed issues (`github.rs:85`, `gitlab.rs:85`).
7. **HTTP client builder fallback**: All three clients fall back to `reqwest::Client::new()` on builder error, losing the 30-second timeout (`jira.rs:27-29`, `github.rs:27-29`, `gitlab.rs:27-29`).

## Security Considerations

- **Credentials are `SensitiveString`**: All three backend API tokens (`JiraConfig.api_token`, `GitHubConfig.api_token`, `GitLabConfig.api_token`) use `SensitiveString` and are accessed via `expose_secret()` only at request construction time.
- **No environment variable fallback**: Tokens are loaded exclusively from `IntegrationConfig` in the config file. There is no `EGGSEC_JIRA_TOKEN` or similar env var support.
- **URL construction**: GitHub uses hardcoded `https://api.github.com` base (`github.rs:36`). Jira and GitLab use user-configured base URLs — SSRF consideration if the URL points to internal infrastructure.
- **Error body truncation**: API error responses are logged but truncated to 200 bytes (`common.rs:4,17`) to avoid log flooding.

## Bug Sweep

| Finding | Location | Severity | Description |
|---------|----------|----------|-------------|
| HTTP client builder fallback loses timeout | `jira.rs:27-29`, `github.rs:27-29`, `gitlab.rs:27-29` | Low | On `Client::builder().build()` failure, falls back to `Client::new()` which has no timeout. Could hang on slow connections. |
| GitLab create_issue labels type mismatch | `gitlab.rs:108` | Low | `labels` field is sent as `serde_json::json!(issue.labels)` (JSON array), but GitLab API expects a comma-separated string for the `labels` field on issue creation. This may cause label creation to fail silently. |
| GitHub search scoped query encoding | `github.rs:229-234` | Low | The scoped query `repo:{owner}/{repo} {query}` is URL-encoded as a single parameter. Complex queries with spaces may not parse correctly. |

*Last verified against source: 2026-08-25*
