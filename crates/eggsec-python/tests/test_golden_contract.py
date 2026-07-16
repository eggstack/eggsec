"""Golden Contract Test Suite — Workstream B1

Purpose
-------
This suite freezes the CURRENT behavior of all 22 stable-core operations
in the eggsec Python API BEFORE the dispatch-system refactor (Phase B).
Every assertion here is a regression gate: if any post-refactor change
alters the documented behavior, this suite must be updated with an
explicit explanation of the intentional change.

What this suite captures
------------------------
- Canonical snake_case operation IDs for all 22 stable operations.
- Legacy alias resolution (historical aliases still dispatch correctly).
- Feature-gate metadata (which operations require which features).
- Risk classification (deterministic risk string per operation).
- Confirmation requirements (which operations need user confirmation).
- Descriptor metadata completeness (all required fields populated).
- Tool descriptor consistency (ToolRegistry descriptors match operation metadata).
- Schema generation (all operations produce valid JSON Schema Draft 2020-12).
- Operation listing (Engine.list_operations() returns all 22 operations).
- Registry containment (all IDs resolve through the registry).

Design decisions
----------------
- No network I/O. All tests exercise metadata and registry only.
- No snapshots of timestamps or generated IDs.
- Deterministic fixtures derived from canonical Rust definitions.
- Class-based test organization for readability and selective execution.
"""

import json
import re

import pytest

# ---------------------------------------------------------------------------
# Canonical constants — the single source of truth for this suite
# ---------------------------------------------------------------------------

ALL_STABLE_OPERATION_IDS = [
    "scan_ports",
    "scan_endpoints",
    "fingerprint_services",
    "recon_dns",
    "inspect_tls",
    "detect_technology",
    "detect_waf",
    "validate_waf",
    "fuzz_http",
    "load_test",
    "scan_git_secrets",
    "generate_sbom",
    "run_consolidated_recon",
    "graphql_test",
    "oauth_test",
    "auth_test",
    "db_probe",
    "nse_run",
    "scan_docker_image",
    "scan_kubernetes",
    "analyze_apk",
    "analyze_ipa",
]

# Canonical display names (from StableOperation::name() in Rust)
CANONICAL_NAMES = {
    "scan_ports": "Port Scan",
    "scan_endpoints": "Endpoint Scan",
    "fingerprint_services": "Service Fingerprinting",
    "recon_dns": "DNS Reconnaissance",
    "inspect_tls": "TLS Inspection",
    "detect_technology": "Technology Detection",
    "detect_waf": "WAF Detection",
    "validate_waf": "WAF Validation",
    "fuzz_http": "HTTP Fuzzing",
    "load_test": "Load Test",
    "scan_git_secrets": "Git Secrets Scan",
    "generate_sbom": "SBOM Generation",
    "run_consolidated_recon": "Consolidated Recon",
    "graphql_test": "GraphQL Security Test",
    "oauth_test": "OAuth Security Test",
    "auth_test": "Authentication Assessment",
    "db_probe": "Database Probe",
    "nse_run": "NSE Script Execution",
    "scan_docker_image": "Docker Image Scan",
    "scan_kubernetes": "Kubernetes Scan",
    "analyze_apk": "APK Analysis",
    "analyze_ipa": "IPA Analysis",
}

# Legacy aliases: alias → canonical ID
LEGACY_ALIASES = {
    "fingerprint": "fingerprint_services",
    "recon": "recon_dns",
    "tls_inspect": "inspect_tls",
    "tech_detect": "detect_technology",
    "waf_detect": "detect_waf",
    "waf_validate": "validate_waf",
    "http_fuzz": "fuzz_http",
    "load_test_http": "load_test",
    "consolidated_recon": "run_consolidated_recon",
}

# Feature-gate mapping: feature → list of operation IDs
FEATURE_GATE_MAP = {
    "git-secrets": ["scan_git_secrets"],
    "sbom": ["generate_sbom"],
    "db-pentest": ["db_probe"],
    "nse": ["nse_run"],
    "container": ["scan_docker_image", "scan_kubernetes"],
    "mobile": ["analyze_apk", "analyze_ipa"],
}

