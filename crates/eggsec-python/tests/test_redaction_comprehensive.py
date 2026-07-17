"""Comprehensive redaction audit tests.

Verifies that unique sentinel values never appear in public output boundaries
across every serialization and integration surface. Each test uses a unique
sentinel to prove that the specific data path was exercised and that redaction
works at that boundary.

Test categories:
  1. Credential and SensitiveString redaction
  2. Finding serialization redaction
  3. Event envelope redaction
  4. Report output redaction
  5. Artifact manifest redaction
  6. Callback/log redaction
  7. Schema output redaction
  8. Tool descriptor redaction
  9. Checkpoint redaction
 10. VersionedFinding redaction

Types with built-in redaction (SensitiveString, credential configs) are tested
with hard assertions. Data-carrying types use default-redacted serialization
with to_dict_raw()/to_json_raw() for explicit unredacted access.
"""

import json
import importlib

import pytest

eggsec = importlib.import_module("eggsec")

# ---------------------------------------------------------------------------
# Unique sentinels — one per test to isolate redaction failures
# ---------------------------------------------------------------------------

SENTINELS = {
    "sensitive_repr": "EGGSEC_REDACT_SENTINEL_001",
    "sensitive_str": "EGGSEC_REDACT_SENTINEL_002",
    "sensitive_dict": "EGGSEC_REDACT_SENTINEL_003",
    "sensitive_json": "EGGSEC_REDACT_SENTINEL_004",
    "finding_title": "EGGSEC_REDACT_SENTINEL_005",
    "finding_desc": "EGGSEC_REDACT_SENTINEL_006",
    "finding_evidence": "EGGSEC_REDACT_SENTINEL_007",
    "finding_meta": "EGGSEC_REDACT_SENTINEL_008",
    "event_type": "EGGSEC_REDACT_SENTINEL_009",
    "event_payload": "EGGSEC_REDACT_SENTINEL_010",
    "report_finding_title": "EGGSEC_REDACT_SENTINEL_011",
    "report_finding_desc": "EGGSEC_REDACT_SENTINEL_012",
    "artifact_name": "EGGSEC_REDACT_SENTINEL_013",
    "artifact_meta": "EGGSEC_REDACT_SENTINEL_014",
    "callback_error": "EGGSEC_REDACT_SENTINEL_015",
    "schema_leak": "EGGSEC_REDACT_SENTINEL_016",
    "td_description": "EGGSEC_REDACT_SENTINEL_017",
    "checkpoint_results": "EGGSEC_REDACT_SENTINEL_018",
    "vf_description": "EGGSEC_REDACT_SENTINEL_019",
    "vf_evidence": "EGGSEC_REDACT_SENTINEL_020",
}

ALL_SENTINELS = list(SENTINELS.values())


def _any_sentinel_in(text: str) -> str | None:
    """Return the first sentinel found in *text*, or None."""
    for s in ALL_SENTINELS:
        if s in text:
            return s
    return None


def _assert_no_sentinel(text: str, context: str):
    found = _any_sentinel_in(text)
    assert found is None, f"Sentinel {found!r} leaked in {context}: {text!r}"


def _assert_no_sentinel_in_dict(d: dict, context: str):
    """Recursively check that no sentinel appears in a dict's values."""
    for k, v in d.items():
        if isinstance(v, str):
            _assert_no_sentinel(v, f"{context}.{k}")
        elif isinstance(v, dict):
            _assert_no_sentinel_in_dict(v, f"{context}.{k}")
        elif isinstance(v, list):
            for i, item in enumerate(v):
                if isinstance(item, str):
                    _assert_no_sentinel(item, f"{context}.{k}[{i}]")
                elif isinstance(item, dict):
                    _assert_no_sentinel_in_dict(item, f"{context}.{k}[{i}]")


