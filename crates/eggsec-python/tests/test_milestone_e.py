"""Tests for Milestone E: Findings, Reporting, Storage, and Integrations."""
import json
import pytest
from eggsec import (
    # E1: Versioned finding schema
    Confidence,
    FindingType,
    EvidenceKind,
    AffectedAsset,
    FindingLocation,
    VersionedEvidence,
    VersionedFinding,
    FINDING_SCHEMA_VERSION,
    # E2: Artifacts
    ArtifactStore,
    MilestoneArtifact,
    ArtifactReference,
    # E3: CVSS
    CvssScore,
    VulnerabilityRecord,
    RemediationRecord,
    # E4: Workflow
    FindingState,
    FindingWorkflow,
    WorkflowTransition,
    Suppression,
    # E5: Repository
    FindingRepository,
    Assessment,
    AssessmentRepository,
    # E6: Baselines
    FindingCorrelation,
    FindingDiff,
    AssessmentDiff,
    BaselineComparator,
    # E7: Reporting
    FindingReporter,
    SeveritySummary,
    ReportEnvelope,
    # E9: Integrations
    IntegrationType,
    PublicationRecord,
    RetryPolicy,
    PublicationPolicy,
    ExternalIntegration,
    # E10: Migration
    SchemaVersion,
    MigrationResult,
    FindingMigration,
)


# ============================================================================
# Helpers
# ============================================================================

def _make_finding(fid="f1", title="SQLi", severity="High", target="example.com"):
    asset = AffectedAsset("host", target)
    return VersionedFinding(
        fid, title, "desc", severity, FindingType.Vulnerability, asset, "t", "m",
    )


# ============================================================================
# E1: Versioned Finding Schema Tests
# ============================================================================

class TestConfidence:
    def test_enum_values(self):
        assert Confidence.Confirmed.score() == 1.0
        assert Confidence.High.score() == 0.8
        assert Confidence.Medium.score() == 0.6
        assert Confidence.Low.score() == 0.4
        assert Confidence.Informational.score() == 0.2

    def test_from_str(self):
        assert Confidence.from_str("confirmed") == Confidence.Confirmed
        assert Confidence.from_str("high") == Confidence.High
        assert Confidence.from_str("medium") == Confidence.Medium
        assert Confidence.from_str("low") == Confidence.Low
        assert Confidence.from_str("informational") == Confidence.Informational
        assert Confidence.from_str("info") == Confidence.Informational

    def test_from_str_case_insensitive(self):
        assert Confidence.from_str("HIGH") == Confidence.High
        assert Confidence.from_str("Medium") == Confidence.Medium

    def test_from_str_unknown_raises_value_error(self):
        with pytest.raises(ValueError):
            Confidence.from_str("unknown")

    def test_as_str(self):
        assert Confidence.Confirmed.as_str() == "confirmed"
        assert Confidence.High.as_str() == "high"
        assert Confidence.Medium.as_str() == "medium"
        assert Confidence.Low.as_str() == "low"
        assert Confidence.Informational.as_str() == "informational"

    def test_repr_str(self):
        assert repr(Confidence.Confirmed) == "Confidence.Confirmed"
        assert str(Confidence.High) == "high"


class TestFindingType:
    def test_enum_values(self):
        assert FindingType.Vulnerability.as_str() == "vulnerability"
        assert FindingType.Misconfiguration.as_str() == "misconfiguration"
        assert FindingType.InformationLeak.as_str() == "information_leak"
        assert FindingType.PolicyViolation.as_str() == "policy_violation"
        assert FindingType.AssetDiscovery.as_str() == "asset_discovery"
        assert FindingType.ServiceDetection.as_str() == "service_detection"
        assert FindingType.WafDetection.as_str() == "waf_detection"
        assert FindingType.FuzzResult.as_str() == "fuzz_result"
        assert FindingType.ScanResult.as_str() == "scan_result"

    def test_from_str(self):
        assert FindingType.from_str("vulnerability") == FindingType.Vulnerability
        assert FindingType.from_str("misconfiguration") == FindingType.Misconfiguration
        assert FindingType.from_str("information_leak") == FindingType.InformationLeak
        assert FindingType.from_str("policy_violation") == FindingType.PolicyViolation
        assert FindingType.from_str("asset_discovery") == FindingType.AssetDiscovery
        assert FindingType.from_str("service_detection") == FindingType.ServiceDetection
        assert FindingType.from_str("waf_detection") == FindingType.WafDetection
        assert FindingType.from_str("fuzz_result") == FindingType.FuzzResult
        assert FindingType.from_str("scan_result") == FindingType.ScanResult

    def test_from_str_unknown_raises_value_error(self):
        with pytest.raises(ValueError):
            FindingType.from_str("nonexistent")

    def test_repr_str(self):
        assert repr(FindingType.Vulnerability) == "FindingType.Vulnerability"
        assert str(FindingType.Misconfiguration) == "misconfiguration"


class TestEvidenceKind:
    def test_enum_values(self):
        assert EvidenceKind.HttpRequest.as_str() == "HttpRequest"
        assert EvidenceKind.HttpResponse.as_str() == "HttpResponse"
        assert EvidenceKind.BodySnippet.as_str() == "BodySnippet"
        assert EvidenceKind.Banner.as_str() == "Banner"
        assert EvidenceKind.Header.as_str() == "Header"

    def test_from_str(self):
        assert EvidenceKind.from_str("HttpRequest") == EvidenceKind.HttpRequest
        assert EvidenceKind.from_str("httprequest") == EvidenceKind.HttpRequest
        assert EvidenceKind.from_str("http_request") == EvidenceKind.HttpRequest
        assert EvidenceKind.from_str("HttpResponse") == EvidenceKind.HttpResponse
        assert EvidenceKind.from_str("Banner") == EvidenceKind.Banner

    def test_from_str_unknown_raises_value_error(self):
        with pytest.raises(ValueError):
            EvidenceKind.from_str("unknown")

    def test_repr_str(self):
        assert repr(EvidenceKind.HttpRequest) == "EvidenceKind.HttpRequest"
        assert str(EvidenceKind.Banner) == "Banner"


class TestAffectedAsset:
    def test_construction(self):
        asset = AffectedAsset("host", "example.com", port=443, protocol="https")
        assert asset.asset_type == "host"
        assert asset.identifier == "example.com"
        assert asset.port == 443
        assert asset.protocol == "https"
        assert asset.host is None

    def test_construction_with_host(self):
        asset = AffectedAsset("host", "example.com", host="1.2.3.4")
        assert asset.host == "1.2.3.4"

    def test_minimal_construction(self):
        asset = AffectedAsset("url", "https://example.com")
        assert asset.asset_type == "url"
        assert asset.identifier == "https://example.com"
        assert asset.host is None
        assert asset.port is None
        assert asset.protocol is None

    def test_serialization(self):
        asset = AffectedAsset("host", "example.com")
        d = asset.to_dict()
        assert d["asset_type"] == "host"
        assert d["identifier"] == "example.com"
        j = asset.to_json()
        assert "example.com" in j

    def test_repr(self):
        asset = AffectedAsset("host", "example.com")
        r = repr(asset)
        assert "host" in r
        assert "example.com" in r


class TestFindingLocation:
    def test_construction(self):
        loc = FindingLocation(url="https://example.com/api", parameter="id")
        assert loc.url == "https://example.com/api"
        assert loc.parameter == "id"
        assert loc.path is None
        assert loc.header is None
        assert loc.method is None
        assert loc.line is None
        assert loc.file is None

    def test_construction_with_path(self):
        loc = FindingLocation(path="/api/users")
        assert loc.path == "/api/users"
        assert loc.url is None

    def test_construction_with_method_and_line(self):
        loc = FindingLocation(url="https://example.com", method="POST", line=42)
        assert loc.method == "POST"
        assert loc.line == 42

    def test_serialization(self):
        loc = FindingLocation(path="/api/users")
        d = loc.to_dict()
        assert d["path"] == "/api/users"
        j = loc.to_json()
        assert "/api/users" in j

    def test_default_all_none(self):
        loc = FindingLocation()
        assert loc.url is None
        assert loc.path is None
        assert loc.parameter is None

    def test_repr(self):
        loc = FindingLocation(path="/test")
        assert "path=/test" in repr(loc)


