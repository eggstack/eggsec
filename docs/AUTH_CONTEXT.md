# Auth Context Configuration

Auth contexts allow testing with multiple user roles.

## File Format

```yaml
version: 1
contexts:
  user:
    description: "Normal user"
    headers:
      Authorization: "Bearer ${USER_TOKEN}"
    cookies:
      session: "${SESSION_COOKIE:-none}"
  admin:
    description: "Admin user"
    headers:
      Authorization: "Bearer ${ADMIN_TOKEN}"
```

The file is parsed as YAML into `AuthContextFile` with
`deny_unknown_fields`, so unknown keys are a hard error rather than being
silently dropped (`crates/eggsec/src/auth_context/mod.rs`). Each entry
supports `description`, `headers`, and `cookies`.

## Environment Variable Interpolation

- `${VAR}` - Required variable, fails if not set
- `${VAR:-default}` - Variable with default value

Interpolated values are resolved when the context is loaded, so prefer
`${VAR}` for secrets and avoid committing files with literal tokens.

## Usage

```bash
# Set tokens
export USER_TOKEN="user-jwt-token"
export ADMIN_TOKEN="admin-jwt-token"

# Use auth context
eggsec fuzz https://api.example.com/users/123 \
  --auth-context auth-context.yaml \
  --auth-role user
```

## Security

- Never commit auth context files with real tokens
- Use environment variable interpolation
- Evidence is redacted in reports by default