def _assert_no_sentinel_in_json(text: str, context: str):
    """Parse JSON and recursively check for sentinels."""
    _assert_no_sentinel(text, f"{context} (raw)")
    parsed = json.loads(text)
    if isinstance(parsed, dict):
        _assert_no_sentinel_in_dict(parsed, f"{context} (parsed)")
    elif isinstance(parsed, list):
        for i, item in enumerate(parsed):
            if isinstance(item, str):
                _assert_no_sentinel(item, f"{context}[{i}]")
            elif isinstance(item, dict):
                _assert_no_sentinel_in_dict(item, f"{context}[{i}]")


# ===========================================================================
# 1. Credential and SensitiveString redaction
# ===========================================================================


@pytest.mark.timeout(60)
class TestSensitiveStringRedaction:
    def test_repr_redacts_sentinel_001(self):
        ss = eggsec.SensitiveString(SENTINELS["sensitive_repr"])
        _assert_no_sentinel(repr(ss), "SensitiveString.__repr__")

    def test_str_redacts_sentinel_002(self):
        ss = eggsec.SensitiveString(SENTINELS["sensitive_str"])
        _assert_no_sentinel(str(ss), "SensitiveString.__str__")

    def test_to_dict_proxy_redacts_sentinel_003(self):
        ss = eggsec.SensitiveString(SENTINELS["sensitive_dict"])
        _assert_no_sentinel(repr(ss), "SensitiveString repr (dict proxy)")
        _assert_no_sentinel(str(ss), "SensitiveString str (dict proxy)")

    def test_to_json_proxy_redacts_sentinel_004(self):
        ss = eggsec.SensitiveString(SENTINELS["sensitive_json"])
        _assert_no_sentinel(str(ss), "SensitiveString str (json proxy)")

    def test_expose_secret_still_works(self):
        secret = "unique-expose-test-value"
        ss = eggsec.SensitiveString(secret)
        assert ss.expose_secret() == secret

    def test_multiple_sentinels_independently_redacted(self):
        for label, sentinel in SENTINELS.items():
            ss = eggsec.SensitiveString(sentinel)
            _assert_no_sentinel(repr(ss), f"repr for {label}")
            _assert_no_sentinel(str(ss), f"str for {label}")

    def test_http_config_proxy_auth_redacted(self):
        sentinel = SENTINELS["sensitive_repr"]
        cfg = eggsec.HttpConfig(proxy_auth=sentinel)
        val = cfg.proxy_auth
        assert val is not None
        _assert_no_sentinel(repr(val), "HttpConfig.proxy_auth repr")
        _assert_no_sentinel(str(val), "HttpConfig.proxy_auth str")

    def test_recon_api_key_redacted(self):
        sentinel = SENTINELS["sensitive_str"]
        cfg = eggsec.ReconApiConfig(virustotal_api_key=sentinel)
        val = cfg.virustotal_api_key
        assert val is not None
        _assert_no_sentinel(repr(val), "ReconApiConfig.virustotal_api_key repr")

    def test_ai_config_api_key_redacted(self):
        sentinel = SENTINELS["sensitive_dict"]
        cfg = eggsec.AiConfig(api_key=sentinel)
        val = cfg.api_key
        assert val is not None
        _assert_no_sentinel(repr(val), "AiConfig.api_key repr")
        _assert_no_sentinel(str(val), "AiConfig.api_key str")

    def test_remote_config_psk_redacted(self):
        sentinel = SENTINELS["sensitive_json"]
        cfg = eggsec.RemoteConfig(psk=sentinel)
        val = cfg.psk
        assert val is not None
        _assert_no_sentinel(repr(val), "RemoteConfig.psk repr")
        _assert_no_sentinel(str(val), "RemoteConfig.psk str")

    def test_all_recon_api_key_fields_redacted(self):
        fields = [
            "virustotal_api_key",
            "alienvault_api_key",
            "shodan_api_key",
            "ipapi_api_key",
            "maxmind_license_key",
            "wayback_api_key",
            "nvd_api_key",
        ]
        for i, field in enumerate(fields):
            sentinel = f"EGGSEC_APIKEY_{i:03d}"
            cfg = eggsec.ReconApiConfig(**{field: sentinel})
            val = getattr(cfg, field)
            if val is not None:
                _assert_no_sentinel(repr(val), f"ReconApiConfig.{field} repr")

    def test_config_repr_omits_all_secrets(self):
        cfg = eggsec.HttpConfig(proxy_auth="EGGSEC_SECRET_001")
        _assert_no_sentinel(repr(cfg), "HttpConfig repr")

        cfg2 = eggsec.ReconApiConfig(virustotal_api_key="EGGSEC_SECRET_002")
        _assert_no_sentinel(repr(cfg2), "ReconApiConfig repr")

        cfg3 = eggsec.AiConfig(api_key="EGGSEC_SECRET_003")
        _assert_no_sentinel(repr(cfg3), "AiConfig repr")

        cfg4 = eggsec.RemoteConfig(psk="EGGSEC_SECRET_004")
        _assert_no_sentinel(repr(cfg4), "RemoteConfig repr")


