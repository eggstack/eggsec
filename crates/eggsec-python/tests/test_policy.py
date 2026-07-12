"""Policy enforcement tests.

Verifies that ManualPermissive, McpStrict, AgentStrict, and CI surfaces
enforce scope, override, and authorization rules correctly.
"""

import pytest
from conftest import SENTINEL_TARGET

import eggsec
from eggsec import (
    Scope,
    ScopeSource,
    EnforcementContext,
    ExecutionPolicy,
    ManualOverride,
    ExecutionSurface,
    LoadedScope,
    OperationRegistry,
)


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------


def make_descriptor(target=SENTINEL_TARGET, risk="passive", requires_explicit_scope=True):
    return eggsec.OperationDescriptorPy(
        operation="scan-ports",
        mode="standard-assessment",
        risk=risk,
        intended_uses=["web-assessment"],
        target=target,
        requires_explicit_scope=requires_explicit_scope,
        required_capabilities=[],
    )


def sentinel_loaded_scope():
    return LoadedScope.explicit(
        Scope.allow_hosts([SENTINEL_TARGET]),
        ScopeSource.config_file(),
        None,
    )


def empty_loaded_scope():
    return LoadedScope.default_empty()


def default_policy():
    return ExecutionPolicy()


# ---------------------------------------------------------------------------
# 1. ManualPermissive allows all operations
# ---------------------------------------------------------------------------


class TestManualPermissive:
    def test_in_scope_allowed(self):
        ctx = EnforcementContext.manual_permissive(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target=SENTINEL_TARGET)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True

    def test_out_of_scope_gets_confirmation_required(self):
        ctx = EnforcementContext.manual_permissive(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target="other.example.com")
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False
        assert outcome.requires_confirmation is True

    def test_empty_scope_allows_when_not_explicit(self):
        ctx = EnforcementContext.manual_permissive(default_policy(), empty_loaded_scope())
        desc = make_descriptor(target="anything.example.com", requires_explicit_scope=False)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True


# ---------------------------------------------------------------------------
# 2. McpStrict denies manual overrides
# ---------------------------------------------------------------------------


class TestMcpStrict:
    def test_deny_on_scope_miss(self):
        ctx = EnforcementContext.mcp_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target="other.example.com")
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False

    def test_allow_in_scope(self):
        ctx = EnforcementContext.mcp_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target=SENTINEL_TARGET)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True

    def test_manual_override_rejected(self):
        ctx = EnforcementContext.mcp_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target="other.example.com")
        mo = ManualOverride(
            assume_yes=True,
            allow_out_of_scope=True,
            reason="test-override",
        )
        with pytest.raises(Exception):
            ctx.approve_manual(ExecutionSurface.mcp_server(), desc, mo)

    def test_surface_is_not_manual(self):
        assert ExecutionSurface.mcp_server().is_manual is False


# ---------------------------------------------------------------------------
# 3. AgentStrict requires scope manifest
# ---------------------------------------------------------------------------


class TestAgentStrict:
    def test_deny_on_scope_miss(self):
        ctx = EnforcementContext.agent_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target="other.example.com")
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False

    def test_allow_in_scope(self):
        ctx = EnforcementContext.agent_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target=SENTINEL_TARGET)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True

    def test_manual_override_rejected(self):
        ctx = EnforcementContext.agent_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target="other.example.com")
        mo = ManualOverride(
            assume_yes=True,
            allow_out_of_scope=True,
            reason="test-override",
        )
        with pytest.raises(Exception):
            ctx.approve_manual(ExecutionSurface.security_agent(), desc, mo)

    def test_surface_is_agent_controlled(self):
        assert ExecutionSurface.security_agent().is_agent_controlled is True


# ---------------------------------------------------------------------------
# 4. Manual override restrictions are enforced
# ---------------------------------------------------------------------------


class TestManualOverrideRestrictions:
    def test_override_permits_out_of_scope(self):
        mo = ManualOverride(
            assume_yes=True,
            allow_out_of_scope=True,
            reason="testing",
        )
        assert mo.permits("out-of-scope") is True

    def test_override_permits_high_risk(self):
        mo = ManualOverride(
            assume_yes=True,
            allow_high_risk=True,
            reason="testing",
        )
        assert mo.permits("high-risk") is True

    def test_override_does_not_permits_unspecified(self):
        mo = ManualOverride(
            assume_yes=True,
            allow_out_of_scope=True,
            reason="testing",
        )
        assert mo.permits("scope-missing") is False

    def test_override_not_honored_on_ci(self):
        ctx = EnforcementContext.ci_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target="other.example.com")
        mo = ManualOverride(
            assume_yes=True,
            allow_out_of_scope=True,
            reason="test-override",
        )
        with pytest.raises(Exception):
            ctx.approve_manual(ExecutionSurface.ci(), desc, mo)


# ---------------------------------------------------------------------------
# 5. Authorization levels are respected
# ---------------------------------------------------------------------------


class TestAuthorizationLevels:
    def test_ci_strict_deny(self):
        ctx = EnforcementContext.ci_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target="other.example.com")
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is False

    def test_ci_strict_allow(self):
        ctx = EnforcementContext.ci_strict(default_policy(), sentinel_loaded_scope())
        desc = make_descriptor(target=SENTINEL_TARGET)
        outcome = ctx.evaluate(desc)
        assert outcome.is_allowed is True

    def test_cli_manual_is_manual(self):
        assert ExecutionSurface.cli_manual().is_manual is True

    def test_tui_manual_is_manual(self):
        assert ExecutionSurface.tui_manual().is_manual is True

    def test_all_surfaces_have_label(self):
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
            assert len(s.label) > 0

    def test_operation_risk_ordering(self):
        r0 = OperationRegistry.find("scan-ports")
        assert r0 is not None
        passive = r0.default_risk
        assert passive is not None
