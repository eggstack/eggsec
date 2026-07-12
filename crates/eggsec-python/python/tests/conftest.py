import pytest

# Keep the two supported test roots composable when pytest imports both
# conftest files in one invocation. The behavior-suite tests import these
# sentinels directly, while the smoke tests only need marker registration.
SENTINEL_TARGET = "sentinel.example.org"
SENTINEL_DOMAIN = SENTINEL_TARGET
SENTINEL_PORT = 9999
SENTINEL_TIMEOUT_MS = 42424
SENTINEL_CONCURRENCY = 7
SENTINEL_MODE = "automation"
SENTINEL_METADATA_KEY = "test_trace_id"
SENTINEL_METADATA_VALUE = "uuid-trace-0xDEADBEEF"
SENTINEL_CORRELATION_ID = "corr-sentinel-12345"


@pytest.fixture
def sentinel_scope():
    import eggsec
    return eggsec.Scope.allow_hosts([SENTINEL_TARGET])


@pytest.fixture
def sentinel_scope_wildcard():
    import eggsec
    return eggsec.Scope.allow_hosts([SENTINEL_TARGET, "*.sentinel.example.org"])


@pytest.fixture
def sentinel_engine(sentinel_scope):
    import eggsec
    return eggsec.Engine(
        sentinel_scope,
        mode=SENTINEL_MODE,
        concurrency=SENTINEL_CONCURRENCY,
        timeout_ms=SENTINEL_TIMEOUT_MS,
    )


@pytest.fixture
def sentinel_async_engine(sentinel_scope):
    import eggsec
    return eggsec.AsyncEngine(
        sentinel_scope,
        mode=SENTINEL_MODE,
        concurrency=SENTINEL_CONCURRENCY,
        timeout_ms=SENTINEL_TIMEOUT_MS,
    )


@pytest.fixture
def deny_all_scope():
    import eggsec
    return eggsec.Scope.deny_all()


@pytest.fixture
def loaded_sentinel_scope():
    import eggsec
    scope = eggsec.Scope.allow_hosts([SENTINEL_TARGET])
    return eggsec.LoadedScope.explicit(
        scope, eggsec.ScopeSource.config_file(), "/tmp/sentinel-scope.toml"
    )


@pytest.fixture
def default_policy():
    import eggsec
    return eggsec.ExecutionPolicy()


def pytest_configure(config):
    config.addinivalue_line("markers", "network: mark test as requiring network access")
