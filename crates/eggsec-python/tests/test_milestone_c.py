"""Tests for Milestone C Python bindings.

These tests verify construction of Python-facing types and basic API surface.
Network-dependent tests (actual scanning) are marked as `network` and excluded
from default test runs.
"""

import pytest


# ── Consolidated Recon ────────────────────────────────────────────────

class TestConsolidatedReconConfig:
    def test_default_config(self):
        from eggsec import ConsolidatedReconConfig

        cfg = ConsolidatedReconConfig()
        assert cfg.run_dns is True
        assert cfg.run_ssl is True
        assert cfg.run_tech_detect is True
        assert cfg.run_subdomain is True
        assert cfg.run_whois is True
        assert cfg.run_cors is True
        assert cfg.run_wayback is True
        assert cfg.run_js_analysis is True
        assert cfg.run_content is True
        assert cfg.run_email is True

    def test_selective_modules(self):
        from eggsec import ConsolidatedReconConfig

        cfg = ConsolidatedReconConfig(
            run_dns=True,
            run_ssl=True,
            run_subdomain=False,
            run_whois=False,
            run_cors=False,
            run_wayback=False,
            run_js_analysis=False,
            run_content=False,
            run_email=False,
            run_tech_detect=False,
        )
        assert cfg.run_dns is True
        assert cfg.run_subdomain is False

    def test_all_disabled(self):
        from eggsec import ConsolidatedReconConfig

        cfg = ConsolidatedReconConfig(
            run_dns=False,
            run_ssl=False,
            run_tech_detect=False,
            run_subdomain=False,
            run_whois=False,
            run_cors=False,
            run_wayback=False,
            run_js_analysis=False,
            run_content=False,
            run_email=False,
        )
        assert cfg.run_dns is False


# ── GraphQL ───────────────────────────────────────────────────────────

class TestGraphQLTestConfig:
    def test_minimal_config(self):
        from eggsec import GraphQLTestConfig

        cfg = GraphQLTestConfig(endpoint="https://example.com/graphql")
        assert cfg.endpoint == "https://example.com/graphql"
        assert cfg.enable_introspection is True
        assert cfg.enable_depth_bypass is True
        assert cfg.enable_alias_overload is True
        assert cfg.timeout_secs == 10

    def test_custom_config(self):
        from eggsec import GraphQLTestConfig

        cfg = GraphQLTestConfig(
            endpoint="https://api.example.com/gql",
            enable_introspection=False,
            enable_depth_bypass=False,
            enable_alias_overload=False,
            timeout_secs=60,
        )
        assert cfg.enable_introspection is False
        assert cfg.timeout_secs == 60


# ── OAuth/OIDC ────────────────────────────────────────────────────────

class TestOAuthTestConfig:
    def test_minimal_config(self):
        from eggsec import OAuthTestConfig

        cfg = OAuthTestConfig(
            client_id="test-client",
            redirect_uri="https://example.com/callback",
        )
        assert cfg.client_id == "test-client"
        assert cfg.enable_redirect_test is True
        assert cfg.enable_scope_test is True
        assert cfg.enable_state_test is True
        assert cfg.enable_grant_test is True

    def test_custom_config(self):
        from eggsec import OAuthTestConfig

        cfg = OAuthTestConfig(
            client_id="id",
            redirect_uri="https://callback.example.com",
            issuer_url="https://auth.example.com",
            client_secret="secret",
            enable_redirect_test=False,
            enable_scope_test=False,
        )
        assert cfg.issuer_url == "https://auth.example.com"
        assert cfg.enable_redirect_test is False


# ── Auth Assessment ───────────────────────────────────────────────────

class TestAuthTestConfig:
    def test_default_config(self):
        from eggsec import AuthTestConfig

        cfg = AuthTestConfig()
        assert cfg.max_attempts == 100
        assert cfg.concurrency == 10
        assert cfg.stop_on_lockout is True
        assert cfg.timeout_secs == 30

    def test_custom_config(self):
        from eggsec import AuthTestConfig

        cfg = AuthTestConfig(
            max_attempts=50,
            concurrency=5,
            timeout_secs=60,
            stop_on_lockout=False,
            usernames=["admin", "user"],
            passwords=["pass1", "pass2"],
        )
        assert cfg.max_attempts == 50
        assert cfg.usernames == ["admin", "user"]


