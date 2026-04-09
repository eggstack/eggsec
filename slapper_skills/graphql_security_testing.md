---
name: graphql_security_testing
description: "GraphQL API security testing for introspection, injection, and authorization issues"
triggers:
  - graphql
  - graphQL
  - introspection
  - query
  - mutation
  - subscription
  - schema
  - batching
  - alias
metadata:
  category: api_testing
  tools: [fuzzer]
  scope: targets
---

## Overview

GraphQL security testing discovers vulnerabilities specific to GraphQL APIs including excessive introspection, injection attacks, rate limiting bypass, and authorization flaws.

## Capabilities

- GraphQL introspection querying
- Schema enumeration
- Query depth limiting bypass
- Alias-based rate limit bypass
- Batching attacks (user enumeration)
- Injection via query variables
- Directive injection
- Mutation testing
- Subscription DoS testing
- Field cost analysis

## Usage

### Basic GraphQL Test

```bash
slapper graphql --target https://api.example.com/graphql
```

### Introspection Query

```bash
slapper graphql --target https://api.example.com/graphql --introspect
```

### Test Query Depth

```bash
slapper graphql --target https://api.example.com/graphql --depth 10
```

### Mutation Testing

```bash
slapper graphql --target https://api.example.com/graphql --test-mutations
```

## Common Vulnerabilities

| Issue | Description | Severity |
|-------|-------------|----------|
| Introspection Enabled | Schema exposed publicly | Medium |
| No Depth Limiting | DoS via deeply nested queries | High |
| No Rate Limiting | User enumeration via batching | Medium |
| IDOR | Access other users' data | Critical |
| Batch Attack | Brute force via query batching | Medium |

## Payloads

**Introspection Query:**
```graphql
{ __schema { types { name } } }
```

**Depth Attack:**
```graphql
query { user { friends { friends { friends ... } } } }
```

**Alias Bypass:**
```graphql
{ a1: user(id: 1) { name } a2: user(id: 2) { name } ... }
```

## Triggers

Keywords: graphql, graphQL, api, introspection, query, mutation, subscription, schema, alias, batching, depth, injection, security, test

## Best Practices

1. Always check if introspection is enabled in production
2. Test for depth limiting to prevent DoS attacks
3. Use batching attacks to enumerate users
4. Test all mutations for authorization issues
5. Look for field-level rate limiting implementations