"""Release 5 Phase A: gap-closure tests.

Covers:
- A6: Schema snapshot determinism
- A7: Framework adapter (OpenAPI) correctness
- A8: Feature unavailability, scope/policy denial parity,
      sync/async normalization, secret sentinel audit.
"""

import json
import os
from pathlib import Path

import pytest


# ---------------------------------------------------------------------------
# A6: Schema snapshot determinism
# ---------------------------------------------------------------------------

class TestSchemaSnapshotDeterminism:
    """Verify that schema generation is deterministic across runs."""

    def test_request_schema_deterministic(self):
        from eggsec import SchemaGenerator
        schema_a = SchemaGenerator.generate_input_schema("scan_ports")
        schema_b = SchemaGenerator.generate_input_schema("scan_ports")
        assert schema_a is not None
        assert schema_a == schema_b

    def test_response_schema_deterministic(self):
        from eggsec import SchemaGenerator
        schema_a = SchemaGenerator.generate_output_schema("scan_ports")
        schema_b = SchemaGenerator.generate_output_schema("scan_ports")
        assert schema_a is not None
        assert schema_a == schema_b

    def test_all_operations_have_schemas(self):
        from eggsec import SchemaGenerator, ToolRegistry
        tools = ToolRegistry.list()
        missing = []
        for desc in tools:
            inp = SchemaGenerator.generate_input_schema(desc.tool_id)
            out = SchemaGenerator.generate_output_schema(desc.tool_id)
            if inp is None:
                missing.append(f"{desc.tool_id}:input")
            if out is None:
                missing.append(f"{desc.tool_id}:output")
        assert missing == [], f"Missing schemas: {missing}"

    def test_schema_json_is_valid(self):
        from eggsec import SchemaGenerator
        raw = SchemaGenerator.generate_input_schema("scan_ports")
        assert raw is not None
        parsed = json.loads(raw)
        assert "$schema" in parsed or "type" in parsed

    def test_all_schemas_method(self):
        from eggsec import SchemaGenerator
        all_schemas = SchemaGenerator.all_schemas()
        assert isinstance(all_schemas, dict)
        assert "scan_ports" in all_schemas
        assert "input_schema" in all_schemas["scan_ports"]
        assert "output_schema" in all_schemas["scan_ports"]


# ---------------------------------------------------------------------------
# A6: Schema fixture comparison (if fixtures exist)
# ---------------------------------------------------------------------------

class TestSchemaFixtures:
    """Compare generated schemas against checked-in fixtures."""

    FIXTURE_DIR = Path(__file__).resolve().parent.parent / "fixtures" / "schema"

    @pytest.mark.skipif(
        not FIXTURE_DIR.exists(),
        reason="Schema fixtures directory not found"
    )
    def test_fixture_count_matches_registry(self):
        from eggsec import ToolRegistry
        fixture_files = list(self.FIXTURE_DIR.glob("*.json"))
        tools = ToolRegistry.list()
        assert len(fixture_files) == len(tools), (
            f"Fixture count ({len(fixture_files)}) != tool count ({len(tools)})"
        )

    @pytest.mark.skipif(
        not FIXTURE_DIR.exists(),
        reason="Schema fixtures directory not found"
    )
    def test_fixtures_match_generated(self):
        from eggsec import SchemaGenerator
        fixture_files = list(self.FIXTURE_DIR.glob("*.json"))
        mismatches = []
        for fixture_path in sorted(fixture_files):
            tool_id = fixture_path.stem
            with open(fixture_path) as f:
                fixture = json.load(f)
            gen_input = SchemaGenerator.generate_input_schema(tool_id)
            gen_output = SchemaGenerator.generate_output_schema(tool_id)
            if gen_input is not None:
                gen_input_parsed = json.loads(gen_input)
                if fixture.get("input_schema") != gen_input_parsed:
                    mismatches.append(f"{tool_id}:input_schema")
            if gen_output is not None:
                gen_output_parsed = json.loads(gen_output)
                if fixture.get("output_schema") != gen_output_parsed:
                    mismatches.append(f"{tool_id}:output_schema")
        assert mismatches == [], f"Schema mismatches: {mismatches}"


# ---------------------------------------------------------------------------
# A7: OpenAPI adapter
# ---------------------------------------------------------------------------

