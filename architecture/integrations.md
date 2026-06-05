# Integrations Module

## Purpose

Issue tracker connectors for Jira, GitHub, and GitLab. Provides a common `IssueTracker` trait for creating, updating, searching, and commenting on issues in external trackers.

## Key Types

| Type | Location | Description |
|------|----------|-------------|
| `IntegrationConfig` | `integrations/mod.rs` | Top-level config holding optional Jira/GitHub/GitLab configs |
| `IssueTracker` | `integrations/mod.rs` | Trait: `create_issue`, `update_issue`, `add_comment`, `get_issue`, `search_issues` |
| `Issue` | `integrations/mod.rs` | Universal issue representation (id, title, description, labels, severity, assignees, status, url, created_at) |
| `IssueUpdate` | `integrations/mod.rs` | Partial update payload for issues |
| `JiraConfig` | `integrations/jira.rs` | Jira connection configuration |
| `GitHubConfig` | `integrations/github.rs` | GitHub connection configuration |
| `GitLabConfig` | `integrations/gitlab.rs` | GitLab connection configuration |

## Files

| File | Description |
|------|-------------|
| `mod.rs` | Module root: `IssueTracker` trait, `Issue`, `IssueUpdate`, `IntegrationConfig` |
| `common.rs` | Shared utilities and types for all integrations |
| `jira.rs` | Jira REST API client implementation |
| `github.rs` | GitHub Issues API client implementation |
| `gitlab.rs` | GitLab Issues API client implementation |

## Implementation Status

Fully implemented. All three tracker backends implement the `IssueTracker` trait. Each provides config structs and API client logic.