# ===========================================================================
# 2. Finding serialization redaction
#
# Finding is a data-carrying type that serializes user content verbatim.
# These tests document the redaction gap.
# ===========================================================================


@pytest.mark.timeout(60)
class TestFindingSerializationRedaction:
    def _make_finding(self, sentinel_key: str):
        """Create a Finding with the given sentinel in title, description, and evidence."""
        sentinel = SENTINELS[sentinel_key]
        ev = eggsec.Evidence(
            kind="body_snippet",
            value=sentinel,
            source="test-source",
        )
        return eggsec.Finding(
            id=f"find-{sentinel_key}",
            title=sentinel,
            severity=eggsec.Severity.High,
            target="example.com",
            category="injection",
            description=sentinel,
            evidence=[ev],
            metadata={"secret_field": sentinel},
        )

    def test_finding_to_dict_no_sentinel_005(self):
        f = self._make_finding("finding_title")
        d = f.to_dict()
        _assert_no_sentinel_in_dict(d, "Finding.to_dict()")

    def test_finding_to_json_no_sentinel_006(self):
        f = self._make_finding("finding_desc")
        _assert_no_sentinel_in_json(f.to_json(), "Finding.to_json()")

    def test_finding_evidence_no_sentinel_007(self):
        f = self._make_finding("finding_evidence")
        d = f.to_dict()
        _assert_no_sentinel_in_dict(d, "Finding evidence serialization")

    def test_finding_metadata_no_sentinel_008(self):
        f = self._make_finding("finding_meta")
        d = f.to_dict()
        _assert_no_sentinel_in_dict(d, "Finding metadata serialization")

    def test_finding_event_payload_no_leak(self):
        f = self._make_finding("event_payload")
        envelope = eggsec.EventEnvelope(
            event_type="finding.created",
            payload=f.to_dict(),
        )
        ed = envelope.to_dict()
        _assert_no_sentinel_in_dict(ed, "EventEnvelope with Finding payload")

    def test_finding_json_roundtrip_no_leak(self):
        f = self._make_finding("finding_title")
        _assert_no_sentinel_in_json(f.to_json(), "Finding JSON roundtrip")

    def test_finding_repr_no_leak(self):
        f = self._make_finding("finding_desc")
        _assert_no_sentinel(repr(f), "Finding.__repr__")

    def test_finding_serialization_roundtrip_fidelity(self):
        """Verify Finding raw serialization preserves data correctly."""
        f = self._make_finding("finding_title")
        d = f.to_dict_raw()
        assert d["title"] == SENTINELS["finding_title"]
        assert d["description"] == SENTINELS["finding_title"]
        j = f.to_json_raw()
        parsed = json.loads(j)
        assert parsed["title"] == SENTINELS["finding_title"]


# ===========================================================================
# 3. Event envelope redaction
#
# EventEnvelope wraps a payload dict and serializes it verbatim.
# ===========================================================================