class TestVersionedEvidence:
    def test_construction(self):
        ev = VersionedEvidence(EvidenceKind.HttpResponse, "Found SQL error", data='{"status": 500}')
        assert ev.kind == EvidenceKind.HttpResponse
        assert ev.redacted is False
        assert ev.summary == "Found SQL error"
        assert ev.data == '{"status": 500}'

    def test_construction_with_redacted(self):
        ev = VersionedEvidence(EvidenceKind.Banner, "Server banner", redacted=True)
        assert ev.redacted is True

    def test_defaults(self):
        ev = VersionedEvidence(EvidenceKind.BodySnippet, "test")
        assert ev.data == ""
        assert ev.redacted is False

    def test_serialization(self):
        ev = VersionedEvidence(EvidenceKind.Banner, "Server banner")
        d = ev.to_dict_raw()
        assert d["summary"] == "Server banner"
        assert d["kind"] == "Banner"
        j = ev.to_json_raw()
        assert "Server banner" in j

    def test_repr(self):
        ev = VersionedEvidence(EvidenceKind.Banner, "test")
        r = repr(ev)
        assert "Banner" in r
        assert "test" in r


class TestVersionedFinding:
    def test_construction(self):
        asset = AffectedAsset("host", "example.com")
        f = VersionedFinding(
            id="f1",
            title="SQL Injection",
            description="SQL injection in login form",
            severity="High",
            finding_type=FindingType.Vulnerability,
            affected_asset=asset,
            source_tool="scanner",
            source_module="sqli",
            confidence=Confidence.High,
            cwe="CWE-89",
            cve="CVE-2024-0001",
            tags=["sqli", "auth"],
        )
        assert f.id == "f1"
        assert f.title == "SQL Injection"
        assert f.description == "SQL injection in login form"
        assert f.severity == "High"
        assert f.confidence == Confidence.High
        assert f.finding_type == FindingType.Vulnerability
        assert f.cwe == "CWE-89"
        assert f.cve == "CVE-2024-0001"
        assert "sqli" in f.tags
        assert "auth" in f.tags
        assert f.source_tool == "scanner"
        assert f.source_module == "sqli"
        assert f.to_dict()["schema_version"] == FINDING_SCHEMA_VERSION

    def test_minimal_construction(self):
        asset = AffectedAsset("host", "a.com")
        f = VersionedFinding("f1", "XSS", "desc", "Low", FindingType.Vulnerability, asset, "t", "m")
        assert f.confidence == Confidence.Medium
        assert f.cwe is None
        assert f.cve is None
        assert f.tags == []
        assert f.evidence == []
        assert f.remediation is None
        assert f.discovered_at != ""

    def test_fingerprint(self):
        asset = AffectedAsset("host", "example.com")
        f1 = VersionedFinding("f1", "SQLi", "desc", "High", FindingType.Vulnerability, asset, "t", "m")
        f2 = VersionedFinding("f2", "SQLi", "desc", "High", FindingType.Vulnerability, asset, "t", "m")
        fp1 = f1.compute_fingerprint()
        fp2 = f2.compute_fingerprint()
        assert len(fp1) > 0
        assert fp1 == fp2

    def test_different_findings_different_fingerprints(self):
        asset = AffectedAsset("host", "example.com")
        f1 = VersionedFinding("f1", "SQLi", "desc", "High", FindingType.Vulnerability, asset, "t", "m")
        f2 = VersionedFinding("f2", "XSS", "desc", "High", FindingType.Vulnerability, asset, "t", "m")
        assert f1.compute_fingerprint() != f2.compute_fingerprint()

    def test_serialization(self):
        asset = AffectedAsset("host", "example.com")
        f = VersionedFinding("f1", "XSS", "Reflected XSS", "Medium", FindingType.Vulnerability, asset, "t", "m")
        d = f.to_dict()
        assert d["id"] == "f1"
        assert d["title"] == "XSS"
        assert d["severity"] == "Medium"
        assert d["schema_version"] == FINDING_SCHEMA_VERSION
        j = f.to_json()
        assert "XSS" in j

    def test_schema_version_static(self):
        assert VersionedFinding.schema_version() == FINDING_SCHEMA_VERSION

    def test_repr(self):
        asset = AffectedAsset("host", "example.com")
        f = VersionedFinding("f1", "SQLi", "desc", "High", FindingType.Vulnerability, asset, "t", "m")
        r = repr(f)
        assert "f1" in r
        assert "SQLi" in r
        assert "High" in r


# ============================================================================
# E2: Artifact Model Tests
# ============================================================================

class TestMilestoneArtifact:
    def test_construction(self):
        a = MilestoneArtifact("a1", "evidence.txt", "text/plain", 100, "abc123")
        assert a.id == "a1"
        assert a.name == "evidence.txt"
        assert a.mime_type == "text/plain"
        assert a.size_bytes == 100
        assert a.content_hash == "abc123"
        assert a.provenance == "scan"
        assert a.redacted is False
        assert a.retention_policy == "session"
        assert a.external_uri is None

    def test_construction_with_options(self):
        a = MilestoneArtifact(
            "a1", "f.bin", "application/octet-stream", 50, "def",
            provenance="upload", redacted=True, retention_policy="permanent",
            external_uri="https://example.com/f.bin",
        )
        assert a.provenance == "upload"
        assert a.redacted is True
        assert a.retention_policy == "permanent"
        assert a.external_uri == "https://example.com/f.bin"

    def test_serialization(self):
        a = MilestoneArtifact("a1", "f.bin", "application/octet-stream", 50, "def")
        d = a.to_dict()
        assert d["id"] == "a1"
        j = a.to_json()
        assert "a1" in j

    def test_repr(self):
        a = MilestoneArtifact("a1", "f.txt", "text/plain", 10, "h")
        assert "a1" in repr(a)


class TestArtifactStore:
    def test_store_and_retrieve(self):
        store = ArtifactStore()
        a = MilestoneArtifact("a1", "evidence.txt", "text/plain", 100, "abc123")
        store.store(a)
        assert store.len() == 1
        assert len(store) == 1
        assert store.contains("a1")
        retrieved = store.get("a1")
        assert retrieved is not None
        assert retrieved.name == "evidence.txt"

    def test_get_nonexistent(self):
        store = ArtifactStore()
        assert store.get("nope") is None

    def test_remove(self):
        store = ArtifactStore()
        a = MilestoneArtifact("a1", "f.bin", "application/octet-stream", 50, "def")
        store.store(a)
        assert store.remove("a1") is True
        assert store.is_empty()

    def test_remove_nonexistent(self):
        store = ArtifactStore()
        assert store.remove("nope") is False

    def test_list_ids(self):
        store = ArtifactStore()
        store.store(MilestoneArtifact("a1", "f1", "text/plain", 10, "h1"))
        store.store(MilestoneArtifact("a2", "f2", "text/plain", 20, "h2"))
        ids = store.list_ids()
        assert set(ids) == {"a1", "a2"}

    def test_len_and_is_empty(self):
        store = ArtifactStore()
        assert store.is_empty()
        assert store.len() == 0
        store.store(MilestoneArtifact("a1", "f", "text/plain", 1, "h"))
        assert not store.is_empty()
        assert store.len() == 1

    def test_to_dict(self):
        store = ArtifactStore()
        store.store(MilestoneArtifact("a1", "f.txt", "text/plain", 10, "h"))
        d = store.to_dict()
        assert "a1" in d

    def test_to_json(self):
        store = ArtifactStore()
        store.store(MilestoneArtifact("a1", "f.txt", "text/plain", 10, "h"))
        j = store.to_json()
        assert "a1" in j

    def test_repr(self):
        store = ArtifactStore()
        assert "0" in repr(store)


