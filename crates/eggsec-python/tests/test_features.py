"""Feature profiles tests.

Verifies that default profile matches documented features, features() and
has_feature() are consistent, feature_matrix() lists all documented features,
and Python-side and Rust-side feature lists match.
"""

import pytest

import eggsec


# ---------------------------------------------------------------------------
# All documented feature names (from features.rs and AGENTS.md)
# ---------------------------------------------------------------------------

ALWAYS_AVAILABLE_FEATURES = [
    "core",
    "scanner",
    "async-api",
    "endpoint-discovery",
    "service-fingerprinting",
    "waf-detection",
    "waf-validation",
    "http-fuzzing",
    "load-testing",
    "findings-reporting",
]

GATED_FEATURES = [
    "websocket",
    "git-secrets",
    "sbom",
    "db-pentest",
    "db-pentest-mongodb",
    "db-pentest-redis",
    "web-proxy",
    "mobile",
    "mobile-dynamic",
    "packet-inspection",
    "stress-testing",
    "nse",
    "container",
    "daemon-client",
    "headless-browser",
    "advanced-hunting",
    "compliance",
    "wireless",
    "evasion",
    "postex",
    "c2",
    "ai-integration",
]

ALL_FEATURES = ALWAYS_AVAILABLE_FEATURES + GATED_FEATURES


# ---------------------------------------------------------------------------
# 1. Default profile matches documented features
# ---------------------------------------------------------------------------


class TestDefaultProfile:
    def test_always_available_features_are_true(self):
        """All always-available features should be True in features()."""
        features = eggsec.features()
        for name in ALWAYS_AVAILABLE_FEATURES:
            assert name in features, f"Feature '{name}' missing from features()"
            assert features[name] is True, f"Always-available feature '{name}' is not True"

    def test_core_is_always_true(self):
        """core feature must always be True."""
        assert eggsec.has_feature("core") is True
        assert eggsec.features()["core"] is True

    def test_scanner_is_always_true(self):
        """scanner feature must always be True."""
        assert eggsec.has_feature("scanner") is True
        assert eggsec.features()["scanner"] is True

    def test_async_api_is_always_true(self):
        """async-api feature must always be True."""
        assert eggsec.has_feature("async-api") is True

    def test_unknown_feature_returns_false(self):
        """has_feature() should return False for unknown feature names."""
        assert eggsec.has_feature("nonexistent-feature-SENTINEL") is False
        assert eggsec.has_feature("totally-made-up") is False


# ---------------------------------------------------------------------------
# 2. features() and has_feature() are consistent
# ---------------------------------------------------------------------------


class TestFeaturesConsistency:
    def test_features_dict_matches_has_feature(self):
        """features() dict and has_feature() should agree for all known features."""
        features = eggsec.features()
        for name in ALL_FEATURES:
            if name in features:
                assert features[name] == eggsec.has_feature(name), (
                    f"Mismatch for feature '{name}': features()={features[name]}, "
                    f"has_feature()={eggsec.has_feature(name)}"
                )

    def test_always_available_in_features_dict(self):
        """All always-available features appear in features() dict."""
        features = eggsec.features()
        for name in ALWAYS_AVAILABLE_FEATURES:
            assert name in features

    def test_gated_features_in_features_dict(self):
        """All gated features appear in features() dict."""
        features = eggsec.features()
        for name in GATED_FEATURES:
            assert name in features, f"Gated feature '{name}' missing from features() dict"


# ---------------------------------------------------------------------------
# 3. feature_matrix() lists all documented features
# ---------------------------------------------------------------------------


class TestFeatureMatrix:
    def test_feature_matrix_returns_dict(self):
        matrix = eggsec.feature_matrix()
        assert isinstance(matrix, dict)

    def test_feature_matrix_contains_all_features(self):
        """feature_matrix should contain all documented feature names."""
        matrix = eggsec.feature_matrix()
        for name in ALL_FEATURES:
            assert name in matrix, f"Feature '{name}' missing from feature_matrix()"

    def test_feature_matrix_has_required_fields(self):
        """Each entry in feature_matrix should have 'available', 'description', 'requires_system_deps'."""
        matrix = eggsec.feature_matrix()
        for name, entry in matrix.items():
            assert "available" in entry, f"Feature '{name}' missing 'available' field"
            assert "description" in entry, f"Feature '{name}' missing 'description' field"
            assert "requires_system_deps" in entry, f"Feature '{name}' missing 'requires_system_deps' field"

    def test_feature_matrix_available_matches_has_feature(self):
        """feature_matrix 'available' should match has_feature() for all features."""
        matrix = eggsec.feature_matrix()
        for name in ALL_FEATURES:
            if name in matrix:
                assert matrix[name]["available"] == eggsec.has_feature(name), (
                    f"feature_matrix '{name}' available={matrix[name]['available']} "
                    f"!= has_feature()={eggsec.has_feature(name)}"
                )

    def test_feature_matrix_descriptions_are_nonempty(self):
        """All feature descriptions should be non-empty strings."""
        matrix = eggsec.feature_matrix()
        for name, entry in matrix.items():
            assert isinstance(entry["description"], str)
            assert len(entry["description"]) > 0, f"Feature '{name}' has empty description"

    def test_feature_matrix_requires_system_deps_is_bool(self):
        """requires_system_deps should be a boolean."""
        matrix = eggsec.feature_matrix()
        for name, entry in matrix.items():
            assert isinstance(entry["requires_system_deps"], bool), (
                f"Feature '{name}' requires_system_deps is not bool: {type(entry['requires_system_deps'])}"
            )


# ---------------------------------------------------------------------------
# 4. Python-side and Rust-side feature lists match
# ---------------------------------------------------------------------------


class TestPythonRustFeatureMatch:
    def test_api_surface_version_features_list(self):
        """api_surface_version() should list all features in features_list."""
        version_info = eggsec.api_surface_version()
        assert "features_list" in version_info
        rust_features = set(version_info["features_list"])
        python_features = set(ALL_FEATURES)
        assert rust_features == python_features, (
            f"Feature list mismatch.\n"
            f"  In Rust but not Python: {rust_features - python_features}\n"
            f"  In Python but not Rust: {python_features - rust_features}"
        )

    def test_feature_count_matches(self):
        """Number of features in features() should match ALL_FEATURES length."""
        features = eggsec.features()
        assert len(features) == len(ALL_FEATURES), (
            f"features() has {len(features)} entries, expected {len(ALL_FEATURES)}"
        )

    def test_feature_matrix_count_matches(self):
        """Number of features in feature_matrix() should match ALL_FEATURES length."""
        matrix = eggsec.feature_matrix()
        assert len(matrix) == len(ALL_FEATURES), (
            f"feature_matrix() has {len(matrix)} entries, expected {len(ALL_FEATURES)}"
        )


# ---------------------------------------------------------------------------
# 5. Build info and version constants
# ---------------------------------------------------------------------------


class TestBuildInfo:
    def test_build_info_returns_dict(self):
        info = eggsec.build_info()
        assert isinstance(info, dict)

    def test_build_info_has_version(self):
        info = eggsec.build_info()
        assert info["version"] == "0.1.0"

    def test_build_info_has_required_fields(self):
        info = eggsec.build_info()
        assert "version" in info
        assert "package_name" in info
        assert "target_triple" in info
        assert "binding_version" in info

    def test_version_constants(self):
        assert eggsec.__schema_version__ == "1.0"
        assert eggsec.__protocol_version__ == "1.0.0"
        assert eggsec.__abi_version__ == "1"

    def test_event_schema_version(self):
        assert eggsec.EVENT_SCHEMA_VERSION == "1.0.0"