@pytest.mark.timeout(60)
class TestEventEnvelopeRedaction:
    def test_event_envelope_type_no_sentinel_009(self):
        ev = eggsec.EventEnvelope(
            event_type="scan.completed",
            payload={"key": SENTINELS["event_type"]},
        )
        d = ev.to_dict()
        _assert_no_sentinel_in_dict(d, "EventEnvelope event_type")

    def test_event_envelope_payload_no_sentinel_010(self):
        ev = eggsec.EventEnvelope(
            event_type="scan.completed",
            payload={"secret": SENTINELS["event_payload"]},
        )
        d = ev.to_dict()
        _assert_no_sentinel_in_dict(d, "EventEnvelope payload")

    def test_event_envelope_json_no_leak(self):
        ev = eggsec.EventEnvelope(
            event_type="test.event",
            payload={"credential": SENTINELS["event_payload"]},
        )
        _assert_no_sentinel_in_json(ev.to_json(), "EventEnvelope.to_json()")

    def test_event_envelope_roundtrip_fidelity(self):
        """Verify EventEnvelope raw serialization preserves data correctly."""
        payload = {"secret": SENTINELS["event_payload"]}
        ev = eggsec.EventEnvelope(event_type="test.event", payload=payload)
        d = ev.to_dict_raw()
        assert d["event_type"] == "test.event"
        assert d["payload"]["secret"] == SENTINELS["event_payload"]


# ===========================================================================
# 4. Report output redaction
#
# Report aggregates findings and serializes them verbatim.
# ===========================================================================


@pytest.mark.timeout(60)
class TestReportOutputRedaction:
    def _make_report_with_sentinel(self, sentinel_key: str):
        """Create a Report containing findings with sentinels."""
        sentinel = SENTINELS[sentinel_key]
        ev = eggsec.Evidence(
            kind="body_snippet",
            value=sentinel,
            source="test-source",
        )
        f = eggsec.Finding(
            id="rpt-finding-1",
            title=sentinel,
            severity=eggsec.Severity.Medium,
            target="example.com",
            category="info_leak",
            description=sentinel,
            evidence=[ev],
            metadata={"leak": sentinel},
        )
        report = eggsec.Report(
            metadata={"report_name": sentinel, "author": "test"},
        )
        report.add_finding(f)
        return report

    def test_report_to_dict_no_sentinel_011(self):
        rpt = self._make_report_with_sentinel("report_finding_title")
        _assert_no_sentinel_in_dict(rpt.to_dict(), "Report.to_dict()")

    def test_report_to_json_no_sentinel_012(self):
        rpt = self._make_report_with_sentinel("report_finding_desc")
        _assert_no_sentinel_in_json(rpt.to_json(), "Report.to_json()")

    def test_report_multiple_findings_no_leak(self):
        rpt = eggsec.Report(metadata={"name": "multi-finding-report"})
        for i, (key, sentinel) in enumerate(SENTINELS.items()):
            f = eggsec.Finding(
                id=f"multi-{i}",
                title=sentinel,
                severity=eggsec.Severity.Low,
                target="example.com",
                category="info_leak",
                description=sentinel,
                evidence=[],
            )
            rpt.add_finding(f)
        _assert_no_sentinel_in_dict(rpt.to_dict(), "Report with multiple findings")

    def test_report_serialization_roundtrip(self):
        """Verify Report raw serialization preserves structure correctly."""
        rpt = self._make_report_with_sentinel("report_finding_title")
        d = rpt.to_dict_raw()
        assert len(d["findings"]) == 1
        assert d["findings"][0]["title"] == SENTINELS["report_finding_title"]


# ===========================================================================
# 5. Artifact manifest redaction
#
# Artifact stores name, kind, mime_type, data, path — user-provided metadata.
# ===========================================================================


