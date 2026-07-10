"""Policy equivalence tests (B9).

Verifies that Python bindings produce the same enforcement outcomes
as the Rust library. Tests mirror the enforcement_matrix.rs patterns.
"""

import pytest
from eggsec import (
    Scope,
    ScopeSource,
    EnforcementContext,
    ExecutionPolicy,
    ManualOverride,
    OperationRisk,
    ExecutionSurface,
    OperationRegistry,
    SensitiveString,
    LoadedScope,
    AuditOutcome,
    OperationDescriptorPy,
    EggsecConfig,
    preflight_with_descriptor,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_descriptor(
    target="example.com",
    risk="passive",
    operation_id="test-op",
    requires_explicit_scope=False,
    required_capabilities=None,
):
    """Build an OperationDescriptorPy via execution_context constructor."""
    return OperationDescriptorPy(
        operation=operation_id,
        mode="standard-assessment",
        risk=risk,
        intended_uses=["web-assessment"],
        target=target,
        requires_explicit_scope=requires_explicit_scope,
        required_capabilities=required_capabilities or [],
    )


def scope_allow(pattern="example.com"):
    return Scope.allow_hosts([pattern])


def loaded_scope(pattern="example.com"):
    return LoadedScope.explicit(scope_allow(pattern), ScopeSource.config_file(), None)


def empty_scope():
    return LoadedScope.default_empty()


def default_policy():
    return ExecutionPolicy()


# ---------------------------------------------------------------------------
# 1. Surface -> profile mapping
# ---------------------------------------------------------------------------


class TestSurfaceProfileMapping:
    def test_cli_manual_is_manual(self):
        assert ExecutionSurface.cli_manual().is_manual is True

    def test_mcp_server_not_manual(self):
        assert ExecutionSurface.mcp_server().is_manual is False

    def test_agent_is_agent_controlled(self):
        assert ExecutionSurface.security_agent().is_agent_controlled is True

    def test_ci_not_manual(self):
        assert ExecutionSurface.ci().is_manual is False

    def test_all_surfaces_accessible(self):
        surfaces = [
            ExecutionSurface.cli_manual(),
            ExecutionSurface.tui_manual(),
            ExecutionSurface.mcp_server(),
            ExecutionSurface.security_agent(),
            ExecutionSurface.ci(),
            ExecutionSurface.rest_api(),
        ]
        for s in surfaces:
            assert s.label is not None


# ---------------------------------------------------------------------------
# 2. Manual surfaces allow in-scope, deny out-of-scope (requires_explicit_scope)
# ---------------------------------------------------------------------------


class TestManualPermissiveScope:
    def test_in_scope_target_allow_on_cli(self):
        ctx = EnforcementContext.manual_permissive(default_policy(), loaded_scope())
        desc = make_descriptor(target="example.com", requires_explicit_scope=True)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True

    def test_out_of_scope_target_not_allowed_with_explicit_scope(self):
        ctx = EnforcementContext.manual_permissive(default_policy(), loaded_scope())
        desc = make_descriptor(target="other.example.com", requires_explicit_scope=True)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False
        assert outcome.requires_confirmation is True

    def test_empty_scope_allows_when_not_explicit(self):
        ctx = EnforcementContext.manual_permissive(default_policy(), empty_scope())
        desc = make_descriptor(target="other.example.com")
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True


class TestStrictSurfacesDeny:
    def test_mcp_deny_on_scope_miss(self):
        ctx = EnforcementContext.mcp_strict(default_policy(), loaded_scope())
        desc = make_descriptor(target="other.example.com", requires_explicit_scope=True)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False

    def test_agent_deny_on_scope_miss(self):
        ctx = EnforcementContext.agent_strict(default_policy(), loaded_scope())
        desc = make_descriptor(target="other.example.com", requires_explicit_scope=True)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False

    def test_ci_deny_on_scope_miss(self):
        ctx = EnforcementContext.ci_strict(default_policy(), loaded_scope())
        desc = make_descriptor(target="other.example.com", requires_explicit_scope=True)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False

    def test_mcp_allow_in_scope(self):
        ctx = EnforcementContext.mcp_strict(default_policy(), loaded_scope())
        desc = make_descriptor(target="example.com", requires_explicit_scope=True)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True


# ---------------------------------------------------------------------------
# 3. Manual override only honored on CliManual/TuiManual
# ---------------------------------------------------------------------------


class TestManualOverride:
    def test_override_permits_confirmation_class(self):
        mo = ManualOverride(
            assume_yes=True,
            allow_out_of_scope=True,
            allow_high_risk=True,
            reason="testing override",
        )
        assert mo.permits("out-of-scope") is True
        assert mo.permits("high-risk") is True
        assert mo.permits("scope-missing") is False

    def test_override_not_honored_on_strict(self):
        mo = ManualOverride(
            assume_yes=True,
            allow_out_of_scope=True,
            reason="testing",
        )
        ctx = EnforcementContext.mcp_strict(default_policy(), loaded_scope())
        desc = make_descriptor(target="other.example.com", requires_explicit_scope=True)
        with pytest.raises(Exception):
            ctx.approve_manual(ExecutionSurface.mcp_server(), desc, mo)


# ---------------------------------------------------------------------------
# 4. Secret redaction
# ---------------------------------------------------------------------------


class TestSecretRedaction:
    def test_sensitive_string_repr_is_redacted(self):
        s = SensitiveString("my-super-secret-key")
        assert "my-super-secret-key" not in repr(s)
        assert "my-super-secret-key" not in str(s)

    def test_sensitive_string_expose_secret(self):
        s = SensitiveString("actual-secret")
        assert s.expose_secret() == "actual-secret"

    def test_sensitive_string_is_empty(self):
        s = SensitiveString("")
        assert s.is_empty() is True
        s2 = SensitiveString("not-empty")
        assert s2.is_empty() is False

    def test_sensitive_string_len(self):
        s = SensitiveString("secret123")
        assert s.len() == 9

    def test_sensitive_string_equality(self):
        a = SensitiveString("same")
        b = SensitiveString("same")
        assert a == b


# ---------------------------------------------------------------------------
# 5. Operation metadata coverage
# ---------------------------------------------------------------------------


class TestOperationMetadata:
    def test_all_operations_discoverable(self):
        ops = OperationRegistry.all_operations()
        assert len(ops) >= 29

    def test_find_by_id(self):
        op = OperationRegistry.find("scan-ports")
        assert op is not None
        assert op.operation_id == "scan-ports"

    def test_find_by_tool_id_alias(self):
        op = OperationRegistry.find_by_tool_id("scan")
        assert op is not None

    def test_descriptor_for_target(self):
        op = OperationRegistry.find("scan-ports")
        assert op is not None
        desc = op.descriptor_for_target("example.com")
        assert desc is not None
        assert desc.target == "example.com"

    def test_operation_risk_levels(self):
        r0 = OperationRisk.passive()
        r1 = OperationRisk.agent_autonomous()
        assert r0.level == 0
        assert r1.level == 14
        assert r1.level > r0.level


# ---------------------------------------------------------------------------
# 6. Preflight determinism
# ---------------------------------------------------------------------------


class TestPreflightDeterminism:
    def test_preflight_in_scope_allows(self):
        desc = make_descriptor(target="example.com")
        result = preflight_with_descriptor(
            desc,
            loaded_scope(),
            default_policy(),
            ExecutionSurface.cli_manual(),
        )
        assert result.outcome == "allow"

    def test_preflight_out_of_scope_not_allow(self):
        desc = make_descriptor(target="other.example.com", requires_explicit_scope=True)
        result = preflight_with_descriptor(
            desc,
            loaded_scope(),
            default_policy(),
            ExecutionSurface.cli_manual(),
        )
        assert result.outcome != "allow"

    def test_preflight_includes_cli_flags(self):
        desc = make_descriptor(target="other.example.com", requires_explicit_scope=True)
        result = preflight_with_descriptor(
            desc,
            loaded_scope(),
            default_policy(),
            ExecutionSurface.cli_manual(),
        )
        assert isinstance(result.suggested_cli_flags, list)


# ---------------------------------------------------------------------------
# 7. Audit event coverage
# ---------------------------------------------------------------------------


class TestAuditEvents:
    def test_audit_outcome_variants(self):
        assert AuditOutcome.allow().name == "allow"
        assert AuditOutcome.warn().name == "warn"
        assert AuditOutcome.confirmed().name == "confirmed"
        assert AuditOutcome.deny().name == "deny"
        assert AuditOutcome.confirmation_required().name == "confirmation-required"


# ---------------------------------------------------------------------------
# 8. Scope enforcement consistency
# ---------------------------------------------------------------------------


class TestScopeConsistency:
    def test_loaded_scope_inherits_target_check(self):
        ls = loaded_scope("example.com")
        assert ls.is_target_allowed("example.com") is True
        assert ls.is_target_allowed("evil.com") is False

    def test_scope_explanation_provided(self):
        ls = loaded_scope("example.com")
        result = ls.explain("example.com")
        assert result is not None
        assert result.allowed is True

    def test_scope_source_tracking(self):
        ls = LoadedScope.explicit(
            scope_allow(), ScopeSource.cli_scope_file(), "/tmp/scope.toml"
        )
        assert ls.source.name == "cli-scope-file"


# ---------------------------------------------------------------------------
# 9. Policy capability checks
# ---------------------------------------------------------------------------


class TestPolicyCapabilities:
    def test_default_policy_no_capabilities(self):
        p = default_policy()
        assert len(p.allowed_capabilities) == 0
        assert len(p.denied_capabilities) == 0

    def test_policy_from_config_roundtrip(self):
        cfg = EggsecConfig.default()
        p = ExecutionPolicy.from_config(cfg)
        assert p is not None


# ---------------------------------------------------------------------------
# 10. Configuration model roundtrip
# ---------------------------------------------------------------------------


class TestConfigRoundtrip:
    def test_default_config_loads(self):
        cfg = EggsecConfig.default()
        assert cfg is not None

    def test_config_validate(self):
        cfg = EggsecConfig.default()
        cfg.validate()  # returns None on success, raises on failure

    def test_config_has_fields(self):
        cfg = EggsecConfig.default()
        assert cfg.http is not None
        assert cfg.auto_save_interval_secs >= 0