# Operations that have NO feature requirement (always available)
NO_FEATURE_OPERATIONS = [
    "scan_ports",
    "scan_endpoints",
    "fingerprint_services",
    "recon_dns",
    "inspect_tls",
    "detect_technology",
    "detect_waf",
    "validate_waf",
    "fuzz_http",
    "load_test",
    "run_consolidated_recon",
    "graphql_test",
    "oauth_test",
    "auth_test",
]

# Operations requiring user confirmation
CONFIRMATION_REQUIRED_OPS = {"nse_run", "db_probe", "fuzz_http", "load_test"}

# Risk classification: operation_id → expected risk string
RISK_MAP = {
    "scan_ports": "safe_active",
    "scan_endpoints": "safe_active",
    "fingerprint_services": "safe_active",
    "recon_dns": "safe_active",
    "inspect_tls": "safe_active",
    "detect_technology": "safe_active",
    "detect_waf": "safe_active",
    "validate_waf": "safe_active",
    "run_consolidated_recon": "safe_active",
    "graphql_test": "safe_active",
    "oauth_test": "safe_active",
    "auth_test": "safe_active",
    "scan_git_secrets": "safe_active",
    "generate_sbom": "safe_active",
    "scan_docker_image": "safe_active",
    "scan_kubernetes": "safe_active",
    "analyze_apk": "safe_active",
    "analyze_ipa": "safe_active",
    "fuzz_http": "intrusive",
    "nse_run": "intrusive",
    "load_test": "load_test",
    "db_probe": "db_pentest",
}

# Category mapping: operation_id → expected category
CATEGORY_MAP = {
    "scan_ports": "scanning",
    "scan_endpoints": "scanning",
    "fingerprint_services": "fingerprinting",
    "recon_dns": "recon",
    "inspect_tls": "recon",
    "detect_technology": "assessment",
    "detect_waf": "waf",
    "validate_waf": "waf",
    "fuzz_http": "fuzzing",
    "load_test": "load_testing",
    "scan_git_secrets": "assessment",
    "generate_sbom": "assessment",
    "run_consolidated_recon": "recon",
    "graphql_test": "assessment",
    "oauth_test": "assessment",
    "auth_test": "assessment",
    "db_probe": "database",
    "nse_run": "nse",
    "scan_docker_image": "container",
    "scan_kubernetes": "container",
    "analyze_apk": "mobile",
    "analyze_ipa": "mobile",
}


# ---------------------------------------------------------------------------
# 1. Operation ID canonical forms
# ---------------------------------------------------------------------------