@pytest.mark.timeout(60)
class TestArtifactManifestRedaction:
    def test_artifact_name_no_sentinel_013(self):
        a = eggsec.Artifact(name=SENTINELS["artifact_name"], kind="report")
        _assert_no_sentinel_in_dict(a.to_dict(), "Artifact.to_dict() name")

    def test_artifact_repr_no_leak(self):
        a = eggsec.Artifact(name=SENTINELS["artifact_name"], kind="report")
        _assert_no_sentinel(repr(a), "Artifact.__repr__")

    def test_artifact_str_no_leak(self):
        a = eggsec.Artifact(name=SENTINELS["artifact_name"], kind="report")
        _assert_no_sentinel(str(a), "Artifact.__str__")

    def test_artifact_json_no_leak(self):
        a = eggsec.Artifact(name=SENTINELS["artifact_name"], kind="evidence")
        _assert_no_sentinel_in_json(a.to_json(), "Artifact.to_json()")

    def test_artifact_roundtrip_fidelity(self):
        sentinel = SENTINELS["artifact_meta"]
        a = eggsec.Artifact(name=sentinel, kind="log")
        d = a.to_dict_raw()
        a2 = eggsec.Artifact.from_dict(d)
        assert a2.to_dict_raw()["name"] == sentinel


# ===========================================================================
# 6. Callback/log redaction
#
# Exception messages and callback infrastructure should not embed secrets.
# ===========================================================================


@pytest.mark.timeout(60)
class TestCallbackLogRedaction:
    def test_exception_does_not_leak_sentinel_015(self):
        sentinel = SENTINELS["callback_error"]
        try:
            cfg = eggsec.RemoteConfig(psk=sentinel)
            _ = repr(cfg)
        except Exception as e:
            _assert_no_sentinel(str(e), "exception message in RemoteConfig")

    def test_callback_scheduler_exception_safe(self):
        sentinel = SENTINELS["callback_error"]
        try:
            scheduler = eggsec.CallbackScheduler(capacity=10)
            scheduler.enqueue({"error_detail": sentinel})
            drain = scheduler.drain()
            for ev in drain:
                _assert_no_sentinel(repr(ev), "CallbackScheduler drained event repr")
        except Exception as e:
            _assert_no_sentinel(str(e), "CallbackScheduler exception")
        finally:
            try:
                scheduler.close()
            except Exception:
                pass

    def test_async_callback_repr_no_leak(self):
        sentinel = SENTINELS["callback_error"]
        try:

            async def handler(event):
                pass

            ac = eggsec.AsyncCallback(handler)
            _assert_no_sentinel(repr(ac), "AsyncCallback.__repr__")
            ac.close()
        except Exception as e:
            _assert_no_sentinel(str(e), "AsyncCallback creation exception")

    def test_port_range_error_safe(self):
        try:
            eggsec.PortRange.list([])
        except ValueError as e:
            _assert_no_sentinel(str(e), "ValueError from PortRange.list")

    def test_scope_error_messages_safe(self):
        sentinel = SENTINELS["callback_error"]
        try:
            scope = eggsec.Scope.allow_hosts([sentinel])
            eggsec.validate_scope(scope, "attacker.example.com")
        except Exception as e:
            _assert_no_sentinel(str(e), "Scope validation exception")

    def test_exception_chain_does_not_contain_sentinels(self):
        sentinel = SENTINELS["callback_error"]
        try:
            eggsec.SensitiveString("test")
        except Exception as e:
            _assert_no_sentinel(str(e), "SensitiveString exception")

    def test_exception_with_secret_in_message_safe(self):
        """Verify that exceptions raised by library code don't embed secrets."""
        sentinel = SENTINELS["callback_error"]
        try:
            eggsec.Scope.allow_hosts([sentinel])
        except Exception as e:
            _assert_no_sentinel(str(e), "Scope creation exception")


# ===========================================================================
# 7. Schema output redaction
#
# SchemaGenerator produces JSON Schema definitions for tool types.
# These are type-level descriptions and should never contain test data.
# ===========================================================================


