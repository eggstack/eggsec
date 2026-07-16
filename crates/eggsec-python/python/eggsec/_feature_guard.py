"""Feature availability guard for lazy imports.

Provides structured error messages when feature-gated capabilities
are not available in the current wheel profile.
"""

from __future__ import annotations

import sys
from typing import Any, Callable


class FeatureUnavailableError(ImportError):
    """Raised when a feature-gated capability is not available.

    Attributes:
        module: The requested module or symbol name.
        feature: The required Cargo feature flag.
        profile: The current wheel build profile.
        maturity: The maturity level of the capability.
        install_hint: Supported installation/build route.
        platform_prereqs: Platform prerequisites, if any.
    """

    def __init__(
        self,
        module: str,
        feature: str,
        profile: str = "default",
        maturity: str = "experimental",
        install_hint: str = "",
        platform_prereqs: str = "",
    ):
        self.module = module
        self.feature = feature
        self.profile = profile
        self.maturity = maturity
        self.install_hint = install_hint
        self.platform_prereqs = platform_prereqs

        parts = [f"'{module}' requires the '{feature}' feature"]
        parts.append(f"Current profile: {profile}")
        if maturity:
            parts.append(f"Maturity: {maturity}")
        if install_hint:
            parts.append(f"Install: {install_hint}")
        if platform_prereqs:
            parts.append(f"Platform: {platform_prereqs}")
        super().__init__(". ".join(parts))


# Feature metadata registry
_FEATURES: dict[str, dict[str, str]] = {
    "wireless": {
        "feature": "wireless",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[wireless] or build with --features wireless",
        "platform_prereqs": "Linux; root required for real scans",
    },
    "wireless-advanced": {
        "feature": "wireless-advanced",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[wireless-advanced]",
        "platform_prereqs": "Linux; root required",
    },
    "mobile": {
        "feature": "mobile",
        "maturity": "stable",
        "install_hint": "pip install eggsec[mobile] or build with --features mobile",
    },
    "mobile-dynamic": {
        "feature": "mobile-dynamic",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[mobile-dynamic]",
        "platform_prereqs": "ADB and Android device/emulator",
    },
    "db-pentest": {
        "feature": "db-pentest",
        "maturity": "stable",
        "install_hint": "pip install eggsec[db-pentest] or build with --features db-pentest",
    },
    "db-pentest-mongodb": {
        "feature": "db-pentest-mongodb",
        "maturity": "provisional",
        "install_hint": "pip install eggsec[db-pentest-mongodb]",
    },
    "db-pentest-redis": {
        "feature": "db-pentest-redis",
        "maturity": "provisional",
        "install_hint": "pip install eggsec[db-pentest-redis]",
    },
    "web-proxy": {
        "feature": "web-proxy",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[web-proxy] or build with --features web-proxy",
    },
    "nse": {
        "feature": "nse",
        "maturity": "stable",
        "install_hint": "pip install eggsec[nse] or build with --features nse",
        "platform_prereqs": "libssl-dev",
    },
    "evasion": {
        "feature": "evasion",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[evasion] or build with --features evasion",
    },
    "postex": {
        "feature": "postex",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[postex] or build with --features postex",
    },
    "c2": {
        "feature": "c2",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[c2] or build with --features c2",
    },
    "container": {
        "feature": "container",
        "maturity": "stable",
        "install_hint": "pip install eggsec[container] or build with --features container",
    },
    "headless-browser": {
        "feature": "headless-browser",
        "maturity": "provisional",
        "install_hint": "pip install eggsec[headless-browser]",
    },
    "websocket": {
        "feature": "websocket",
        "maturity": "provisional",
        "install_hint": "pip install eggsec[websocket]",
    },
    "git-secrets": {
        "feature": "git-secrets",
        "maturity": "stable",
        "install_hint": "pip install eggsec[git-secrets]",
    },
    "sbom": {
        "feature": "sbom",
        "maturity": "stable",
        "install_hint": "pip install eggsec[sbom]",
    },
    "packet-inspection": {
        "feature": "packet-inspection",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[packet-inspection]",
        "platform_prereqs": "libpcap-dev",
    },
    "stress-testing": {
        "feature": "stress-testing",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[stress-testing]",
    },
    "daemon-client": {
        "feature": "daemon-client",
        "maturity": "provisional",
        "install_hint": "pip install eggsec[daemon-client]",
    },
    "advanced-hunting": {
        "feature": "advanced-hunting",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[advanced-hunting]",
    },
    "ai-integration": {
        "feature": "ai-integration",
        "maturity": "experimental",
        "install_hint": "pip install eggsec[ai-integration]",
    },
    "compliance": {
        "feature": "compliance",
        "maturity": "provisional",
        "install_hint": "pip install eggsec[compliance]",
    },
}


