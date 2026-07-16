"""Managed session types for eggsec.

This submodule contains browser, mobile, database, proxy, and capture
session lifecycle types.

Maturity: provisional (session contracts)
"""

# Session contract (always available)
try:
    from .._core import (
        SessionState,
        SessionIdentity,
        SessionStats,
        SessionCloseMode,
        SessionEvent,
        SessionEventStream,
        SessionCapabilities,
        create_session_event,
    )
except (AttributeError, ImportError):
    pass

# Mobile sessions (feature-gated: mobile)
try:
    from .._core import (
        MobileDeviceDescriptor,
        MobileDeviceCapabilities,
        MobileSessionConfig,
        MobileSessionState,
        MobileSessionStats,
        MobileSession,
        AsyncMobileSession,
        MobileDeviceRegistry,
        StaticAnalysisSummary,
        AnalysisTarget,
        DynamicAnalysisPlan,
        InstrumentationConfig,
        InstrumentationScript,
        InstrumentationEvent,
        InstrumentationResult,
        MobileEvidenceKind,
        MobileEvidence,
        MobileEvidenceCollection,
    )
except (AttributeError, ImportError):
    pass

# Browser sessions (feature-gated: headless-browser)
try:
    from .._core import (
        BrowserCapabilities,
        BrowserSessionState,
        BrowserSessionConfig,
        BrowserSessionStats,
        BrowserNavigationEvent,
        BrowserConsoleEvent,
        BrowserNetworkEvent,
        BrowserDomSnapshot,
        BrowserFormInfo,
        BrowserFormField,
        BrowserLinkInfo,
        BrowserStorageInfo,
        BrowserCookieInfo,
        BrowserSession,
        AsyncBrowserSession,
        BrowserDomEvent,
        BrowserDownloadEvent,
        BrowserSecurityObservation,
    )
except (AttributeError, ImportError):
    pass

# Database sessions (feature-gated: db-pentest)
try:
    from .._core import (
        DbDriverInfoPy as DbDriverInfo,
        DbCapabilityPy as DbCapability,
        DbCredentialProviderPy as DbCredentialProvider,
        DbSessionConfigPy as DbSessionConfig,
        DbDriverRegistryPy as DbDriverRegistry,
        DbTargetPy as DbTarget,
        DatabaseSessionStatePy as DatabaseSessionState,
        DatabaseConnectionMetadataPy as DatabaseConnectionMetadata,
        DatabaseSessionStatsPy as DatabaseSessionStats,
        DatabaseCredentialRequestPy as DatabaseCredentialRequest,
        DatabaseCredentialResultPy as DatabaseCredentialResult,
        DatabaseQueryPy as DatabaseQuery,
        DatabaseQueryResultPy as DatabaseQueryResult,
        DatabaseColumnPy as DatabaseColumn,
        DatabaseTableInfoPy as DatabaseTableInfo,
        DatabaseSchemaInfoPy as DatabaseSchemaInfo,
        DatabasePrivilegeInfoPy as DatabasePrivilegeInfo,
        StaticCredentialProviderPy as StaticCredentialProvider,
        EnvironmentCredentialProviderPy as EnvironmentCredentialProvider,
        CallbackCredentialProviderPy as CallbackCredentialProvider,
        DatabaseRowStreamPy as DatabaseRowStream,
        DatabaseQueryPlanPy as DatabaseQueryPlan,
        DatabaseIndexInfoPy as DatabaseIndexInfo,
        DatabaseExtensionInfoPy as DatabaseExtensionInfo,
    )
except (AttributeError, ImportError):
    pass

# Proxy sessions (feature-gated: web-proxy)
try:
    from .._core import (
        ProxyTypePy as ProxyType,
        RotationStrategyPy as RotationStrategy,
        ProxyConfigPy as ProxyConfig,
        ProxyEntryPy as ProxyEntry,
        ProxyManagerPy as ProxyManager,
        HealthCheckResultPy as HealthCheckResult,
        ProxyHealthPy as ProxyHealth,
        InterceptConfigPy as InterceptConfig,
        CapturedExchangePy as CapturedExchange,
        InterceptSessionResultPy as InterceptSessionResult,
        InterceptSessionStatePy as InterceptSessionState,
        InterceptStatsPy as InterceptStats,
        InterceptFilterPy as InterceptFilter,
        InterceptRulePy as InterceptRule,
        CertificateAuthorityConfigPy as CertificateAuthorityConfig,
        IssuedCertificatePy as IssuedCertificate,
        HarEntryPy as HarEntry,
        HarDocumentPy as HarDocument,
        MutationDecisionPy as MutationDecision,
        MutationErrorPy as MutationError,
        CertificateAuthorityPy as CertificateAuthority,
        CertificateStorePy as CertificateStore,
        ReplayRequestPy as ReplayRequest,
        ReplayResultPy as ReplayResult,
        ResponseComparisonPy as ResponseComparison,
        ComparisonRulePy as ComparisonRule,
        run_intercept_session,
        async_run_intercept_session,
    )
except (AttributeError, ImportError):
    pass
