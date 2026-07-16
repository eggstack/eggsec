"""Experimental APIs for eggsec.

.. warning::

    This namespace contains unstable APIs that may change or be removed
    without notice. Use at your own risk.

Experimental capabilities are isolated here to keep the core import clean.
Importing ``eggsec.experimental`` does not affect ``import eggsec``.

Maturity: experimental
See ``docs/python/namespace.md`` for the experimental namespace policy.
"""

__all__: list[str] = []

# Wireless testing (feature-gated: wireless)
try:
    from .._core import (
        SecurityTypePy as SecurityType,
        WirelessNetworkPy as WirelessNetwork,
        WirelessVulnerabilityPy as WirelessVulnerability,
        WirelessScanResultPy as WirelessScanResult,
        WirelessScanConfigPy as WirelessScanConfig,
        wireless_scan,
        async_wireless_scan,
        wireless_analyze_networks,
    )
    __all__.extend([
        "SecurityType", "WirelessNetwork", "WirelessVulnerability",
        "WirelessScanResult", "WirelessScanConfig",
        "wireless_scan", "async_wireless_scan", "wireless_analyze_networks",
    ])
except (AttributeError, ImportError):
    pass

# Evasion validation (feature-gated: evasion)
try:
    from .._core import (
        EvasionTargetTypePy as EvasionTargetType,
        EvasionCategoryPy as EvasionCategory,
        EvasionRiskPy as EvasionRisk,
        EvasionTechniquePy as EvasionTechnique,
        EvasionDetectionPy as EvasionDetection,
        EvasionSummaryPy as EvasionSummary,
        EvasionReportPy as EvasionReport,
        EvasionScanConfigPy as EvasionScanConfig,
        evasion_scan,
        async_evasion_scan,
        evasion_list_techniques,
    )
    __all__.extend([
        "EvasionTargetType", "EvasionCategory", "EvasionRisk",
        "EvasionTechnique", "EvasionDetection", "EvasionSummary",
        "EvasionReport", "EvasionScanConfig",
        "evasion_scan", "async_evasion_scan", "evasion_list_techniques",
    ])
except (AttributeError, ImportError):
    pass

# Post-exploitation simulation (feature-gated: postex)
try:
    from .._core import (
        PostexCategoryPy as PostexCategory,
        PostexRiskPy as PostexRisk,
        PostexProfilePy as PostexProfile,
        PostexTechniquePy as PostexTechnique,
        PostexDetectionPy as PostexDetection,
        PostexSummaryPy as PostexSummary,
        PostexReportPy as PostexReport,
        PostexScanConfigPy as PostexScanConfig,
        postex_scan,
        async_postex_scan,
        postex_list_techniques,
    )
    __all__.extend([
        "PostexCategory", "PostexRisk", "PostexProfile",
        "PostexTechnique", "PostexDetection", "PostexSummary",
        "PostexReport", "PostexScanConfig",
        "postex_scan", "async_postex_scan", "postex_list_techniques",
    ])
except (AttributeError, ImportError):
    pass

# C2 simulation (feature-gated: c2)
try:
    from .._core import (
        BeaconProtocolPy as BeaconProtocol,
        TaskTypePy as C2TaskType,
        TaskStatusPy as C2TaskStatus,
        OpsecCategoryPy as OpsecCategory,
        OpsecSeverityPy as OpsecSeverity,
        CampaignPhasePy as CampaignPhase,
        C2CampaignPy as C2Campaign,
        BeaconResultPy as BeaconResult,
        C2TaskResultPy as C2TaskResult,
        OpsecFindingPy as OpsecFinding,
        OpsecAssessmentPy as OpsecAssessment,
        C2SummaryPy as C2Summary,
        C2ReportPy as C2Report,
        C2ScanConfigPy as C2ScanConfig,
        c2_scan,
        async_c2_scan,
        c2_get_campaign,
    )
    __all__.extend([
        "BeaconProtocol", "C2TaskType", "C2TaskStatus",
        "OpsecCategory", "OpsecSeverity", "CampaignPhase",
        "C2Campaign", "BeaconResult", "C2TaskResult",
        "OpsecFinding", "OpsecAssessment", "C2Summary",
        "C2Report", "C2ScanConfig",
        "c2_scan", "async_c2_scan", "c2_get_campaign",
    ])
except (AttributeError, ImportError):
    pass