# ── Browser Assessment (feature-gated) ────────────────────────────────

class TestBrowserTestConfig:
    def test_default_config(self):
        from eggsec import BrowserTestConfig

        cfg = BrowserTestConfig()
        assert cfg.check_dom_xss is True
        assert cfg.discover_spa_routes is True
        assert cfg.check_client_security is True
        assert cfg.timeout_ms == 30000
        assert "alert" in cfg.xss_payload

    def test_custom_config(self):
        from eggsec import BrowserTestConfig

        cfg = BrowserTestConfig(
            check_dom_xss=False,
            timeout_ms=60000,
            xss_payload="<script>alert('xss')</script>",
        )
        assert cfg.check_dom_xss is False
        assert cfg.timeout_ms == 60000


# ── Advanced Hunting (feature-gated) ──────────────────────────────────

class TestHuntTestConfig:
    def test_default_config(self):
        from eggsec import HuntTestConfig

        cfg = HuntTestConfig()
        assert cfg.check_attack_chains is True
        assert cfg.check_business_logic is True
        assert cfg.check_race_conditions is True
        assert cfg.check_authz_bypass is True
        assert cfg.check_session is True
        assert cfg.concurrency == 10
        assert cfg.timeout_ms == 10000

    def test_custom_config(self):
        from eggsec import HuntTestConfig

        cfg = HuntTestConfig(
            check_attack_chains=False,
            concurrency=20,
            timeout_ms=30000,
        )
        assert cfg.check_attack_chains is False
        assert cfg.concurrency == 20


# ── Import verification ───────────────────────────────────────────────

class TestImports:
    """Verify all Milestone C types and functions are importable."""

    def test_consolidated_recon_types(self):
        from eggsec import (
            ConsolidatedReconConfig,
            ConsolidatedReconReport,
            ReconModuleResult,
            run_consolidated_recon,
            async_run_consolidated_recon,
        )
        assert callable(run_consolidated_recon)
        assert callable(async_run_consolidated_recon)

    def test_graphql_types(self):
        from eggsec import (
            GraphQLVulnerability,
            GraphQLTestResult,
            GraphQLType,
            GraphQLField,
            GraphQLArg,
            GraphQLInputField,
            GraphQLSchema,
            GraphQLTestConfig,
            graphql_test,
            async_graphql_test,
        )
        assert callable(graphql_test)
        assert callable(async_graphql_test)

    def test_oauth_types(self):
        from eggsec import (
            OAuthVulnerability,
            OAuthEndpointKind,
            OAuthEndpoint,
            OAuthTestResult,
            OAuthTestConfig,
            oauth_discover_endpoints,
            oauth_test,
            async_oauth_test,
        )
        assert callable(oauth_discover_endpoints)
        assert callable(oauth_test)
        assert callable(async_oauth_test)

    def test_auth_types(self):
        from eggsec import (
            AuthTestType,
            AuthFinding,
            AuthTestConfig,
            AuthTestReport,
            auth_test,
            async_auth_test,
        )
        assert callable(auth_test)
        assert callable(async_auth_test)

    def test_browser_types(self):
        from eggsec import (
            XssSource,
            XssSink,
            DomXssFinding,
            DiscoveryMethod,
            SpaRoute,
            ClientIssueType,
            ClientIssue,
            BrowserTestConfig,
            BrowserTestReport,
            browser_test,
            async_browser_test,
        )
        assert callable(browser_test)
        assert callable(async_browser_test)

    def test_hunt_types(self):
        from eggsec import (
            ChainType,
            ChainStep,
            AttackChain,
            FlawType,
            BusinessLogicFlaw,
            RaceType,
            RaceCondition,
            BypassType,
            AuthzBypass,
            SessionIssueType,
            SessionIssue,
            HuntTestConfig,
            HuntReport,
            hunt_test,
            async_hunt_test,
        )
        assert callable(hunt_test)
        assert callable(async_hunt_test)

    def test_function_signatures(self):
        """Verify that key functions have expected signatures."""
        import inspect
        from eggsec import run_consolidated_recon, graphql_test, oauth_test

        # All should be regular functions (not coroutines)
        assert not inspect.iscoroutinefunction(run_consolidated_recon)
        assert not inspect.iscoroutinefunction(graphql_test)
        assert not inspect.iscoroutinefunction(oauth_test)