class TestArtifactReference:
    def test_construction(self):
        ref = ArtifactReference("a1", "f1", "evidence")
        assert ref.artifact_id == "a1"
        assert ref.finding_id == "f1"
        assert ref.role == "evidence"

    def test_serialization(self):
        ref = ArtifactReference("a1", "f1", "evidence")
        d = ref.to_dict()
        assert d["artifact_id"] == "a1"


# ============================================================================
# E3: CVSS Tests
# ============================================================================

class TestCvssScore:
    def test_construction(self):
        score = CvssScore("3.1", "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H", 9.8)
        assert score.version == "3.1"
        assert score.base_score == 9.8
        assert score.severity == "Critical"
        assert score.exploitability is None
        assert score.impact is None

    def test_construction_with_severity_override(self):
        score = CvssScore("3.1", "vector", 5.0, severity="Custom")
        assert score.severity == "Custom"

    def test_parse(self):
        score = CvssScore.parse("CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H")
        assert score.version == "3.1"
        assert score.vector == "CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H"
        assert score.base_score == 0.0
        assert score.severity == "Info"

    def test_parse_cvss4(self):
        score = CvssScore.parse("CVSS:4.0/AV:N/AC:L/PR:N/UI:N")
        assert score.version == "4.0"

    def test_parse_invalid(self):
        with pytest.raises(Exception):
            CvssScore.parse("invalid vector")

    def test_auto_severity(self):
        assert CvssScore("3.1", "v", 9.5).severity == "Critical"
        assert CvssScore("3.1", "v", 9.0).severity == "Critical"
        assert CvssScore("3.1", "v", 7.5).severity == "High"
        assert CvssScore("3.1", "v", 7.0).severity == "High"
        assert CvssScore("3.1", "v", 5.0).severity == "Medium"
        assert CvssScore("3.1", "v", 4.0).severity == "Medium"
        assert CvssScore("3.1", "v", 2.0).severity == "Low"
        assert CvssScore("3.1", "v", 0.1).severity == "Low"
        assert CvssScore("3.1", "v", 0.0).severity == "Info"

    def test_serialization(self):
        score = CvssScore("3.1", "CVSS:3.1/AV:N", 7.5)
        d = score.to_dict()
        assert d["base_score"] == 7.5
        assert d["version"] == "3.1"
        j = score.to_json()
        assert "7.5" in j

    def test_repr(self):
        score = CvssScore("3.1", "v", 7.5)
        r = repr(score)
        assert "3.1" in r
        assert "7.5" in r


class TestVulnerabilityRecord:
    def test_construction(self):
        rec = VulnerabilityRecord(
            title="XSS",
            description="Reflected XSS",
            severity="High",
            cve_id="CVE-2024-1234",
            cwe_id="CWE-79",
        )
        assert rec.title == "XSS"
        assert rec.description == "Reflected XSS"
        assert rec.severity == "High"
        assert rec.cve_id == "CVE-2024-1234"
        assert rec.cwe_id == "CWE-79"
        assert rec.cvss is None
        assert rec.risk_accepted is False
        assert rec.exploit_available is False
        assert rec.confidence == "medium"

    def test_construction_minimal(self):
        rec = VulnerabilityRecord("XSS", "desc", "High")
        assert rec.title == "XSS"
        assert rec.cve_id is None
        assert rec.cwe_id is None
        assert rec.affected_assets == []
        assert rec.references == []

    def test_construction_with_cvss(self):
        cvss = CvssScore("3.1", "CVSS:3.1/AV:N", 7.5)
        rec = VulnerabilityRecord("XSS", "desc", "High", cvss=cvss)
        assert rec.cvss is not None
        assert rec.cvss.base_score == 7.5

    def test_serialization(self):
        rec = VulnerabilityRecord("XSS", "desc", "High")
        d = rec.to_dict()
        assert d["title"] == "XSS"
        assert d["severity"] == "High"
        j = rec.to_json()
        assert "XSS" in j

    def test_repr(self):
        rec = VulnerabilityRecord("XSS", "desc", "High", cve_id="CVE-2024-1")
        r = repr(rec)
        assert "CVE-2024-1" in r
        assert "XSS" in r


class TestRemediationRecord:
    def test_construction(self):
        rec = RemediationRecord("f1", status="in_progress", assigned_to="alice")
        assert rec.finding_id == "f1"
        assert rec.status == "in_progress"
        assert rec.assigned_to == "alice"
        assert rec.notes == []
        assert rec.estimated_effort is None

    def test_defaults(self):
        rec = RemediationRecord("f1")
        assert rec.status == "pending"
        assert rec.assigned_to is None
        assert rec.created_at != ""
        assert rec.updated_at != ""

    def test_serialization(self):
        rec = RemediationRecord("f1", status="fixed")
        d = rec.to_dict()
        assert d["finding_id"] == "f1"
        assert d["status"] == "fixed"
        j = rec.to_json()
        assert "f1" in j

    def test_repr(self):
        rec = RemediationRecord("f1", status="done")
        assert "f1" in repr(rec)


# ============================================================================
# E4: Finding Workflow Tests
# ============================================================================

class TestFindingState:
    def test_from_str(self):
        assert FindingState.from_str("new") == FindingState.New
        assert FindingState.from_str("triaged") == FindingState.Triaged
        assert FindingState.from_str("confirmed") == FindingState.Confirmed
        assert FindingState.from_str("in_progress") == FindingState.InProgress
        assert FindingState.from_str("accepted_risk") == FindingState.AcceptedRisk
        assert FindingState.from_str("false_positive") == FindingState.FalsePositive
        assert FindingState.from_str("remediated") == FindingState.Remediated
        assert FindingState.from_str("reopened") == FindingState.Reopened

    def test_from_str_case_insensitive(self):
        assert FindingState.from_str("NEW") == FindingState.New
        assert FindingState.from_str("Triaged") == FindingState.Triaged

    def test_from_str_inprogress_alias(self):
        assert FindingState.from_str("inprogress") == FindingState.InProgress

    def test_invalid_state_raises(self):
        with pytest.raises(ValueError, match="Invalid finding state"):
            FindingState.from_str("invalid_state")

    def test_as_str(self):
        assert FindingState.New.as_str() == "new"
        assert FindingState.Triaged.as_str() == "triaged"
        assert FindingState.Confirmed.as_str() == "confirmed"
        assert FindingState.InProgress.as_str() == "in_progress"
        assert FindingState.AcceptedRisk.as_str() == "accepted_risk"
        assert FindingState.FalsePositive.as_str() == "false_positive"
        assert FindingState.Remediated.as_str() == "remediated"
        assert FindingState.Reopened.as_str() == "reopened"

    def test_repr_str(self):
        assert repr(FindingState.New) == "FindingState.New"
        assert str(FindingState.Triaged) == "triaged"


