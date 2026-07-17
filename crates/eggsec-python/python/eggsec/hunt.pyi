"""Type stubs for advanced vulnerability hunting module."""

from typing import List, Optional

class ChainType:
    """Attack chain types."""
    PrivilegeEscalation: str
    DataExfiltration: str
    RemoteCodeExecution: str
    LateralMovement: str
    Persistence: str
    DenialOfService: str
    def __hash__(self) -> int: ...

class ChainStep:
    """A step in an attack chain."""
    step_number: int
    technique: str
    description: str
    endpoint: Optional[str]

class AttackChain:
    """A multi-step attack chain."""
    id: str
    name: str
    chain_type: str
    steps: List[ChainStep]
    severity: str
    description: str
    remediation: str
    cvss_score: Optional[float]

class FlawType:
    """Business logic flaw types."""
    PriceManipulation: str
    PrivilegeEscalation: str
    RateLimitBypass: str
    CartManipulation: str
    CreditOverflow: str
    WorkflowBypass: str
    InsufficientValidation: str
    TrustBoundaryViolation: str
    TimeTravel: str
    IntegerOverflow: str
    def __hash__(self) -> int: ...

class BusinessLogicFlaw:
    """A business logic flaw."""
    id: str
    flaw_type: str
    severity: str
    description: str
    location: str
    evidence: str
    remediation: str
    cvss_score: Optional[float]

class RaceType:
    """Race condition types."""
    TimeOfCheckTimeOfUse: str
    ConcurrentFundsTransfer: str
    InventoryOverSale: str
    SessionRace: str
    CouponReuse: str
    CommentRace: str
    ResponseInconsistency: str
    TimingAnomaly: str
    def __hash__(self) -> int: ...

class RaceCondition:
    """A race condition finding."""
    id: str
    race_type: str
    severity: str
    description: str
    endpoint: str
    evidence: str
    remediation: str
    cvss_score: Optional[float]

class BypassType:
    """Authorization bypass types."""
    Idor: str
    MissingAuthorization: str
    PrivilegeEscalation: str
    ForceBrowsing: str
    ApiKeyLeak: str
    JWTBypass: str
    RoleManipulation: str
    def __hash__(self) -> int: ...

class AuthzBypass:
    """An authorization bypass finding."""
    id: str
    bypass_type: str
    severity: str
    description: str
    endpoint: str
    evidence: str
    remediation: str
    cvss_score: Optional[float]

class SessionIssueType:
    """Session issue types."""
    SessionFixation: str
    SessionTimeout: str
    TokenPrediction: str
    InsufficientEntropy: str
    MissingHttpOnly: str
    MissingSecure: str
    MissingSameSite: str
    Csrf: str
    ConcurrentSessions: str
    def __hash__(self) -> int: ...

class SessionIssue:
    """A session security issue."""
    id: str
    issue_type: str
    severity: str
    description: str
    evidence: str
    remediation: str
    cvss_score: Optional[float]

class HuntTestConfig:
    """Configuration for advanced vulnerability hunting."""
    check_attack_chains: bool
    check_business_logic: bool
    check_race_conditions: bool
    check_authz_bypass: bool
    check_session: bool
    concurrency: int
    timeout_ms: int
    def __init__(
        self,
        *,
        check_attack_chains: bool = True,
        check_business_logic: bool = True,
        check_race_conditions: bool = True,
        check_authz_bypass: bool = True,
        check_session: bool = True,
        concurrency: int = 10,
        timeout_ms: int = 10000,
    ) -> None: ...

class HuntReport:
    """Complete advanced hunting report."""
    target: str
    attack_chains: List[AttackChain]
    business_logic: List[BusinessLogicFlaw]
    race_conditions: List[RaceCondition]
    authz_bypasses: List[AuthzBypass]
    session_issues: List[SessionIssue]
    total_findings: int

def hunt_test(
    target: str, config: Optional[HuntTestConfig] = None
) -> HuntReport:
    """Run advanced vulnerability hunting."""
    ...

async def async_hunt_test(
    target: str, config: Optional[HuntTestConfig] = None
) -> HuntReport:
    """Run advanced vulnerability hunting (async)."""
    ...
