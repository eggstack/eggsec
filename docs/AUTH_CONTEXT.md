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
  admin:
    description: "Admin user"
    headers:
      Authorization: "Bearer ${ADMIN_TOKEN}"
```

## Environment Variable Interpolation

- `${VAR}` - Required variable, fails if not set
- `${VAR:-default}` - Variable with default value

## Usage

```bash
# Set tokens
export USER_TOKEN="user-jwt-token"
export ADMIN_TOKEN="admin-jwt-token"

# Use auth context
slapper fuzz https://api.example.com/users/123 \
  --auth-context auth-context.yaml \
  --auth-role user
```

## Security

- Never commit auth context files with real tokens
- Use environment variable interpolation
- Evidence is redacted in reports by default
