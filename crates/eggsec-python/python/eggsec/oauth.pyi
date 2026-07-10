"""Type stubs for OAuth/OIDC security assessment module."""

from typing import List, Optional

class OAuthVulnerability:
    """OAuth vulnerability types."""
    RedirectUriValidation: str
    StateParameterMissing: str
    ScopeEscalation: str
    GrantTypeMixing: str
    PKCEBypass: str
    TokenLeakage: str

class OAuthEndpointKind:
    """OAuth endpoint types."""
    OidcDiscovery: str
    OAuthDiscovery: str
    Authorize: str
    Token: str
    UserInfo: str
    Jwks: str
    Revoke: str

class OAuthEndpoint:
    """An OAuth/OIDC endpoint."""
    url: str
    kind: str

class OAuthTestResult:
    """Result from an OAuth security test."""
    vulnerability: str
    success: bool
    endpoint: str
    proof: str
    severity: str
    description: str

class OAuthTestConfig:
    """Configuration for OAuth/OIDC security testing."""
    client_id: str
    redirect_uri: str
    issuer_url: Optional[str]
    client_secret: Optional[str]
    test_redirect: bool
    test_scope: bool
    test_state: bool
    test_grant: bool
    timeout_secs: int
    def __init__(
        self,
        client_id: str,
        redirect_uri: str,
        *,
        issuer_url: Optional[str] = None,
        client_secret: Optional[str] = None,
        test_redirect: bool = True,
        test_scope: bool = True,
        test_state: bool = True,
        test_grant: bool = True,
        timeout_secs: int = 30,
    ) -> None: ...

def oauth_discover_endpoints(issuer: str) -> List[OAuthEndpoint]:
    """Discover OAuth/OIDC endpoints from an issuer URL."""
    ...

def oauth_test(config: OAuthTestConfig) -> List[OAuthTestResult]:
    """Run OAuth/OIDC security tests."""
    ...

async def async_oauth_test(config: OAuthTestConfig) -> List[OAuthTestResult]:
    """Run OAuth/OIDC security tests (async)."""
    ...