# Advanced hunting (feature-gated: advanced-hunting)
try:
    from .._core import (
        hunt_test,
        async_hunt_test,
        ChainTypePy as ChainType,
        ChainStepPy as ChainStep,
        AttackChainPy as AttackChain,
        FlawTypePy as FlawType,
        BusinessLogicFlawPy as BusinessLogicFlaw,
        RaceTypePy as RaceType,
        RaceConditionPy as RaceCondition,
        BypassTypePy as BypassType,
        AuthzBypassPy as AuthzBypass,
        SessionIssueTypePy as SessionIssueType,
        SessionIssuePy as SessionIssue,
        HuntTestConfigPy as HuntTestConfig,
        HuntReportPy as HuntReport,
    )
    __all__.extend([
        "hunt_test", "async_hunt_test",
        "ChainType", "ChainStep", "AttackChain",
        "FlawType", "BusinessLogicFlaw",
        "RaceType", "RaceCondition",
        "BypassType", "AuthzBypass",
        "SessionIssueType", "SessionIssue",
        "HuntTestConfig", "HuntReport",
    ])
except (AttributeError, ImportError):
    pass

# AI post-processing (feature-gated: ai-integration)
try:
    from .._core import (
        AiProviderPy as AiProvider,
        PluginLanguagePy as PluginLanguage,
        ScriptTargetPy as ScriptTarget,
        AiAnalysisResultPy as AiAnalysisResult,
        AiPayloadSuggestionPy as AiPayloadSuggestion,
        AiWafBypassSuggestionPy as AiWafBypassSuggestion,
        AiCacheStatsPy as AiCacheStats,
        ScriptMetadataPy as ScriptMetadata,
        GeneratedScriptPy as GeneratedScript,
        AiCachePy as AiCache,
        ai_analyze_finding,
        async_ai_analyze_finding,
        ai_generate_payloads,
        ai_suggest_waf_bypass,
        ai_generate_script,
    )
    __all__.extend([
        "AiProvider", "PluginLanguage", "ScriptTarget",
        "AiAnalysisResult", "AiPayloadSuggestion", "AiWafBypassSuggestion",
        "AiCacheStats", "ScriptMetadata", "GeneratedScript", "AiCache",
        "ai_analyze_finding", "async_ai_analyze_finding",
        "ai_generate_payloads", "ai_suggest_waf_bypass", "ai_generate_script",
    ])
except (AttributeError, ImportError):
    pass

# Stress testing (feature-gated: stress-testing)
try:
    from .._core import (
        StressTypePy as StressType,
        StressConfigPy as StressConfig,
        StressStatsPy as StressStats,
        StressResultPy as StressResult,
        stress_test,
        async_stress_test,
    )
    __all__.extend([
        "StressType", "StressConfig", "StressStats", "StressResult",
        "stress_test", "async_stress_test",
    ])
except (AttributeError, ImportError):
    pass

# Mobile dynamic (feature-gated: mobile-dynamic)
try:
    from .._core import (
        MobileDevicePy as MobileDevice,
        DynamicMobileConfigPy as DynamicMobileConfig,
        DynamicMobileReportPy as DynamicMobileReport,
        list_mobile_devices,
        dynamic_mobile_analysis,
    )
    __all__.extend([
        "MobileDevice", "DynamicMobileConfig", "DynamicMobileReport",
        "list_mobile_devices", "dynamic_mobile_analysis",
    ])
except (AttributeError, ImportError):
    pass

# Browser test (feature-gated: headless-browser)
try:
    from .._core import (
        browser_test,
        async_browser_test,
        XssSourcePy as XssSource,
        XssSinkPy as XssSink,
        DomXssFindingPy as DomXssFinding,
        DiscoveryMethodPy as DiscoveryMethod,
        SpaRoutePy as SpaRoute,
        ClientIssueTypePy as ClientIssueType,
        ClientIssuePy as ClientIssue,
        BrowserTestConfigPy as BrowserTestConfig,
        BrowserTestReportPy as BrowserTestReport,
    )
    __all__.extend([
        "browser_test", "async_browser_test",
        "XssSource", "XssSink", "DomXssFinding", "DiscoveryMethod",
        "SpaRoute", "ClientIssueType", "ClientIssue",
        "BrowserTestConfig", "BrowserTestReport",
    ])
except (AttributeError, ImportError):
    pass