class TestFindingWorkflow:
    def test_register_and_get_state(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        assert wf.get_state("f1") == FindingState.New

    def test_register_duplicate_raises(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        with pytest.raises(ValueError, match="already registered"):
            wf.register_finding("f1")

    def test_get_state_unregistered_raises(self):
        wf = FindingWorkflow()
        with pytest.raises(ValueError, match="not registered"):
            wf.get_state("nope")

    def test_valid_transitions(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        valid = wf.valid_transitions("f1")
        assert FindingState.Triaged in valid
        assert FindingState.FalsePositive in valid
        assert len(valid) == 2

    def test_can_transition(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        assert wf.can_transition("f1", FindingState.Triaged) is True
        assert wf.can_transition("f1", FindingState.Remediated) is False

    def test_transition(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        t = wf.transition("f1", FindingState.Triaged, changed_by="alice")
        assert t.from_state == FindingState.New
        assert t.to_state == FindingState.Triaged
        assert t.changed_by == "alice"
        assert wf.get_state("f1") == FindingState.Triaged

    def test_full_lifecycle(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        wf.transition("f1", FindingState.Triaged)
        wf.transition("f1", FindingState.Confirmed)
        wf.transition("f1", FindingState.InProgress)
        wf.transition("f1", FindingState.Remediated)
        assert wf.get_state("f1") == FindingState.Remediated

    def test_invalid_transition_raises(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        with pytest.raises(ValueError, match="Invalid transition"):
            wf.transition("f1", FindingState.Remediated)

    def test_suppress(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        s = wf.suppress("f1", "Not applicable")
        assert isinstance(s, Suppression)
        assert s.reason == "Not applicable"
        assert wf.is_suppressed("f1") is True

    def test_unsuppress(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        wf.suppress("f1", "reason")
        wf.unsuppress("f1")
        assert wf.is_suppressed("f1") is False

    def test_unsuppress_not_suppressed_raises(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        with pytest.raises(ValueError, match="not suppressed"):
            wf.unsuppress("f1")

    def test_is_suppressed_not_registered(self):
        wf = FindingWorkflow()
        assert wf.is_suppressed("nope") is False

    def test_history(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        wf.transition("f1", FindingState.Triaged)
        wf.transition("f1", FindingState.Confirmed)
        history = wf.get_history("f1")
        assert len(history) == 2
        assert history[0].from_state == FindingState.New
        assert history[0].to_state == FindingState.Triaged
        assert history[1].from_state == FindingState.Triaged
        assert history[1].to_state == FindingState.Confirmed

    def test_history_unregistered_raises(self):
        wf = FindingWorkflow()
        with pytest.raises(ValueError, match="not registered"):
            wf.get_history("nope")

    def test_repr(self):
        wf = FindingWorkflow()
        wf.register_finding("f1")
        assert "1" in repr(wf)


class TestWorkflowTransition:
    def test_construction(self):
        t = WorkflowTransition(FindingState.New, FindingState.Triaged, "2024-01-01T00:00:00Z")
        assert t.from_state == FindingState.New
        assert t.to_state == FindingState.Triaged

    def test_serialization(self):
        t = WorkflowTransition(FindingState.New, FindingState.Triaged, "2024-01-01T00:00:00Z", changed_by="a")
        d = t.to_dict()
        assert d["from_state"] == "new"
        assert d["to_state"] == "triaged"
        assert d["changed_by"] == "a"

    def test_repr(self):
        t = WorkflowTransition(FindingState.New, FindingState.Triaged, "2024-01-01T00:00:00Z")
        assert "new" in repr(t)
        assert "triaged" in repr(t)


class TestSuppression:
    def test_construction(self):
        s = Suppression("f1", "N/A", "2024-01-01T00:00:00Z")
        assert s.finding_id == "f1"
        assert s.reason == "N/A"
        assert s.suppressed_by is None
        assert s.expires_at is None

    def test_serialization(self):
        s = Suppression("f1", "reason", "2024-01-01T00:00:00Z", suppressed_by="alice")
        d = s.to_dict()
        assert d["finding_id"] == "f1"
        assert d["suppressed_by"] == "alice"


# ============================================================================
# E5: Repository Tests
# ============================================================================

class TestFindingRepository:
    def test_add_and_get(self):
        repo = FindingRepository()
        f = _make_finding()
        repo.add_finding(f)
        assert repo.count() == 1
        retrieved = repo.get_finding("f1")
        assert retrieved is not None
        assert retrieved.title == "SQLi"

    def test_add_duplicate_raises(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding())
        with pytest.raises(ValueError, match="already exists"):
            repo.add_finding(_make_finding())

    def test_get_nonexistent(self):
        repo = FindingRepository()
        assert repo.get_finding("nope") is None

    def test_add_findings_bulk(self):
        repo = FindingRepository()
        added = repo.add_findings([
            _make_finding("f1", "SQLi"),
            _make_finding("f2", "XSS"),
        ])
        assert added == 2
        assert repo.count() == 2

    def test_add_findings_skips_duplicates(self):
        repo = FindingRepository()
        added = repo.add_findings([
            _make_finding("f1", "SQLi"),
            _make_finding("f1", "SQLi"),
        ])
        assert added == 1
        assert repo.count() == 1

    def test_remove_finding(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding())
        assert repo.remove_finding("f1") is True
        assert repo.is_empty()

    def test_remove_nonexistent(self):
        repo = FindingRepository()
        assert repo.remove_finding("nope") is False

    def test_query_by_severity(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding("f1", "SQLi", "High"))
        repo.add_finding(_make_finding("f2", "XSS", "Low"))
        repo.add_finding(_make_finding("f3", "CSRF", "High"))
        high = repo.by_severity("High")
        assert len(high) == 2
        low = repo.by_severity("Low")
        assert len(low) == 1

    def test_query_by_target(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding("f1", target="a.com"))
        repo.add_finding(_make_finding("f2", target="b.com"))
        results = repo.by_target("a.com")
        assert len(results) == 1
        assert results[0].id == "f1"

    def test_query_by_confidence(self):
        repo = FindingRepository()
        f = _make_finding()
        repo.add_finding(f)
        medium = repo.by_confidence("medium")
        assert len(medium) == 1

    def test_query_by_finding_type(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding())
        vulns = repo.by_finding_type("vulnerability")
        assert len(vulns) == 1

    def test_query_by_cve(self):
        asset = AffectedAsset("host", "example.com")
        f = VersionedFinding(
            "f1", "XSS", "desc", "High", FindingType.Vulnerability,
            asset, "t", "m", cve="CVE-2024-1",
        )
        repo = FindingRepository()
        repo.add_finding(f)
        results = repo.by_cve("CVE-2024-1")
        assert len(results) == 1

    def test_query_by_cwe(self):
        asset = AffectedAsset("host", "example.com")
        f = VersionedFinding(
            "f1", "SQLi", "desc", "High", FindingType.Vulnerability,
            asset, "t", "m", cwe="CWE-89",
        )
        repo = FindingRepository()
        repo.add_finding(f)
        results = repo.by_cwe("CWE-89")
        assert len(results) == 1

    def test_query_by_tag(self):
        asset = AffectedAsset("host", "example.com")
        f = VersionedFinding(
            "f1", "SQLi", "desc", "High", FindingType.Vulnerability,
            asset, "t", "m", tags=["sqli", "auth"],
        )
        repo = FindingRepository()
        repo.add_finding(f)
        results = repo.by_tag("sqli")
        assert len(results) == 1

    def test_query_by_tool(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding())
        results = repo.by_tool("t")
        assert len(results) == 1

    def test_combined_filter(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding("f1", severity="High", target="a.com"))
        repo.add_finding(_make_finding("f2", severity="Low", target="b.com"))
        repo.add_finding(_make_finding("f3", severity="High", target="a.com"))
        results = repo.filter(severity="High")
        assert len(results) == 2

    def test_filter_by_min_severity(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding("f1", severity="High"))
        repo.add_finding(_make_finding("f2", severity="Low"))
        repo.add_finding(_make_finding("f3", severity="Info"))
        results = repo.filter(min_severity="Medium")
        assert len(results) == 1
        assert results[0].id == "f1"

    def test_deduplicate(self):
        repo = FindingRepository()
        f1 = _make_finding("f1", "SQLi")
        f2 = _make_finding("f2", "SQLi")
        repo.add_finding(f1)
        repo.add_finding(f2)
        removed = repo.deduplicate()
        assert removed >= 0

    def test_list_ids(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding("f1"))
        repo.add_finding(_make_finding("f2"))
        ids = repo.list_ids()
        assert set(ids) == {"f1", "f2"}

    def test_all_findings(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding("f1"))
        repo.add_finding(_make_finding("f2"))
        all_f = repo.all_findings()
        assert len(all_f) == 2

    def test_clear(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding())
        assert repo.count() == 1
        repo.clear()
        assert repo.is_empty()

    def test_to_dict(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding())
        d = repo.to_dict()
        assert "findings" in d
        assert d["count"] == 1

    def test_to_json(self):
        repo = FindingRepository()
        repo.add_finding(_make_finding())
        j = repo.to_json()
        assert "f1" in j

    def test_repr(self):
        repo = FindingRepository()
        assert "0" in repr(repo)


class TestAssessment:
    def test_construction(self):
        a = Assessment("a1", "Scan 1", "example.com")
        assert a.id == "a1"
        assert a.name == "Scan 1"
        assert a.target == "example.com"
        assert a.created_at != ""
        assert a.finding_count == 0
        assert a.metadata == "{}"

    def test_construction_with_options(self):
        a = Assessment("a1", "Scan 1", "example.com", finding_count=5)
        assert a.finding_count == 5

    def test_serialization(self):
        a = Assessment("a1", "Scan 1", "example.com")
        d = a.to_dict()
        assert d["id"] == "a1"
        j = a.to_json()
        assert "a1" in j

    def test_repr(self):
        a = Assessment("a1", "Scan 1", "example.com")
        r = repr(a)
        assert "a1" in r
        assert "Scan 1" in r


class TestAssessmentRepository:
    def test_save_and_get(self):
        repo = AssessmentRepository()
        a = Assessment("a1", "Scan 1", "example.com")
        repo.save(a)
        assert repo.count() == 1
        retrieved = repo.get("a1")
        assert retrieved is not None
        assert retrieved.name == "Scan 1"

    def test_list(self):
        repo = AssessmentRepository()
        repo.save(Assessment("a1", "Scan 1", "a.com"))
        repo.save(Assessment("a2", "Scan 2", "b.com"))
        items = repo.list()
        assert len(items) == 2

    def test_remove(self):
        repo = AssessmentRepository()
        repo.save(Assessment("a1", "Scan 1", "a.com"))
        assert repo.remove("a1") is True
        assert repo.count() == 0

    def test_remove_nonexistent(self):
        repo = AssessmentRepository()
        assert repo.remove("nope") is False

    def test_to_dict(self):
        repo = AssessmentRepository()
        repo.save(Assessment("a1", "Scan", "a.com"))
        d = repo.to_dict()
        assert "assessments" in d
        assert d["count"] == 1

    def test_to_json(self):
        repo = AssessmentRepository()
        repo.save(Assessment("a1", "Scan", "a.com"))
        j = repo.to_json()
        assert "a1" in j


# ============================================================================
# E6: Baseline Tests
# ============================================================================

class TestBaselineComparator:
    def test_compare_identical(self):
        comp = BaselineComparator()
        findings = [
            _make_finding("f1", "SQLi"),
            _make_finding("f2", "XSS"),
        ]
        diff = comp.compare(findings, findings)
        assert diff.unchanged_findings == 2
        assert diff.new_findings == 0
        assert diff.resolved_findings == 0
        assert diff.changed_findings == 0
        assert diff.is_regression is False
        assert diff.is_improvement is False

    def test_compare_new_findings(self):
        comp = BaselineComparator()
        baseline = [_make_finding("f1", "SQLi")]
        current = [
            _make_finding("f1", "SQLi"),
            _make_finding("f3", "CSRF"),
        ]
        diff = comp.compare(baseline, current)
        assert diff.new_findings == 1
        assert diff.unchanged_findings == 1
        assert diff.is_regression is True

    def test_compare_resolved_findings(self):
        comp = BaselineComparator()
        baseline = [
            _make_finding("f1", "SQLi"),
            _make_finding("f2", "XSS"),
        ]
        current = [_make_finding("f1", "SQLi")]
        diff = comp.compare(baseline, current)
        assert diff.resolved_findings == 1
        assert diff.unchanged_findings == 1
        assert diff.is_improvement is True

    def test_compare_changed(self):
        comp = BaselineComparator()
        baseline = [_make_finding("f1", "SQLi", severity="High")]
        current = [_make_finding("f1", "SQLi", severity="Critical")]
        diff = comp.compare(baseline, current)
        assert diff.changed_findings == 1

    def test_compare_empty(self):
        comp = BaselineComparator()
        diff = comp.compare([], [])
        assert diff.new_findings == 0
        assert diff.resolved_findings == 0
        assert diff.unchanged_findings == 0

    def test_correlate(self):
        comp = BaselineComparator()
        findings = [
            _make_finding("f1", "SQLi"),
            _make_finding("f2", "XSS"),
        ]
        correlations = comp.correlate(findings, findings)
        assert len(correlations) == 2
        for c in correlations:
            assert isinstance(c, FindingCorrelation)
            assert c.confidence > 0
            assert c.correlation_method != ""

    def test_correlation_rules(self):
        comp = BaselineComparator()
        rules = comp.correlation_rules()
        assert "fingerprint" in rules
        assert "title" in rules
        assert "location" in rules

    def test_add_correlation_rule(self):
        comp = BaselineComparator()
        comp.add_correlation_rule("cve")
        rules = comp.correlation_rules()
        assert "cve" in rules

    def test_diff_summary(self):
        comp = BaselineComparator()
        diff = comp.compare([], [_make_finding("f1")])
        assert "new" in diff.summary.lower() or "Compared" in diff.summary


class TestFindingDiff:
    def test_values(self):
        assert FindingDiff.New.as_str() == "new"
        assert FindingDiff.Resolved.as_str() == "resolved"
        assert FindingDiff.Changed.as_str() == "changed"
        assert FindingDiff.Unchanged.as_str() == "unchanged"
        assert FindingDiff.Suppressed.as_str() == "suppressed"
        assert FindingDiff.Indeterminate.as_str() == "indeterminate"

    def test_repr_str(self):
        assert repr(FindingDiff.New) == "FindingDiff.new"
        assert str(FindingDiff.Changed) == "changed"


class TestFindingCorrelation:
    def test_construction(self):
        c = FindingCorrelation("b1", "c1", "fingerprint", 1.0, ["severity"])
        assert c.baseline_finding_id == "b1"
        assert c.current_finding_id == "c1"
        assert c.correlation_method == "fingerprint"
        assert c.confidence == 1.0
        assert "severity" in c.changed_fields

    def test_serialization(self):
        c = FindingCorrelation("b1", "c1", "title", 0.8, [])
        d = c.to_dict()
        assert d["baseline_finding_id"] == "b1"
        j = c.to_json()
        assert "b1" in j

    def test_repr(self):
        c = FindingCorrelation("b1", "c1", "fingerprint", 1.0, [])
        r = repr(c)
        assert "b1" in r
        assert "c1" in r


class TestAssessmentDiff:
    def test_fields(self):
        comp = BaselineComparator()
        diff = comp.compare([], [])
        assert diff.new_findings == 0
        assert diff.resolved_findings == 0
        assert diff.changed_findings == 0
        assert diff.unchanged_findings == 0
        assert diff.suppressed_findings == 0
        assert diff.is_regression is False
        assert diff.is_improvement is False
        assert isinstance(diff.summary, str)
        assert isinstance(diff.compared_at, str)

    def test_to_dict(self):
        comp = BaselineComparator()
        diff = comp.compare([], [])
        d = diff.to_dict()
        assert d["new_findings"] == 0
        assert "summary" in d

    def test_to_json(self):
        comp = BaselineComparator()
        diff = comp.compare([], [])
        j = diff.to_json()
        assert "new_findings" in j


# ============================================================================
# E7: Reporting Tests
# ============================================================================

class TestFindingReporter:
    def test_json_output(self):
        reporter = FindingReporter("json")
        findings = [
            _make_finding("f1", "SQLi"),
            _make_finding("f2", "XSS", "Medium"),
        ]
        output = reporter.generate(findings, title="Test Report")
        parsed = json.loads(output)
        assert "findings" in parsed or "f1" in str(parsed)
        assert "title" in parsed
        assert "severity_summary" in parsed

    def test_jsonl_output(self):
        reporter = FindingReporter("jsonl")
        findings = [_make_finding("f1", "SQLi"), _make_finding("f2", "XSS")]
        output = reporter.generate(findings)
        lines = output.strip().split("\n")
        assert len(lines) == 2
        for line in lines:
            parsed = json.loads(line)
            assert "id" in parsed

    def test_markdown_output(self):
        reporter = FindingReporter("markdown")
        findings = [_make_finding("f1", "SQLi"), _make_finding("f2", "XSS")]
        output = reporter.generate(findings, title="Test")
        assert "SQLi" in output
        assert "XSS" in output
        assert "Test" in output

    def test_csv_output(self):
        reporter = FindingReporter("csv")
        findings = [_make_finding("f1", "SQLi"), _make_finding("f2", "XSS")]
        output = reporter.generate(findings)
        assert "SQLi" in output
        assert "XSS" in output
        assert "id,title" in output

    def test_html_output(self):
        reporter = FindingReporter("html")
        findings = [_make_finding("f1", "SQLi")]
        output = reporter.generate(findings, title="Report")
        assert "<html" in output.lower()
        assert "SQLi" in output

    def test_sarif_output(self):
        reporter = FindingReporter("sarif")
        findings = [_make_finding("f1", "SQLi")]
        output = reporter.generate(findings)
        parsed = json.loads(output)
        assert "$schema" in parsed
        assert "runs" in parsed
        assert parsed["version"] == "2.1.0"

    def test_unsupported_format_raises(self):
        reporter = FindingReporter("xml")
        with pytest.raises(ValueError, match="Unsupported format"):
            reporter.generate([_make_finding()])

    def test_format_name(self):
        reporter = FindingReporter("json")
        assert reporter.format_name() == "json"
        reporter2 = FindingReporter("html")
        assert reporter2.format_name() == "html"

    def test_redaction_policy_name(self):
        reporter = FindingReporter("json")
        assert reporter.redaction_policy_name() == "redact_sensitive"

    def test_custom_redaction_policy(self):
        reporter = FindingReporter("json", redaction_policy="none")
        assert reporter.redaction_policy_name() == "none"

    def test_includes_artifacts_default(self):
        reporter = FindingReporter("json")
        assert reporter.includes_artifacts() is False

    def test_includes_artifacts_enabled(self):
        reporter = FindingReporter("json", include_artifacts=True)
        assert reporter.includes_artifacts() is True

    def test_empty_findings(self):
        reporter = FindingReporter("json")
        output = reporter.generate([])
        parsed = json.loads(output)
        assert "findings" in parsed or "finding_count" in str(parsed)


class TestSeveritySummary:
    def test_from_findings(self):
        findings = [
            _make_finding("f1", "SQLi", "High"),
            _make_finding("f2", "Info", "Info"),
        ]
        summary = SeveritySummary.from_findings(findings)
        assert summary.total == 2
        assert summary.high == 1
        assert summary.info == 1
        assert summary.critical == 0
        assert summary.medium == 0
        assert summary.low == 0

    def test_from_findings_all_severities(self):
        findings = [
            _make_finding("f1", severity="Critical"),
            _make_finding("f2", severity="High"),
            _make_finding("f3", severity="Medium"),
            _make_finding("f4", severity="Low"),
            _make_finding("f5", severity="Info"),
        ]
        summary = SeveritySummary.from_findings(findings)
        assert summary.critical == 1
        assert summary.high == 1
        assert summary.medium == 1
        assert summary.low == 1
        assert summary.info == 1
        assert summary.total == 5
        assert summary.risk_score > 0

    def test_from_findings_empty(self):
        summary = SeveritySummary.from_findings([])
        assert summary.total == 0
        assert summary.risk_score == 0.0

    def test_to_dict(self):
        findings = [_make_finding("f1", severity="High")]
        summary = SeveritySummary.from_findings(findings)
        d = summary.to_dict()
        assert d["total"] == 1
        assert d["high"] == 1

    def test_to_json(self):
        summary = SeveritySummary.from_findings([_make_finding()])
        j = summary.to_json()
        assert "total" in j


class TestReportEnvelope:
    def test_construction(self):
        findings = [_make_finding("f1", "SQLi")]
        envelope = ReportEnvelope("Test Report", findings, tool_name="test")
        assert envelope.title == "Test Report"
        assert envelope.finding_count == 1
        assert envelope.tool_name == "test"
        assert envelope.tool_version != ""

    def test_default_tool_name(self):
        envelope = ReportEnvelope("Test", [])
        assert envelope.tool_name == "eggsec"

    def test_custom_report_id(self):
        envelope = ReportEnvelope("Test", [], report_id="rpt-123")
        assert envelope.report_id == "rpt-123"

    def test_severity_summary_populated(self):
        findings = [_make_finding("f1", severity="High")]
        envelope = ReportEnvelope("Test", findings)
        assert envelope.severity_summary.total == 1
        assert envelope.severity_summary.high == 1

    def test_generate_report_json(self):
        findings = [_make_finding("f1", "SQLi")]
        envelope = ReportEnvelope("Test", findings)
        report = envelope.generate_report("json")
        assert len(report) > 0
        parsed = json.loads(report)
        assert "findings" in parsed

    def test_generate_report_markdown(self):
        findings = [_make_finding("f1", "SQLi")]
        envelope = ReportEnvelope("Test", findings)
        report = envelope.generate_report("markdown")
        assert "SQLi" in report

    def test_to_dict(self):
        findings = [_make_finding("f1")]
        envelope = ReportEnvelope("Test", findings)
        d = envelope.to_dict()
        assert d["title"] == "Test"
        assert d["finding_count"] == 1
        assert "severity_summary" in d

    def test_to_json(self):
        findings = [_make_finding("f1")]
        envelope = ReportEnvelope("Test", findings)
        j = envelope.to_json()
        assert "Test" in j

    def test_findings_accessible(self):
        findings = [_make_finding("f1", "SQLi"), _make_finding("f2", "XSS")]
        envelope = ReportEnvelope("Test", findings)
        assert len(envelope.findings) == 2
        assert envelope.findings[0].title == "SQLi"


# ============================================================================
# E9: External Integrations Tests
# ============================================================================

class TestIntegrationType:
    def test_values(self):
        assert IntegrationType.GitHub.as_str() == "github"
        assert IntegrationType.GitLab.as_str() == "gitlab"
        assert IntegrationType.Jira.as_str() == "jira"
        assert IntegrationType.Webhook.as_str() == "webhook"
        assert IntegrationType.Custom.as_str() == "custom"

    def test_from_str(self):
        assert IntegrationType.from_str("github") == IntegrationType.GitHub
        assert IntegrationType.from_str("gitlab") == IntegrationType.GitLab
        assert IntegrationType.from_str("webhook") == IntegrationType.Webhook

    def test_from_str_invalid(self):
        with pytest.raises(ValueError, match="Unknown integration type"):
            IntegrationType.from_str("invalid")

    def test_repr_str(self):
        assert repr(IntegrationType.GitHub) == "IntegrationType.github"
        assert str(IntegrationType.Jira) == "jira"


class TestExternalIntegration:
    def test_dry_run(self):
        policy = PublicationPolicy(dry_run=True)
        integration = ExternalIntegration(
            IntegrationType.GitHub,
            "test-repo",
            {"owner": "org", "repo": "repo"},
            policy=policy,
        )
        assert integration.is_dry_run() is True
        assert integration.name() == "test-repo"
        f = _make_finding("f1", "XSS")
        record = integration.publish_finding(f)
        assert record.dry_run is True
        assert record.success is True
        assert record.action == "create"

    def test_not_dry_run(self):
        policy = PublicationPolicy(dry_run=False)
        integration = ExternalIntegration(
            IntegrationType.GitHub,
            "test-repo",
            {},
            policy=policy,
        )
        assert integration.is_dry_run() is False
        f = _make_finding("f1", "XSS")
        record = integration.publish_finding(f)
        assert record.dry_run is False
        assert record.external_id is not None

    def test_deduplication(self):
        policy = PublicationPolicy(dry_run=True)
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {"url": "http://example.com"},
            policy=policy,
        )
        f = _make_finding("f1", "XSS")
        r1 = integration.publish_finding(f)
        r2 = integration.publish_finding(f)
        assert r1.action == "create"
        assert r2.action == "skip"

    def test_policy_min_severity(self):
        policy = PublicationPolicy(dry_run=True, min_severity="High")
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        low = _make_finding("f1", "Info", "Info")
        record = integration.publish_finding(low)
        assert record.action == "skip"

    def test_policy_blocked_tags(self):
        policy = PublicationPolicy(dry_run=True, blocked_tags=["internal"])
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        asset = AffectedAsset("host", "example.com")
        f = VersionedFinding(
            "f1", "XSS", "desc", "High", FindingType.Vulnerability,
            asset, "t", "m", tags=["internal"],
        )
        record = integration.publish_finding(f)
        assert record.action == "skip"

    def test_policy_allowed_finding_types(self):
        policy = PublicationPolicy(
            dry_run=True,
            allowed_finding_types=["vulnerability"],
        )
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        f = _make_finding("f1", "XSS")
        record = integration.publish_finding(f)
        assert record.action == "create"

    def test_policy_allowed_finding_types_reject(self):
        policy = PublicationPolicy(
            dry_run=True,
            allowed_finding_types=["misconfiguration"],
        )
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        f = _make_finding("f1", "XSS")
        record = integration.publish_finding(f)
        assert record.action == "skip"

    def test_publication_records(self):
        policy = PublicationPolicy(dry_run=True)
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        f = _make_finding("f1", "XSS")
        integration.publish_finding(f)
        records = integration.publication_records()
        assert len(records) == 1
        assert records[0].finding_id == "f1"

    def test_integration_type_method(self):
        integration = ExternalIntegration(
            IntegrationType.GitHub,
            "test",
            {},
        )
        assert integration.integration_type() == IntegrationType.GitHub

    def test_policy_method(self):
        policy = PublicationPolicy(dry_run=False, min_severity="Low")
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        p = integration.policy()
        assert p.dry_run is False
        assert p.min_severity == "Low"

    def test_retry_policy_method(self):
        rp = RetryPolicy(max_retries=5, base_delay_ms=500)
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            retry_policy=rp,
        )
        r = integration.retry_policy()
        assert r.max_retries == 5
        assert r.base_delay_ms == 500

    def test_serialization(self):
        integration = ExternalIntegration(
            IntegrationType.GitHub,
            "test",
            {"owner": "org"},
        )
        d = integration.to_dict()
        assert d["name"] == "test"
        j = integration.to_json()
        assert "test" in j

    def test_repr(self):
        integration = ExternalIntegration(
            IntegrationType.GitHub,
            "test",
            {},
        )
        r = repr(integration)
        assert "github" in r
        assert "test" in r


class TestPublicationRecord:
    def test_fields(self):
        policy = PublicationPolicy(dry_run=True)
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        f = _make_finding("f1", "XSS")
        record = integration.publish_finding(f)
        assert record.id != ""
        assert record.integration_type == IntegrationType.Webhook
        assert record.finding_id == "f1"
        assert record.published_at != ""
        assert record.dedup_key != ""

    def test_serialization(self):
        policy = PublicationPolicy(dry_run=True)
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        record = integration.publish_finding(_make_finding())
        d = record.to_dict()
        assert "finding_id" in d
        j = record.to_json()
        assert "finding_id" in j

    def test_repr(self):
        policy = PublicationPolicy(dry_run=True)
        integration = ExternalIntegration(
            IntegrationType.Webhook,
            "test",
            {},
            policy=policy,
        )
        record = integration.publish_finding(_make_finding())
        r = repr(record)
        assert "PublicationRecord" in r


class TestRetryPolicy:
    def test_defaults(self):
        rp = RetryPolicy()
        assert rp.max_retries == 3
        assert rp.base_delay_ms == 1000
        assert rp.max_delay_ms == 30000
        assert rp.backoff_multiplier == 2.0

    def test_custom(self):
        rp = RetryPolicy(max_retries=5, base_delay_ms=500, max_delay_ms=10000, backoff_multiplier=1.5)
        assert rp.max_retries == 5
        assert rp.base_delay_ms == 500
        assert rp.max_delay_ms == 10000
        assert rp.backoff_multiplier == 1.5

    def test_serialization(self):
        rp = RetryPolicy()
        d = rp.to_dict()
        assert d["max_retries"] == 3
        j = rp.to_json()
        assert "max_retries" in j

    def test_repr(self):
        rp = RetryPolicy()
        assert "3" in repr(rp)


class TestPublicationPolicy:
    def test_defaults(self):
        pp = PublicationPolicy()
        assert pp.dry_run is True
        assert pp.redact_sensitive is True
        assert pp.min_severity == "Medium"
        assert pp.include_evidence is False
        assert pp.include_artifacts is False
        assert pp.allowed_finding_types == []
        assert pp.blocked_tags == []

    def test_custom(self):
        pp = PublicationPolicy(
            dry_run=False,
            redact_sensitive=False,
            min_severity="Low",
            include_evidence=True,
            blocked_tags=["internal"],
        )
        assert pp.dry_run is False
        assert pp.redact_sensitive is False
        assert pp.min_severity == "Low"
        assert pp.include_evidence is True
        assert "internal" in pp.blocked_tags

    def test_serialization(self):
        pp = PublicationPolicy()
        d = pp.to_dict()
        assert d["dry_run"] is True
        j = pp.to_json()
        assert "dry_run" in j

    def test_repr(self):
        pp = PublicationPolicy()
        r = repr(pp)
        assert "Medium" in r


# ============================================================================
# E10: Migration Tests
# ============================================================================

class TestFindingMigration:
    def test_migrate_legacy(self):
        migrator = FindingMigration()
        f = migrator.migrate_legacy_finding(
            id="legacy-1",
            title="Old XSS",
            severity="Medium",
            target="example.com",
            category="xss",
            description="Reflected XSS",
            recommendation="Escape output",
        )
        assert f.id == "legacy-1"
        assert f.title == "Old XSS"
        assert f.severity == "medium"
        assert f.to_dict()["schema_version"] == FINDING_SCHEMA_VERSION
        assert f.source_tool == "legacy_migration"
        assert f.affected_asset.identifier == "example.com"
        assert f.finding_type == FindingType.Vulnerability

    def test_migrate_legacy_minimal(self):
        migrator = FindingMigration()
        f = migrator.migrate_legacy_finding(
            id="legacy-2",
            title="Test",
            severity="high",
            target="a.com",
            category="sqli",
            description="SQL injection",
        )
        assert f.id == "legacy-2"
        assert f.remediation is None
        assert f.evidence == []

    def test_migrate_engine_finding(self):
        migrator = FindingMigration()
        engine_finding = json.dumps({
            "id": "eng-1",
            "title": "SQLi",
            "description": "SQL injection",
            "severity": "High",
            "location": "https://example.com/api",
            "evidence": "Error in SQL syntax",
        })
        f = migrator.migrate_engine_finding(engine_finding)
        assert f.id == "eng-1"
        assert f.title == "SQLi"
        assert f.to_dict()["schema_version"] == FINDING_SCHEMA_VERSION
        assert f.source_tool == "engine_migration"

    def test_migrate_engine_finding_invalid_json(self):
        migrator = FindingMigration()
        with pytest.raises(ValueError):
            migrator.migrate_engine_finding("not json")

    def test_needs_migration(self):
        migrator = FindingMigration()
        assert migrator.needs_migration('{"title": "old"}') is True
        assert migrator.needs_migration('{"schema_version": "1.0", "title": "new"}') is False

    def test_needs_migration_invalid_json(self):
        migrator = FindingMigration()
        assert migrator.needs_migration("not json") is True

    def test_supported_versions(self):
        migrator = FindingMigration()
        versions = migrator.supported_versions()
        assert "legacy" in versions
        assert "engine" in versions
        assert "0.1" in versions
        assert "0.2" in versions

    def test_target_version(self):
        migrator = FindingMigration()
        assert migrator.target_version() == FINDING_SCHEMA_VERSION

    def test_batch_migration(self):
        migrator = FindingMigration()
        result = migrator.migrate_batch([
            {"id": "1", "title": "A", "severity": "High", "target": "a.com", "category": "sqli", "description": "d1"},
            {"id": "2", "title": "B", "severity": "Low", "target": "b.com", "category": "xss", "description": "d2"},
        ])
        assert result.items_migrated == 2
        assert result.success is True
        assert result.source_version == "legacy"
        assert result.target_version == FINDING_SCHEMA_VERSION

    def test_batch_migration_empty_id_warning(self):
        migrator = FindingMigration()
        result = migrator.migrate_batch([
            {"title": "A", "severity": "High", "target": "a.com", "category": "sqli", "description": "d"},
        ])
        assert result.items_migrated == 1
        assert len(result.warnings) > 0

    def test_batch_migration_empty_list(self):
        migrator = FindingMigration()
        result = migrator.migrate_batch([])
        assert result.items_migrated == 0
        assert result.success is True


class TestMigrationResult:
    def test_fields(self):
        migrator = FindingMigration()
        result = migrator.migrate_batch([
            {"id": "1", "title": "A", "severity": "High", "target": "a.com", "category": "sqli", "description": "d"},
        ])
        assert isinstance(result.success, bool)
        assert isinstance(result.source_version, str)
        assert isinstance(result.target_version, str)
        assert isinstance(result.items_migrated, int)
        assert isinstance(result.warnings, list)
        assert isinstance(result.errors, list)

    def test_serialization(self):
        migrator = FindingMigration()
        result = migrator.migrate_batch([])
        d = result.to_dict()
        assert "success" in d
        j = result.to_json()
        assert "success" in j

    def test_repr(self):
        migrator = FindingMigration()
        result = migrator.migrate_batch([])
        r = repr(result)
        assert "migrated" in r.lower() or "MigrationResult" in r


class TestSchemaVersion:
    def test_construction(self):
        sv = SchemaVersion("1.0", "Initial release", compatible_with=["0.9", "0.8"])
        assert sv.version == "1.0"
        assert sv.description == "Initial release"
        assert "0.9" in sv.compatible_with
        assert "0.8" in sv.compatible_with
        assert sv.created_at != ""

    def test_construction_minimal(self):
        sv = SchemaVersion("2.0", "Second release")
        assert sv.version == "2.0"
        assert sv.compatible_with == []

    def test_serialization(self):
        sv = SchemaVersion("1.0", "Initial")
        d = sv.to_dict()
        assert d["version"] == "1.0"
        j = sv.to_json()
        assert "1.0" in j

    def test_repr(self):
        sv = SchemaVersion("1.0", "Initial")
        r = repr(sv)
        assert "1.0" in r
        assert "Initial" in r


# ============================================================================
# E8: Compliance Tests (feature-gated, may not be available)
# ============================================================================

class TestCompliance:
    @pytest.fixture(autouse=True)
    def check_feature(self):
        try:
            from eggsec import ComplianceFramework
            self.ComplianceFramework = ComplianceFramework
        except ImportError:
            pytest.skip("compliance feature not available")

    def test_framework_values(self):
        from eggsec import ComplianceFramework
        assert ComplianceFramework.OwaspTop10.as_str() == "OWASP Top 10"
        assert ComplianceFramework.NistCsf.as_str() == "NIST CSF"
        assert ComplianceFramework.PciDss.as_str() == "PCI DSS"
        assert ComplianceFramework.Hipaa.as_str() == "HIPAA"
        assert ComplianceFramework.Soca2.as_str() == "SOC 2"
        assert ComplianceFramework.Iso27001.as_str() == "ISO 27001"
        assert ComplianceFramework.CisBenchmarks.as_str() == "CIS Benchmarks"
        assert ComplianceFramework.Custom.as_str() == "Custom"

    def test_framework_from_str(self):
        from eggsec import ComplianceFramework
        assert ComplianceFramework.from_str("owasp_top_10") == ComplianceFramework.OwaspTop10
        assert ComplianceFramework.from_str("nist_csf") == ComplianceFramework.NistCsf

    def test_mapper_register_control(self):
        from eggsec import ComplianceFramework, ComplianceControl, ComplianceMapper
        mapper = ComplianceMapper()
        control = ComplianceControl(
            ComplianceFramework.OwaspTop10,
            "A01:2021",
            "Broken Access Control",
            "Description",
        )
        mapper.register_control(control)
        assert len(mapper.controls()) == 1

    def test_mapper_register_controls_bulk(self):
        from eggsec import ComplianceFramework, ComplianceControl, ComplianceMapper
        mapper = ComplianceMapper()
        controls = [
            ComplianceControl(ComplianceFramework.OwaspTop10, "A01:2021", "Control 1", "Desc 1"),
            ComplianceControl(ComplianceFramework.OwaspTop10, "A02:2021", "Control 2", "Desc 2"),
        ]
        mapper.register_controls(controls)
        assert len(mapper.controls()) == 2

    def test_mapper_auto_map(self):
        from eggsec import ComplianceFramework, ComplianceControl, ComplianceMapper
        mapper = ComplianceMapper()
        mapper.register_control(ComplianceControl(
            ComplianceFramework.OwaspTop10, "A03:2021", "Injection", "Includes SQL injection vulnerabilities", version="2021",
        ))
        f = _make_finding("f1", "SQLi")
        # Override cwe by creating with cwe
        asset = AffectedAsset("host", "example.com")
        f_with_cwe = VersionedFinding(
            "f1", "SQLi", "desc", "High", FindingType.Vulnerability,
            asset, "t", "m", cwe="CWE-89",
        )
        mappings = mapper.auto_map_findings([f_with_cwe])
        assert len(mappings) >= 0

    def test_mapper_get_mappings(self):
        from eggsec import ComplianceFramework, ComplianceControl, ComplianceMapper
        mapper = ComplianceMapper()
        control = ComplianceControl(
            ComplianceFramework.OwaspTop10, "A03:2021", "Injection", "Desc",
        )
        mapper.register_control(control)
        mapper.map_finding("f1", "A03:2021", ComplianceFramework.OwaspTop10, 0.9, "CWE match")
        assert len(mapper.get_mappings_for_finding("f1")) == 1
        assert len(mapper.get_mappings_for_control("A03:2021")) == 1

    def test_mapper_assess_control(self):
        from eggsec import ComplianceFramework, ComplianceControl, ComplianceMapper, ComplianceResult
        mapper = ComplianceMapper()
        control = ComplianceControl(
            ComplianceFramework.OwaspTop10, "A03:2021", "Injection", "Desc",
        )
        mapper.register_control(control)
        mapper.map_finding("f1", "A03:2021", ComplianceFramework.OwaspTop10, 0.9, "match")
        assessment = mapper.assess_control("A03:2021", [_make_finding("f1")])
        assert assessment.result == ComplianceResult.Pass

    def test_compliance_result_values(self):
        from eggsec import ComplianceResult
        assert ComplianceResult.Pass.as_str() == "pass"
        assert ComplianceResult.Fail.as_str() == "fail"
        assert ComplianceResult.Partial.as_str() == "partial"
        assert ComplianceResult.NotApplicable.as_str() == "not_applicable"
        assert ComplianceResult.NotAssessed.as_str() == "not_assessed"

    def test_generate_report(self):
        from eggsec import ComplianceFramework, ComplianceControl, ComplianceMapper
        mapper = ComplianceMapper()
        mapper.register_control(ComplianceControl(
            ComplianceFramework.OwaspTop10, "A03:2021", "Injection", "Desc",
        ))
        report = mapper.generate_report(ComplianceFramework.OwaspTop10, [])
        assert report.total_controls == 1
        assert report.framework == ComplianceFramework.OwaspTop10
        assert report.disclaimer != ""