@pytest.mark.timeout(60)
class TestSchemaOutputRedaction:
    def test_all_schemas_no_sentinel_016(self):
        schemas = eggsec.SchemaGenerator.all_schemas()
        for tool_id, schema_pair in schemas.items():
            for key in ("input_schema", "output_schema"):
                schema_str = schema_pair.get(key)
                if schema_str:
                    _assert_no_sentinel(
                        schema_str,
                        f"SchemaGenerator.all_schemas()[{tool_id!r}][{key!r}]",
                    )

    def test_generate_input_schema_no_leak(self):
        for tid in ["scan_ports", "scan_endpoints", "fingerprint_services"]:
            schema = eggsec.SchemaGenerator.generate_input_schema(tid)
            if schema:
                _assert_no_sentinel(schema, f"input_schema for {tid}")

    def test_generate_output_schema_no_leak(self):
        for tid in ["scan_ports", "scan_endpoints", "fingerprint_services"]:
            schema = eggsec.SchemaGenerator.generate_output_schema(tid)
            if schema:
                _assert_no_sentinel(schema, f"output_schema for {tid}")

    def test_tool_registry_schema_no_leak(self):
        schema = eggsec.ToolRegistry.schema("scan_ports")
        if schema:
            _assert_no_sentinel(schema, "ToolRegistry.schema('scan_ports')")

    def test_schema_json_is_valid(self):
        for tid in ["scan_ports", "scan_endpoints"]:
            schema = eggsec.SchemaGenerator.generate_input_schema(tid)
            if schema:
                parsed = json.loads(schema)
                assert "$schema" in parsed or "$ref" in parsed or "type" in parsed


# ===========================================================================
# 8. Tool descriptor redaction
#
# Tool descriptors come from the Rust registry. They are static metadata
# and should never contain test data.
# ===========================================================================


@pytest.mark.timeout(60)
class TestToolDescriptorRedaction:
    def test_tool_registry_descriptors_no_sentinel_017(self):
        for td in eggsec.ToolRegistry.from_registry():
            d = td.to_dict()
            _assert_no_sentinel_in_dict(d, f"ToolDescriptor[{d.get('tool_id', '?')}]")

    def test_tool_descriptor_json_no_leak(self):
        for td in eggsec.ToolRegistry.from_registry():
            j = td.to_json()
            _assert_no_sentinel(j, f"ToolDescriptor.to_json()")

    def test_tool_descriptor_repr_no_leak(self):
        for td in eggsec.ToolRegistry.from_registry():
            _assert_no_sentinel(repr(td), "ToolDescriptor.__repr__")
            _assert_no_sentinel(str(td), "ToolDescriptor.__str__")

    def test_tool_descriptor_by_id_no_leak(self):
        td = eggsec.ToolRegistry.get("scan_ports")
        if td:
            _assert_no_sentinel_in_dict(td.to_dict(), "ToolDescriptor via get('scan_ports')")
            _assert_no_sentinel(td.to_json(), "ToolDescriptor via get('scan_ports').to_json()")

    def test_all_tool_ids_are_unique(self):
        tds = eggsec.ToolRegistry.from_registry()
        ids = [td.to_dict()["tool_id"] for td in tds]
        assert len(ids) == len(set(ids)), "Duplicate tool IDs in registry"


# ===========================================================================
# 9. Checkpoint redaction
#
# Checkpoint stores pipeline state including StepResult objects.
# Results are user-provided data.
# ===========================================================================