class TestOpenApiAdapter:
    """Verify OpenAPI derived adapter produces valid output."""

    def test_tool_to_openapi(self):
        from eggsec import OpenApiAdapter
        result = OpenApiAdapter.tool_to_openapi("scan_ports")
        assert result is not None
        assert "post" in result
        op = result["post"]
        assert op["operationId"] == "scan_ports"
        assert "summary" in op
        assert "tags" in op
        assert "parameters" in op

    def test_tool_to_openapi_unknown(self):
        from eggsec import OpenApiAdapter
        result = OpenApiAdapter.tool_to_openapi("nonexistent_tool")
        assert result is None

    def test_full_openapi_spec(self):
        from eggsec import OpenApiAdapter, ToolRegistry
        spec = OpenApiAdapter.full_openapi_spec()
        assert isinstance(spec, dict)
        assert spec["openapi"] == "3.0.3"
        assert "info" in spec
        assert "paths" in spec
        tools = ToolRegistry.list()
        assert len(spec["paths"]) == len(tools)

    def test_openapi_has_risk_metadata(self):
        from eggsec import OpenApiAdapter
        result = OpenApiAdapter.tool_to_openapi("fuzz_http")
        assert result is not None
        op = result["post"]
        assert "x-eggsec-risk" in op
        assert op["x-eggsec-risk"] == "intrusive"

    def test_openapi_confirmation_metadata(self):
        from eggsec import OpenApiAdapter
        result = OpenApiAdapter.tool_to_openapi("nse_run")
        assert result is not None
        op = result["post"]
        assert "x-eggsec-confirmation-required" in op
        assert op["x-eggsec-confirmation-required"] is True


# ---------------------------------------------------------------------------
# A8: Feature-gated operations discoverable but report unavailability
# ---------------------------------------------------------------------------

class TestFeatureUnavailability:
    """Feature-gated operations should be discoverable but report structured
    unavailability when the feature is not compiled."""

    def test_all_operations_in_registry(self):
        from eggsec import ToolRegistry
        tools = ToolRegistry.list()
        tool_ids = {t.tool_id for t in tools}
        # All 22 stable operations should be present regardless of features
        expected = {
            "scan_ports", "scan_endpoints", "fingerprint_services",
            "recon_dns", "inspect_tls", "detect_technology",
            "detect_waf", "validate_waf", "fuzz_http", "load_test",
            "scan_git_secrets", "generate_sbom",
            "run_consolidated_recon", "graphql_test", "oauth_test",
            "auth_test", "db_probe", "nse_run",
            "scan_docker_image", "scan_kubernetes",
            "analyze_apk", "analyze_ipa",
        }
        missing = expected - tool_ids
        assert not missing, f"Missing operations: {missing}"

    def test_feature_gated_descriptor_has_feature(self):
        from eggsec import ToolRegistry
        # These operations require specific features
        feature_ops = {
            "db_probe": "db-pentest",
            "nse_run": "nse",
            "analyze_apk": "mobile",
            "analyze_ipa": "mobile",
        }
        for tool_id, expected_feature in feature_ops.items():
            desc = ToolRegistry.get(tool_id)
            assert desc is not None, f"Descriptor not found for {tool_id}"
            assert desc.feature_required == expected_feature, (
                f"{tool_id}: expected feature '{expected_feature}', got '{desc.feature_required}'"
            )

    def test_operations_for_feature(self):
        from eggsec import ToolRegistry
        db_ops = ToolRegistry.operations_for_feature("db-pentest")
        assert len(db_ops) >= 1
        assert any(t.tool_id == "db_probe" for t in db_ops)

    def test_validate_rejects_missing_required(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {})
        assert not report.valid
        assert any("target" in e for e in report.errors)

    def test_validate_warns_unknown_fields(self):
        from eggsec import ToolRegistry
        report = ToolRegistry.validate("scan_ports", {"target": "127.0.0.1", "unknown_field": True})
        assert report.valid
        assert any("unknown_field" in w for w in report.warnings)


# ---------------------------------------------------------------------------
# A8: Scope/policy denial parity
# ---------------------------------------------------------------------------