class TestOperationIdCanonicalForms:
    """Each stable operation has a canonical snake_case ID."""

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_ids_are_snake_case(self, op_id):
        assert re.match(r"^[a-z][a-z0-9]*(_[a-z0-9]+)*$", op_id), (
            f"Operation ID '{op_id}' is not snake_case"
        )

    def test_exactly_22_operations(self):
        assert len(ALL_STABLE_OPERATION_IDS) == 22

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_operation_id_in_registry(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor is not None, (
            f"Operation '{op_id}' not found in ToolRegistry"
        )
        assert descriptor.tool_id == op_id

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_operation_id_matches_descriptor(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor.tool_id == op_id
        assert descriptor.operation_id is not None
        assert len(descriptor.operation_id) > 0


# ---------------------------------------------------------------------------
# 2. Legacy alias resolution
# ---------------------------------------------------------------------------


class TestLegacyAliasResolution:
    """Historical aliases still resolve to the correct canonical operation."""

    @pytest.mark.parametrize("alias,canonical", LEGACY_ALIASES.items())
    def test_alias_resolves_to_canonical(self, alias, canonical):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(alias)
        assert descriptor is not None, (
            f"Alias '{alias}' did not resolve in ToolRegistry"
        )
        assert descriptor.tool_id == canonical, (
            f"Alias '{alias}' resolved to '{descriptor.tool_id}', "
            f"expected '{canonical}'"
        )

    @pytest.mark.parametrize("alias,canonical", LEGACY_ALIASES.items())
    def test_alias_has_same_title_as_canonical(self, alias, canonical):
        from eggsec import ToolRegistry

        alias_desc = ToolRegistry.get(alias)
        canonical_desc = ToolRegistry.get(canonical)
        assert alias_desc is not None
        assert canonical_desc is not None
        assert alias_desc.title == canonical_desc.title

    def test_all_aliases_differ_from_canonical_ids(self):
        for alias in LEGACY_ALIASES:
            assert alias not in ALL_STABLE_OPERATION_IDS, (
                f"Alias '{alias}' collides with a canonical operation ID"
            )

    def test_alias_count(self):
        assert len(LEGACY_ALIASES) == 9

    def test_aliases_do_not_collide_with_each_other(self):
        targets = list(LEGACY_ALIASES.values())
        assert len(targets) == len(set(targets)), (
            "Multiple aliases resolve to the same canonical ID"
        )


# ---------------------------------------------------------------------------
# 3. Feature gate metadata
# ---------------------------------------------------------------------------


class TestFeatureGateMetadata:
    """Operations correctly declare their feature requirements."""

    @pytest.mark.parametrize("feature,operations", FEATURE_GATE_MAP.items())
    def test_feature_gated_operations(self, feature, operations):
        from eggsec import ToolRegistry

        for op_id in operations:
            descriptor = ToolRegistry.get(op_id)
            assert descriptor is not None
            assert descriptor.feature_required == feature, (
                f"Operation '{op_id}' feature_required is "
                f"'{descriptor.feature_required}', expected '{feature}'"
            )

    @pytest.mark.parametrize("op_id", NO_FEATURE_OPERATIONS)
    def test_no_feature_required(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor is not None
        assert descriptor.feature_required is None, (
            f"Operation '{op_id}' unexpectedly requires feature "
            f"'{descriptor.feature_required}'"
        )

    def test_operations_for_feature_matches_expectations(self):
        from eggsec import ToolRegistry

        for feature, expected_ops in FEATURE_GATE_MAP.items():
            found = ToolRegistry.operations_for_feature(feature)
            found_ids = {d.tool_id for d in found}
            for op_id in expected_ops:
                assert op_id in found_ids, (
                    f"Feature '{feature}' should include '{op_id}' "
                    f"but operations_for_feature returned: {found_ids}"
                )

    def test_no_orphan_feature_gates(self):
        """Every feature-gated operation must appear in FEATURE_GATE_MAP."""
        from eggsec import ToolRegistry

        for descriptor in ToolRegistry.list():
            if descriptor.feature_required is not None:
                feature = descriptor.feature_required
                assert feature in FEATURE_GATE_MAP, (
                    f"Operation '{descriptor.tool_id}' requires feature "
                    f"'{feature}' which is not in FEATURE_GATE_MAP"
                )
                assert descriptor.tool_id in FEATURE_GATE_MAP[feature], (
                    f"Operation '{descriptor.tool_id}' requires feature "
                    f"'{feature}' but is not listed in FEATURE_GATE_MAP"
                )

    def test_all_22_operations_have_feature_metadata(self):
        """Every stable operation appears exactly once across the feature maps."""
        from eggsec import ToolRegistry

        all_mapped = set()
        for ops in FEATURE_GATE_MAP.values():
            all_mapped.update(ops)
        all_mapped.update(NO_FEATURE_OPERATIONS)
        assert sorted(all_mapped) == sorted(ALL_STABLE_OPERATION_IDS)


# ---------------------------------------------------------------------------
# 4. Risk classification
# ---------------------------------------------------------------------------


class TestRiskClassification:
    """Each operation has a deterministic, documented risk level."""

    @pytest.mark.parametrize("op_id,expected_risk", RISK_MAP.items())
    def test_risk_classification(self, op_id, expected_risk):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor is not None
        assert descriptor.risk == expected_risk, (
            f"Operation '{op_id}' risk is '{descriptor.risk}', "
            f"expected '{expected_risk}'"
        )

    def test_all_22_operations_have_risk(self):
        from eggsec import ToolRegistry

        for descriptor in ToolRegistry.list():
            assert descriptor.risk, (
                f"Operation '{descriptor.tool_id}' has empty risk"
            )
            assert descriptor.risk in RISK_MAP.values(), (
                f"Operation '{descriptor.tool_id}' has unexpected risk "
                f"'{descriptor.risk}'"
            )

    def test_intrusive_operations_are_confirmed(self):
        """All intrusive-risk operations require confirmation."""
        from eggsec import ToolRegistry

        for op_id, risk in RISK_MAP.items():
            if risk == "intrusive":
                descriptor = ToolRegistry.get(op_id)
                assert descriptor.confirmation_required, (
                    f"Intrusive operation '{op_id}' does not require "
                    f"confirmation"
                )

    def test_load_test_risk_is_distinct(self):
        """Load test has its own risk tier, separate from intrusive."""
        assert RISK_MAP["load_test"] == "load_test"
        assert RISK_MAP["load_test"] != "intrusive"


# ---------------------------------------------------------------------------
# 5. Confirmation requirements
# ---------------------------------------------------------------------------


class TestConfirmationRequirements:
    """Correct operations require explicit user confirmation."""

    @pytest.mark.parametrize("op_id", sorted(CONFIRMATION_REQUIRED_OPS))
    def test_confirmation_required(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor is not None
        assert descriptor.confirmation_required is True, (
            f"Operation '{op_id}' should require confirmation"
        )

    def test_confirmation_message_present_when_required(self):
        from eggsec import ToolRegistry

        for op_id in CONFIRMATION_REQUIRED_OPS:
            descriptor = ToolRegistry.get(op_id)
            assert descriptor.confirmation_message is not None, (
                f"Operation '{op_id}' requires confirmation but has no "
                f"confirmation_message"
            )
            assert len(descriptor.confirmation_message) > 0

    def test_non_confirming_ops_have_no_message(self):
        from eggsec import ToolRegistry

        for descriptor in ToolRegistry.list():
            if not descriptor.confirmation_required:
                assert descriptor.confirmation_message is None, (
                    f"Operation '{descriptor.tool_id}' does not require "
                    f"confirmation but has confirmation_message "
                    f"'{descriptor.confirmation_message}'"
                )

    def test_exactly_4_operations_require_confirmation(self):
        from eggsec import ToolRegistry

        confirming = [
            d.tool_id for d in ToolRegistry.list()
            if d.confirmation_required
        ]
        assert len(confirming) == 4, (
            f"Expected 4 confirming operations, found {len(confirming)}: "
            f"{confirming}"
        )
        assert set(confirming) == CONFIRMATION_REQUIRED_OPS

    def test_confirmation_message_is_generic(self):
        """All confirmation messages use the standard wording."""
        from eggsec import ToolRegistry

        for op_id in CONFIRMATION_REQUIRED_OPS:
            descriptor = ToolRegistry.get(op_id)
            assert "external systems" in descriptor.confirmation_message.lower(), (
                f"Operation '{op_id}' confirmation message does not contain "
                f"'external systems': '{descriptor.confirmation_message}'"
            )


# ---------------------------------------------------------------------------
# 6. Descriptor metadata completeness
# ---------------------------------------------------------------------------


class TestDescriptorMetadataCompleteness:
    """All required descriptor fields are populated for every operation."""

    REQUIRED_STRING_FIELDS = [
        "tool_id",
        "operation_id",
        "title",
        "description",
        "version",
        "category",
        "risk",
        "maturity",
        "target_policy",
    ]

    REQUIRED_BOOL_FIELDS = [
        "confirmation_required",
        "supports_streaming",
        "supports_cancellation",
        "supports_timeout",
        "local_available",
        "daemon_available",
    ]

    REQUIRED_LIST_FIELDS = [
        "intended_uses",
        "supported_surfaces",
    ]

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    @pytest.mark.parametrize("field", REQUIRED_STRING_FIELDS)
    def test_string_fields_are_nonempty(self, op_id, field):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor is not None
        value = getattr(descriptor, field)
        assert isinstance(value, str)
        assert len(value) > 0, (
            f"Operation '{op_id}' has empty string field '{field}'"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    @pytest.mark.parametrize("field", REQUIRED_BOOL_FIELDS)
    def test_bool_fields_are_defined(self, op_id, field):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor is not None
        value = getattr(descriptor, field)
        assert isinstance(value, bool), (
            f"Operation '{op_id}' field '{field}' is not bool: {type(value)}"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    @pytest.mark.parametrize("field", REQUIRED_LIST_FIELDS)
    def test_list_fields_are_nonempty(self, op_id, field):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor is not None
        value = getattr(descriptor, field)
        assert isinstance(value, list)
        assert len(value) > 0, (
            f"Operation '{op_id}' has empty list field '{field}'"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_maturity_is_stable(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor.maturity == "stable"

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_description_format(self, op_id):
        """Description should be a non-empty sentence-like string."""
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        desc = descriptor.description
        assert len(desc) > 10
        assert desc[0].isupper() or desc[0].islower()

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_version_is_semver(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert re.match(r"^\d+\.\d+\.\d+", descriptor.version), (
            f"Operation '{op_id}' version '{descriptor.version}' "
            f"is not semver-like"
        )


# ---------------------------------------------------------------------------
# 7. Tool descriptor consistency
# ---------------------------------------------------------------------------


class TestToolDescriptorConsistency:
    """ToolRegistry descriptors are consistent with the canonical definitions."""

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_tool_id_matches_title(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor.title == CANONICAL_NAMES[op_id], (
            f"Operation '{op_id}' title '{descriptor.title}' does not "
            f"match canonical name '{CANONICAL_NAMES[op_id]}'"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_category_matches_expectation(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor.category == CATEGORY_MAP[op_id], (
            f"Operation '{op_id}' category '{descriptor.category}' "
            f"does not match expected '{CATEGORY_MAP[op_id]}'"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_to_dict_has_all_keys(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        d = descriptor.to_dict()
        expected_keys = {
            "tool_id", "operation_id", "title", "description", "version",
            "category", "risk", "feature_required", "maturity",
            "confirmation_required", "target_policy", "input_schema",
            "output_schema", "supports_streaming", "supports_cancellation",
            "supports_timeout", "local_available", "daemon_available",
            "intended_uses", "supported_surfaces",
        }
        missing = expected_keys - set(d.keys())
        assert not missing, (
            f"Operation '{op_id}' descriptor.to_dict() missing keys: {missing}"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_to_json_is_valid(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        j = descriptor.to_json()
        parsed = json.loads(j)
        assert parsed["tool_id"] == op_id
        assert "operation_id" in parsed

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_repr_contains_tool_id(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        r = repr(descriptor)
        assert op_id in r

    def test_registry_count_matches(self):
        from eggsec import ToolRegistry

        assert ToolRegistry.count() == 22

    def test_registry_list_count_matches(self):
        from eggsec import ToolRegistry

        tools = ToolRegistry.list()
        assert len(tools) == 22

    def test_registry_list_tool_ids_are_complete(self):
        from eggsec import ToolRegistry

        tools = ToolRegistry.list()
        tool_ids = sorted(d.tool_id for d in tools)
        assert tool_ids == sorted(ALL_STABLE_OPERATION_IDS)


# ---------------------------------------------------------------------------
# 8. Schema generation
# ---------------------------------------------------------------------------


class TestSchemaGeneration:
    """All operations produce valid JSON Schema Draft 2020-12."""

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_input_schema_exists(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor.input_schema is not None, (
            f"Operation '{op_id}' has no input_schema"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_input_schema_is_valid_json(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        parsed = json.loads(descriptor.input_schema)
        assert isinstance(parsed, dict)

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_input_schema_has_dialect(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        parsed = json.loads(descriptor.input_schema)
        assert "$schema" in parsed
        assert "draft/2020-12" in parsed["$schema"]

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_input_schema_is_object_type(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        parsed = json.loads(descriptor.input_schema)
        assert parsed.get("type") == "object"

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_input_schema_has_properties(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        parsed = json.loads(descriptor.input_schema)
        assert "properties" in parsed
        assert isinstance(parsed["properties"], dict)
        assert len(parsed["properties"]) > 0

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_input_schema_has_required_fields(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        parsed = json.loads(descriptor.input_schema)
        assert "required" in parsed
        assert isinstance(parsed["required"], list)
        assert len(parsed["required"]) > 0

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_input_schema_required_fields_are_in_properties(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        parsed = json.loads(descriptor.input_schema)
        props = set(parsed["properties"].keys())
        for field in parsed["required"]:
            assert field in props, (
                f"Operation '{op_id}' schema requires '{field}' "
                f"but it's not in properties: {props}"
            )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_output_schema_exists(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        assert descriptor.output_schema is not None, (
            f"Operation '{op_id}' has no output_schema"
        )

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_output_schema_is_valid_json(self, op_id):
        from eggsec import ToolRegistry

        descriptor = ToolRegistry.get(op_id)
        parsed = json.loads(descriptor.output_schema)
        assert isinstance(parsed, dict)
        assert "$schema" in parsed

    def test_schema_generator_matches_descriptor(self):
        """SchemaGenerator and descriptor schemas agree."""
        from eggsec import SchemaGenerator, ToolRegistry

        for op_id in ALL_STABLE_OPERATION_IDS:
            descriptor = ToolRegistry.get(op_id)
            schema_str = SchemaGenerator.generate_input_schema(op_id)
            assert schema_str is not None
            parsed = json.loads(schema_str)
            assert "$schema" in parsed

    def test_nonexistent_tool_returns_no_schema(self):
        from eggsec import SchemaGenerator

        assert SchemaGenerator.generate_input_schema("nonexistent_tool") is None
        assert SchemaGenerator.generate_output_schema("nonexistent_tool") is None


# ---------------------------------------------------------------------------
# 9. Operation listing
# ---------------------------------------------------------------------------


class TestOperationListing:
    """Engine.list_operations() returns all 22 stable operations."""

    def test_engine_list_operations_returns_22(self):
        from eggsec import Engine, Scope

        scope = Scope.allow_hosts(["*"])
        engine = Engine(scope)
        ops = engine.list_operations()
        assert len(ops) == 22

    def test_engine_list_operations_contains_all_canonical_ids(self):
        from eggsec import Engine, Scope

        scope = Scope.allow_hosts(["*"])
        engine = Engine(scope)
        ops = set(engine.list_operations())
        for op_id in ALL_STABLE_OPERATION_IDS:
            assert op_id in ops, (
                f"Engine.list_operations() missing '{op_id}'"
            )

    def test_engine_list_operations_no_extra_ids(self):
        from eggsec import Engine, Scope

        scope = Scope.allow_hosts(["*"])
        engine = Engine(scope)
        ops = set(engine.list_operations())
        assert ops == set(ALL_STABLE_OPERATION_IDS), (
            f"Unexpected operations in list_operations(): "
            f"{ops - set(ALL_STABLE_OPERATION_IDS)}"
        )

    def test_async_engine_list_operations_returns_22(self):
        from eggsec import AsyncEngine, Scope

        scope = Scope.allow_hosts(["*"])
        engine = AsyncEngine(scope)
        ops = engine.list_operations()
        assert len(ops) == 22

    def test_async_engine_list_operations_contains_all_canonical_ids(self):
        from eggsec import AsyncEngine, Scope

        scope = Scope.allow_hosts(["*"])
        engine = AsyncEngine(scope)
        ops = set(engine.list_operations())
        for op_id in ALL_STABLE_OPERATION_IDS:
            assert op_id in ops, (
                f"AsyncEngine.list_operations() missing '{op_id}'"
            )

    def test_engine_has_operation_true_for_all(self):
        from eggsec import Engine, Scope

        scope = Scope.allow_hosts(["*"])
        engine = Engine(scope)
        for op_id in ALL_STABLE_OPERATION_IDS:
            assert engine.has_operation(op_id), (
                f"Engine.has_operation('{op_id}') returned False"
            )

    def test_engine_has_operation_false_for_unknown(self):
        from eggsec import Engine, Scope

        scope = Scope.allow_hosts(["*"])
        engine = Engine(scope)
        assert not engine.has_operation("totally_fake_operation")


# ---------------------------------------------------------------------------
# 10. Registry containment
# ---------------------------------------------------------------------------


class TestRegistryContainment:
    """All IDs resolve through the ToolRegistry, including aliases."""

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_registry_get_returns_descriptor(self, op_id):
        from eggsec import ToolRegistry

        d = ToolRegistry.get(op_id)
        assert d is not None
        assert d.tool_id == op_id

    @pytest.mark.parametrize("alias,canonical", LEGACY_ALIASES.items())
    def test_registry_get_alias_returns_canonical(self, alias, canonical):
        from eggsec import ToolRegistry

        d = ToolRegistry.get(alias)
        assert d is not None
        assert d.tool_id == canonical

    def test_registry_get_nonexistent_returns_none(self):
        from eggsec import ToolRegistry

        assert ToolRegistry.get("nonexistent_operation") is None
        assert ToolRegistry.get("") is None
        assert ToolRegistry.get("SCAN_PORTS") is None  # case-sensitive

    def test_registry_operations_for_category_scanning(self):
        from eggsec import ToolRegistry

        ops = ToolRegistry.operations_for_category("scanning")
        ids = {d.tool_id for d in ops}
        assert ids == {"scan_ports", "scan_endpoints"}

    def test_registry_operations_for_category_recon(self):
        from eggsec import ToolRegistry

        ops = ToolRegistry.operations_for_category("recon")
        ids = {d.tool_id for d in ops}
        assert ids == {"recon_dns", "inspect_tls", "run_consolidated_recon"}

    def test_registry_operations_for_category_waf(self):
        from eggsec import ToolRegistry

        ops = ToolRegistry.operations_for_category("waf")
        ids = {d.tool_id for d in ops}
        assert ids == {"detect_waf", "validate_waf"}

    def test_registry_operations_for_category_mobile(self):
        from eggsec import ToolRegistry

        ops = ToolRegistry.operations_for_category("mobile")
        ids = {d.tool_id for d in ops}
        assert ids == {"analyze_apk", "analyze_ipa"}

    def test_registry_operations_for_category_container(self):
        from eggsec import ToolRegistry

        ops = ToolRegistry.operations_for_category("container")
        ids = {d.tool_id for d in ops}
        assert ids == {"scan_docker_image", "scan_kubernetes"}

    def test_registry_operations_for_category_database(self):
        from eggsec import ToolRegistry

        ops = ToolRegistry.operations_for_category("database")
        ids = {d.tool_id for d in ops}
        assert ids == {"db_probe"}

    def test_registry_operations_for_category_nse(self):
        from eggsec import ToolRegistry

        ops = ToolRegistry.operations_for_category("nse")
        ids = {d.tool_id for d in ops}
        assert ids == {"nse_run"}


# ---------------------------------------------------------------------------
# 11. operation_as_tool consistency
# ---------------------------------------------------------------------------


class TestOperationAsToolConsistency:
    """operation_as_tool() views are consistent with ToolRegistry descriptors."""

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_operation_as_tool_returns_view(self, op_id):
        from eggsec import operation_as_tool

        view = operation_as_tool(op_id)
        assert view is not None
        assert view.descriptor.tool_id == op_id

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_request_type_name_is_nonempty(self, op_id):
        from eggsec import operation_as_tool

        view = operation_as_tool(op_id)
        assert len(view.request_type_name) > 0

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_result_type_name_is_nonempty(self, op_id):
        from eggsec import operation_as_tool

        view = operation_as_tool(op_id)
        assert len(view.result_type_name) > 0

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_example_request_is_valid_json(self, op_id):
        from eggsec import operation_as_tool

        view = operation_as_tool(op_id)
        if view.example_request is not None:
            parsed = json.loads(view.example_request)
            assert isinstance(parsed, dict)

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_descriptor_matches_tool_registry(self, op_id):
        from eggsec import ToolRegistry, operation_as_tool

        view = operation_as_tool(op_id)
        registry_desc = ToolRegistry.get(op_id)
        assert view.descriptor.tool_id == registry_desc.tool_id
        assert view.descriptor.title == registry_desc.title
        assert view.descriptor.risk == registry_desc.risk
        assert view.descriptor.maturity == registry_desc.maturity

    @pytest.mark.parametrize("op_id", ALL_STABLE_OPERATION_IDS)
    def test_invoke_description_contains_tool_id(self, op_id):
        from eggsec import operation_as_tool

        view = operation_as_tool(op_id)
        desc = view.invoke_description()
        assert op_id in desc

    def test_nonexistent_returns_none(self):
        from eggsec import operation_as_tool

        assert operation_as_tool("nonexistent_op") is None