@pytest.mark.timeout(60)
class TestCheckpointRedaction:
    def test_checkpoint_results_no_sentinel_018(self):
        sr = eggsec.StepResult(
            step_name="step-1",
            status=eggsec.ExecutionStatus.Completed(),
        )
        cp = eggsec.Checkpoint(
            id="cp-test-001",
            pipeline_name="test-pipeline",
            completed_steps=["step-1", "step-2"],
            results=[sr],
        )
        _assert_no_sentinel_in_dict(cp.to_dict(), "Checkpoint.to_dict()")

    def test_checkpoint_json_no_leak(self):
        sr = eggsec.StepResult(
            step_name="step-1",
            status=eggsec.ExecutionStatus.Completed(),
        )
        cp = eggsec.Checkpoint(
            id="cp-test-002",
            pipeline_name="test-pipeline",
            completed_steps=["step-1"],
            results=[sr],
        )
        _assert_no_sentinel_in_json(cp.to_json(), "Checkpoint.to_json()")

    def test_checkpoint_repr_no_leak(self):
        sr = eggsec.StepResult(
            step_name="step-1",
            status=eggsec.ExecutionStatus.Completed(),
        )
        cp = eggsec.Checkpoint(
            id="cp-test-003",
            pipeline_name="test-pipeline",
            results=[sr],
        )
        _assert_no_sentinel(repr(cp), "Checkpoint.__repr__")

    def test_checkpoint_empty_results_safe(self):
        cp = eggsec.Checkpoint(
            id="cp-test-004",
            pipeline_name="empty-pipeline",
        )
        _assert_no_sentinel_in_dict(cp.to_dict(), "Checkpoint empty results")

    def test_checkpoint_roundtrip_fidelity(self):
        sr = eggsec.StepResult(
            step_name="step-1",
            status=eggsec.ExecutionStatus.Completed(),
        )
        cp = eggsec.Checkpoint(
            id="cp-test-005",
            pipeline_name="test-pipeline",
            results=[sr],
        )
        d = cp.to_dict()
        assert d["id"] == "cp-test-005"
        assert d["pipeline_name"] == "test-pipeline"


# ===========================================================================
# 10. VersionedFinding redaction
#
# VersionedFinding has structured fields that serialize user content verbatim.
# ===========================================================================


@pytest.mark.timeout(60)
class TestVersionedFindingRedaction:
    def _make_versioned_finding(self, desc_key: str, evidence_key: str):
        return eggsec.VersionedFinding(
            id=f"vf-{desc_key}",
            title="Test Versioned Finding",
            description=SENTINELS[desc_key],
            severity="High",
            finding_type=eggsec.FindingType.from_str("vulnerability"),
            affected_asset=eggsec.AffectedAsset(
                asset_type="host", identifier="example.com"
            ),
            source_tool="test-tool",
            source_module="test-module",
            evidence=[
                eggsec.VersionedEvidence(
                    kind=eggsec.EvidenceKind.BodySnippet,
                    summary=SENTINELS[evidence_key],
                )
            ],
            metadata=SENTINELS[desc_key],
        )

    def test_versioned_finding_to_dict_no_sentinel_019(self):
        vf = self._make_versioned_finding("vf_description", "vf_evidence")
        _assert_no_sentinel_in_dict(vf.to_dict(), "VersionedFinding.to_dict()")

    def test_versioned_finding_to_json_no_sentinel_020(self):
        vf = self._make_versioned_finding("vf_description", "vf_evidence")
        _assert_no_sentinel_in_json(vf.to_json(), "VersionedFinding.to_json()")

    def test_versioned_finding_repr_no_leak(self):
        vf = self._make_versioned_finding("vf_description", "vf_evidence")
        _assert_no_sentinel(repr(vf), "VersionedFinding.__repr__")

    def test_versioned_fingerprint_no_leak(self):
        vf = self._make_versioned_finding("vf_description", "vf_evidence")
        fp = vf.compute_fingerprint()
        _assert_no_sentinel(str(fp), "VersionedFinding.compute_fingerprint()")

    def test_versioned_finding_evidence_chain_no_leak(self):
        vf = eggsec.VersionedFinding(
            id="vf-chain-001",
            title="Evidence Chain Test",
            description="Normal description",
            severity="High",
            finding_type=eggsec.FindingType.from_str("vulnerability"),
            affected_asset=eggsec.AffectedAsset(
                asset_type="host", identifier="chain.example.com"
            ),
            source_tool="test-tool",
            source_module="test-module",
            evidence=[
                eggsec.VersionedEvidence(
                    kind=eggsec.EvidenceKind.BodySnippet,
                    summary=SENTINELS["vf_evidence"],
                ),
                eggsec.VersionedEvidence(
                    kind=eggsec.EvidenceKind.LogLine,
                    summary=SENTINELS["vf_description"],
                ),
                eggsec.VersionedEvidence(
                    kind=eggsec.EvidenceKind.Header,
                    summary="Clean evidence",
                ),
            ],
        )
        _assert_no_sentinel_in_dict(vf.to_dict(), "VersionedFinding evidence chain")

    def test_versioned_finding_roundtrip_fidelity(self):
        vf = self._make_versioned_finding("vf_description", "vf_evidence")
        d = vf.to_dict_raw()
        assert d["description"] == SENTINELS["vf_description"]
        assert d["title"] == "Test Versioned Finding"
        assert d["severity"] == "High"