class TestScopePolicyDenialParity:
    """Scope and policy denials must be identical through operation and
    tool invocation paths."""

    def test_scope_denial_via_run(self):
        from eggsec import Engine, Scope, OperationRequest
        scope = Scope.allow_hosts(["192.0.2.1"])  # TEST-NET, not reachable
        engine = Engine(scope)
        req = OperationRequest("scan_ports", "10.0.0.1", timeout_ms=1000)
        result = engine.run(req)
        assert result.error is not None
        assert "scope" in result.error.kind.lower() or result.error.kind == "scope_denial"

    def test_scope_denial_via_invoke_tool(self):
        from eggsec import Engine, Scope
        scope = Scope.allow_hosts(["192.0.2.1"])
        engine = Engine(scope)
        result = engine.invoke_tool("scan_ports", "10.0.0.1", timeout_ms=1000)
        assert result.error is not None
        assert "scope" in result.error.kind.lower() or result.error.kind == "scope_denial"

    def test_scope_denial_via_invoke_tool_request(self):
        from eggsec import Engine, Scope, ToolRequest, ToolTarget
        scope = Scope.allow_hosts(["192.0.2.1"])
        engine = Engine(scope)
        target = ToolTarget.ip("10.0.0.1")
        request = ToolRequest.new(tool="scan_ports", target=target)
        result = engine.invoke_tool_request(request)
        assert result.error is not None
        assert "scope" in result.error.kind.lower() or result.error.kind == "scope_denial"


# ---------------------------------------------------------------------------
# A8: Sync/async response normalization
# ---------------------------------------------------------------------------

class TestSyncAsyncNormalization:
    """Sync and async tool responses should normalize identically."""

    def test_invoke_tool_returns_operation_result(self):
        from eggsec import Engine, Scope
        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        result = engine.invoke_tool("scan_ports", "127.0.0.1", timeout_ms=2000)
        # Should return an OperationResult with standard fields
        assert hasattr(result, "status")
        assert hasattr(result, "error")
        assert hasattr(result, "metadata")
        assert hasattr(result, "artifacts")

    def test_invoke_tool_request_returns_operation_result(self):
        from eggsec import Engine, Scope, ToolRequest, ToolTarget
        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = Engine(scope)
        target = ToolTarget.ip("127.0.0.1")
        request = ToolRequest.new(tool="scan_ports", target=target)
        result = engine.invoke_tool_request(request)
        assert hasattr(result, "status")
        assert hasattr(result, "error")

    def test_async_invoke_tool_returns_future(self):
        from eggsec import AsyncEngine, Scope
        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)
        future = engine.async_invoke_tool("scan_ports", "127.0.0.1", timeout_ms=2000)
        assert future is not None

    def test_async_invoke_tool_request_returns_future(self):
        from eggsec import AsyncEngine, Scope, ToolRequest, ToolTarget
        scope = Scope.allow_hosts(["127.0.0.1"])
        engine = AsyncEngine(scope)
        target = ToolTarget.ip("127.0.0.1")
        request = ToolRequest.new(tool="scan_ports", target=target)
        future = engine.async_invoke_tool_request(request)
        assert future is not None


# ---------------------------------------------------------------------------
# A8: Secret sentinel audit for tool surfaces
# ---------------------------------------------------------------------------

class TestSecretSentinelAudit:
    """Verify that no secret-bearing values leak through tool-core surfaces."""

    def test_auth_config_redacted_in_repr(self):
        from eggsec import ToolAuthConfig
        cfg = ToolAuthConfig.api_key("sk-secret123", "Authorization")
        r = repr(cfg)
        assert "sk-secret123" not in r
        assert "[REDACTED]" in r

    def test_auth_config_redacted_in_str(self):
        from eggsec import ToolAuthConfig
        cfg = ToolAuthConfig.bearer("tok_abc123")
        s = str(cfg)
        assert "tok_abc123" not in s
        assert "[REDACTED]" in s

    def test_auth_config_redacted_in_to_dict(self):
        from eggsec import ToolAuthConfig
        cfg = ToolAuthConfig.api_key("secret-key-42", "X-Api-Key")
        d = cfg.to_dict()
        # All credential values should be redacted
        for v in d["credentials"].values():
            assert v == "[REDACTED]"

    def test_auth_config_redacted_in_to_json(self):
        from eggsec import ToolAuthConfig
        cfg = ToolAuthConfig.basic("admin", "hunter2")
        j = cfg.to_json()
        parsed = json.loads(j)
        # credentials should only have keys, not values
        assert isinstance(parsed["credentials"], list)
        assert "username" in parsed["credentials"]

    def test_descriptor_no_credential_values(self):
        from eggsec import ToolRegistry
        desc = ToolRegistry.get("scan_ports")
        assert desc is not None
        d = desc.to_dict()
        j = desc.to_json()
        # No credential values should appear
        for val in d.values():
            if isinstance(val, str):
                assert "sk-" not in val
                assert "password" not in val.lower() or val in ("stable", "safe_active", "scanning", "target-required")