def require_feature(feature_key: str, symbol: str = "") -> None:
    """Check if a feature is available, raise FeatureUnavailableError if not.

    Args:
        feature_key: Key in the _FEATURES registry.
        symbol: Optional symbol name for error context.

    Raises:
        FeatureUnavailableError: If the feature is not available.
    """
    if feature_key not in _FEATURES:
        raise ValueError(f"Unknown feature key: {feature_key}")

    meta = _FEATURES[feature_key]
    # Check if the feature is actually available by trying to import _core
    # and checking if the feature gate is compiled in
    try:
        from . import _core
        # The feature is available if we can import without error
        return
    except ImportError:
        pass

    module_name = symbol or feature_key
    raise FeatureUnavailableError(
        module=module_name,
        feature=meta["feature"],
        maturity=meta.get("maturity", "experimental"),
        install_hint=meta.get("install_hint", ""),
        platform_prereqs=meta.get("platform_prereqs", ""),
    )


def list_unavailable_features() -> list[dict[str, str]]:
    """List features that are not available in the current build.

    Returns:
        List of dicts with feature info for unavailable capabilities.
    """
    unavailable = []
    try:
        from . import _core
        # Try to detect which features are compiled in
        # by attempting to access feature-gated symbols
        feature_checks = {
            "wireless": lambda: _core.wireless_scan,
            "evasion": lambda: _core.evasion_scan,
            "postex": lambda: _core.postex_scan,
            "c2": lambda: _core.c2_scan,
            "mobile": lambda: _core.analyze_apk,
            "mobile-dynamic": lambda: _core.list_mobile_devices,
            "db-pentest": lambda: _core.db_probe,
            "web-proxy": lambda: _core.create_proxy_manager,
            "nse": lambda: _core.nse_run,
            "container": lambda: _core.scan_docker_image,
            "headless-browser": lambda: _core.browser_test,
            "websocket": lambda: _core.websocket_probe,
            "packet-inspection": lambda: _core.parse_pcap,
            "stress-testing": lambda: _core.stress_test,
            "daemon-client": lambda: _core.daemon_connect,
            "advanced-hunting": lambda: _core.hunt_test,
            "ai-integration": lambda: _core.ai_analyze_finding,
            "git-secrets": lambda: _core.scan_git_secrets,
            "sbom": lambda: _core.generate_sbom,
            "compliance": lambda: _core.ComplianceFramework,
        }
        for key, check in feature_checks.items():
            try:
                check()
            except (AttributeError, ImportError):
                meta = _FEATURES.get(key, {})
                unavailable.append({
                    "feature": key,
                    "cargo_feature": meta.get("feature", key),
                    "maturity": meta.get("maturity", "experimental"),
                    "install_hint": meta.get("install_hint", ""),
                    "platform_prereqs": meta.get("platform_prereqs", ""),
                })
    except ImportError:
        # If _core itself can't be imported, all features are unavailable
        for key, meta in _FEATURES.items():
            unavailable.append({
                "feature": key,
                "cargo_feature": meta.get("feature", key),
                "maturity": meta.get("maturity", "experimental"),
                "install_hint": meta.get("install_hint", ""),
                "platform_prereqs": meta.get("platform_prereqs", ""),
            })
    return unavailable
