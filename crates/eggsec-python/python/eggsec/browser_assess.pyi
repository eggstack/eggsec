"""Type stubs for headless browser security assessment module."""

from typing import List, Optional

class XssSource:
    """DOM XSS source types."""
    URLParameter: str
    HashFragment: str
    PostMessage: str
    LocalStorage: str
    SessionStorage: str
    Cookie: str
    DocumentReferrer: str
    WindowName: str

class XssSink:
    """DOM XSS sink types."""
    Eval: str
    InnerHTML: str
    DocumentWrite: str
    Location: str
    WindowOpen: str
    setTimeout: str
    setInterval: str
    Function: str
    Attribute: str
    Style: str

class DomXssFinding:
    """A DOM XSS finding."""
    id: str
    source: str
    sink: str
    location: str
    severity: str
    description: str
    evidence: str
    remediation: str
    cvss_score: Optional[float]

class DiscoveryMethod:
    """SPA route discovery methods."""
    HistoryAPI: str
    Fragment: str
    AJAXIntercept: str
    StaticAnalysis: str

class SpaRoute:
    """A discovered SPA route."""
    path: str
    method: str
    parameters: List[str]
    discovered_via: str

class ClientIssueType:
    """Client-side issue types."""
    MissingCSP: str
    InsecureDependency: str
    SensitiveDataExposure: str
    MissingSubresourceIntegrity: str
    InsecureWebStorage: str
    CORSMisconfiguration: str

class ClientIssue:
    """A client-side security issue."""
    id: str
    issue_type: str
    severity: str
    location: str
    description: str
    evidence: str
    remediation: str
    cvss_score: Optional[float]

class BrowserTestConfig:
    """Configuration for headless browser testing."""
    check_dom_xss: bool
    discover_spa_routes: bool
    check_client_security: bool
    timeout_ms: int
    xss_payload: str
    def __init__(
        self,
        *,
        check_dom_xss: bool = True,
        discover_spa_routes: bool = True,
        check_client_security: bool = True,
        timeout_ms: int = 30000,
        xss_payload: str = '<img src=x onerror=alert(1)>',
    ) -> None: ...

class BrowserTestReport:
    """Complete browser security test report."""
    target: str
    dom_xss: List[DomXssFinding]
    spa_routes: List[SpaRoute]
    client_issues: List[ClientIssue]
    total_findings: int

def browser_test(
    target: str, config: Optional[BrowserTestConfig] = None
) -> BrowserTestReport:
    """Run headless browser security testing."""
    ...

async def async_browser_test(
    target: str, config: Optional[BrowserTestConfig] = None
) -> BrowserTestReport:
    """Run headless browser security testing (async)."""
    ...