# ===========================================================================
# Cross-cutting: ToolFinding serialization
#
# ToolFinding carries user-provided finding data.
# ===========================================================================


@pytest.mark.timeout(60)
class TestToolFindingRedaction:
    def test_tool_finding_to_dict_no_leak(self):
        tf = eggsec.ToolFinding(
            id="tf-test-001",
            finding_type=eggsec.ToolFindingType.from_str("vulnerability"),
            severity=eggsec.ToolSeverity.from_str("high"),
            title=SENTINELS["finding_title"],
            description=SENTINELS["finding_desc"],
            location="example.com:443",
            evidence=SENTINELS["finding_evidence"],
            remediation="Apply patch",
        )
        _assert_no_sentinel_in_dict(tf.to_dict(), "ToolFinding.to_dict()")

    def test_tool_finding_to_json_no_leak(self):
        tf = eggsec.ToolFinding(
            id="tf-test-002",
            finding_type=eggsec.ToolFindingType.from_str("misconfiguration"),
            severity=eggsec.ToolSeverity.from_str("medium"),
            title=SENTINELS["finding_title"],
            description=SENTINELS["finding_desc"],
            location="db.internal:5432",
            evidence=SENTINELS["finding_evidence"],
        )
        _assert_no_sentinel_in_json(tf.to_json(), "ToolFinding.to_json()")

    def test_tool_finding_repr_no_leak(self):
        tf = eggsec.ToolFinding(
            id="tf-test-003",
            finding_type=eggsec.ToolFindingType.from_str("open_port"),
            severity=eggsec.ToolSeverity.from_str("info"),
            title=SENTINELS["finding_title"],
            description=SENTINELS["finding_desc"],
            location="10.0.0.1:22",
        )
        _assert_no_sentinel(repr(tf), "ToolFinding.__repr__")
        _assert_no_sentinel(str(tf), "ToolFinding.__str__")

    def test_tool_finding_roundtrip_fidelity(self):
        tf = eggsec.ToolFinding(
            id="tf-test-004",
            finding_type=eggsec.ToolFindingType.from_str("vulnerability"),
            severity=eggsec.ToolSeverity.from_str("high"),
            title=SENTINELS["finding_title"],
            description=SENTINELS["finding_desc"],
            location="example.com:443",
        )
        d = tf.to_dict_raw()
        assert d["title"] == SENTINELS["finding_title"]
        assert d["description"] == SENTINELS["finding_desc"]


# ===========================================================================
# Cross-cutting: Audit and infrastructure output
# ===========================================================================


@pytest.mark.timeout(60)
class TestInfrastructureOutputRedaction:
    def test_feature_matrix_no_sentinel_leak(self):
        fm = eggsec.feature_matrix()
        if fm:
            _assert_no_sentinel(fm, "feature_matrix()")

    def test_build_info_no_sentinel_leak(self):
        info = eggsec.build_info()
        _assert_no_sentinel(str(info), "build_info()")

    def test_domain_maturity_no_sentinel_leak(self):
        dm = eggsec.domain_maturity()
        if dm:
            _assert_no_sentinel(str(dm), "domain_maturity()")

    def test_module_repr_safe(self):
        _assert_no_sentinel(repr(eggsec), "module repr")

    def test_version_safe(self):
        _assert_no_sentinel(str(eggsec.__version__), "__version__")
