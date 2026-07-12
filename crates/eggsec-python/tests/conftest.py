"""Shared fixtures for eggsec-python behavior tests.

Uses SENTINEL values to prove data came from the fixture, not defaults.
"""

import pytest

# Sentinel values for proving data provenance
SENTINEL_TARGET = "sentinel.example.org"
SENTINEL_DOMAIN = "sentinel.example.org"
SENTINEL_PORT = 9999
SENTINEL_TIMEOUT_MS = 42424
SENTINEL_CONCURRENCY = 7
SENTINEL_MODE = "automation"
SENTINEL_METADATA_KEY = "test_trace_id"
SENTINEL_METADATA_VALUE = "uuid-trace-0xDEADBEEF"
SENTINEL_CORRELATION_ID = "corr-sentinel-12345"


@pytest.fixture
def sentinel_scope():
    """Scope that allows only the sentinel target."""
    import eggsec
    return eggsec.Scope.allow_hosts([SENTINEL_TARGET])


@pytest.fixture
def sentinel_scope_wildcard():
    """Scope that allows the sentinel domain and subdomains."""
    import eggsec
    return eggsec.Scope.allow_hosts([SENTINEL_TARGET, "*.sentinel.example.org"])


@pytest.fixture
def sentinel_engine(sentinel_scope):
    """Engine configured with sentinel scope."""
    import eggsec
    return eggsec.Engine(
        sentinel_scope,
        mode=SENTINEL_MODE,
        concurrency=SENTINEL_CONCURRENCY,
        timeout_ms=SENTINEL_TIMEOUT_MS,
    )


@pytest.fixture
def sentinel_async_engine(sentinel_scope):
    """AsyncEngine configured with sentinel scope."""
    import eggsec
    return eggsec.AsyncEngine(
        sentinel_scope,
        mode=SENTINEL_MODE,
        concurrency=SENTINEL_CONCURRENCY,
        timeout_ms=SENTINEL_TIMEOUT_MS,
    )


@pytest.fixture
def deny_all_scope():
    """Scope that denies everything."""
    import eggsec
    return eggsec.Scope.deny_all()


@pytest.fixture
def loaded_sentinel_scope():
    """LoadedScope with explicit provenance for sentinel target."""
    import eggsec
    scope = eggsec.Scope.allow_hosts([SENTINEL_TARGET])
    return eggsec.LoadedScope.explicit(
        scope, eggsec.ScopeSource.config_file(), "/tmp/sentinel-scope.toml"
    )


@pytest.fixture
def default_policy():
    """Default execution policy."""
    import eggsec
    return eggsec.ExecutionPolicy()


@pytest.fixture
def sentinel_descriptor():
    """OperationDescriptorPy targeting the sentinel host."""
    import eggsec
    return eggsec.OperationDescriptorPy(
        operation="scan-ports",
        mode="standard-assessment",
        risk="passive",
        intended_uses=["web-assessment"],
        target=SENTINEL_TARGET,
        requires_explicit_scope=True,
        required_capabilities=[],
    )
