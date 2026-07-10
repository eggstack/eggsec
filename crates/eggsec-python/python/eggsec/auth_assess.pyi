"""Type stubs for authentication assessment module."""

from typing import List, Optional

class AuthTestType:
    """Authentication test types."""
    BruteForce: str
    CredentialStuffing: str
    AccountLockout: str
    RateLimitBypass: str
    MfaBypass: str
    SessionFixation: str
    TimingAttack: str
    PasswordPolicy: str

class AuthFinding:
    """A finding from authentication testing."""
    test_type: str
    severity: str
    title: str
    description: str
    recommendation: str

class AuthTestConfig:
    """Configuration for authentication testing."""
    target: str
    max_attempts: int
    concurrency: int
    timeout_secs: int
    stop_on_lockout: bool
    usernames: Optional[List[str]]
    passwords: Optional[List[str]]
    def __init__(
        self,
        target: str,
        *,
        max_attempts: int = 100,
        concurrency: int = 10,
        timeout_secs: int = 30,
        stop_on_lockout: bool = True,
        usernames: Optional[List[str]] = None,
        passwords: Optional[List[str]] = None,
    ) -> None: ...

class AuthTestReport:
    """Complete authentication test report."""
    target: str
    tests_run: int
    brute_force: Optional[dict]
    credential_stuffing: Optional[dict]
    lockout_detection: Optional[dict]
    rate_limit: Optional[dict]
    mfa: Optional[dict]
    session: Optional[dict]
    timing: Optional[dict]
    password_policy: Optional[dict]
    total_attempts: int
    findings: List[AuthFinding]

def auth_test(config: AuthTestConfig) -> AuthTestReport:
    """Run authentication security tests."""
    ...

async def async_auth_test(config: AuthTestConfig) -> AuthTestReport:
    """Run authentication security tests (async)."""
    ...
