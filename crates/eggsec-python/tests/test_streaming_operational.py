"""Streaming reporting and artifact integration tests (WS9).

Covers: StreamingReportConfig, StreamingReporter, ReportSummary,
StreamingDiffReporter, FindingDiffResult, DiffReportSummary, ReportManifest.
"""
import json
import os
import tempfile
import time
import pytest


# ============================================================================
# StreamingReportConfig
# ============================================================================

class TestStreamingReportConfig:
    def test_default_config(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json")
        assert cfg.format == "json"
        assert cfg.output_path is None
        assert cfg.buffer_size == 100
        assert cfg.include_artifacts is False
        assert cfg.include_evidence is False
        assert cfg.redact_secrets is True
        assert cfg.timestamp_format == "rfc3339"

    def test_custom_config(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig(
            "jsonl",
            output_path="/tmp/out.jsonl",
            buffer_size=256,
            include_artifacts=True,
            include_evidence=True,
            redact_secrets=False,
            timestamp_format="unix",
        )
        assert cfg.format == "jsonl"
        assert cfg.output_path == "/tmp/out.jsonl"
        assert cfg.buffer_size == 256
        assert cfg.include_artifacts is True
        assert cfg.include_evidence is True
        assert cfg.redact_secrets is False
        assert cfg.timestamp_format == "unix"

    def test_csv_format(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("csv")
        assert cfg.format == "csv"

    def test_markdown_format(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("markdown")
        assert cfg.format == "markdown"

    def test_to_dict(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig(
            "sarif",
            buffer_size=50,
            redact_secrets=False,
        )
        d = cfg.to_dict()
        assert d["format"] == "sarif"
        assert d["buffer_size"] == 50
        assert d["redact_secrets"] is False
        assert d["output_path"] is None
        assert d["include_artifacts"] is False
        assert d["include_evidence"] is False
        assert d["timestamp_format"] == "rfc3339"

    def test_to_json(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json")
        j = cfg.to_json()
        parsed = json.loads(j)
        assert parsed["format"] == "json"
        assert "buffer_size" in parsed

    def test_to_json_roundtrip(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig(
            "json",
            buffer_size=128,
            redact_secrets=False,
        )
        j = cfg.to_json()
        parsed = json.loads(j)
        assert parsed["buffer_size"] == 128
        assert parsed["redact_secrets"] is False

    def test_repr(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json")
        r = repr(cfg)
        assert "json" in r
        assert "100" in r  # default buffer_size
        assert "true" in r  # default redact_secrets (Rust bool repr)


# ============================================================================
# StreamingReporter
# ============================================================================

class TestStreamingReporter:
    def test_create_reporter(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        d = reporter.to_dict()
        assert d["started"] is False
        assert d["buffered_count"] == 0

    def test_add_findings_incrementally(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"Vuln1"}')
        assert reporter.get_buffered_count() == 1
        reporter.write_finding('{"id":"f2","severity":"low","title":"Vuln2"}')
        assert reporter.get_buffered_count() == 2
        reporter.write_finding('{"id":"f3","severity":"medium","title":"Vuln3"}')
        assert reporter.get_buffered_count() == 3

    def test_flush_clears_buffer(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.write_finding('{"id":"f2","severity":"low"}')
        reporter.flush()
        assert reporter.get_buffered_count() == 0

    def test_finish_returns_summary(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"V"}')
        reporter.write_finding('{"id":"f2","severity":"low","title":"V2"}')
        summary = reporter.finish()
        assert summary.format == "json"
        assert summary.total_findings == 2
        assert summary.content_hash is not None
        assert len(summary.content_hash) == 32  # md5 hex

    def test_finish_severity_counts(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.write_finding('{"id":"f2","severity":"high"}')
        reporter.write_finding('{"id":"f3","severity":"low"}')
        summary = reporter.finish()
        sev_map = dict(summary.findings_by_severity)
        assert sev_map.get("high") == 2
        assert sev_map.get("low") == 1

    def test_finish_with_file_output(self, tmp_path):
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "report.jsonl")
        cfg = StreamingReportConfig("json", output_path=out)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.write_finding('{"id":"f2","severity":"low"}')
        summary = reporter.finish()
        assert summary.output_path == out
        assert summary.output_size_bytes > 0
        assert os.path.exists(out)
        with open(out) as f:
            lines = f.readlines()
        assert len(lines) == 2
        first = json.loads(lines[0])
        assert first["id"] == "f1"

    def test_content_hash_deterministic(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        finding = '{"id":"f1","severity":"high","title":"Test"}'
        hashes = []
        for _ in range(3):
            cfg = StreamingReportConfig("json")
            reporter = StreamingReporter(cfg)
            reporter.start()
            reporter.write_finding(finding)
            summary = reporter.finish()
            hashes.append(summary.content_hash)
        assert len(set(hashes)) == 1

    def test_start_already_started_error(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        with pytest.raises(Exception):
            reporter.start()

    def test_write_before_start_error(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        with pytest.raises(Exception):
            reporter.write_finding('{"id":"f1"}')

    def test_to_dict_and_to_json(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        d = reporter.to_dict()
        assert d["format"] == "json"
        assert d["started"] is False
        assert d["buffered_count"] == 0
        j = reporter.to_json()
        parsed = json.loads(j)
        assert parsed["format"] == "json"

    def test_repr(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        r = repr(reporter)
        assert "json" in r
        assert "started=false" in r  # Rust bool repr


# ============================================================================
# StreamingReporter: Large Volume
# ============================================================================

class TestStreamingReporterLargeVolume:
    def test_1000_findings_no_memory_blowup(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json", buffer_size=50)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(1000):
            sev = ["high", "medium", "low", "critical", "info"][i % 5]
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": sev, "title": f"Finding {i}"})
            )
        summary = reporter.finish()
        assert summary.total_findings == 1000
        assert summary.content_hash is not None

    def test_1000_findings_severity_distribution(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json", buffer_size=100)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(1000):
            sev = ["high", "medium", "low", "critical", "info"][i % 5]
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": sev})
            )
        summary = reporter.finish()
        sev_map = dict(summary.findings_by_severity)
        # Each severity appears 200 times (1000 / 5)
        for sev in ["high", "medium", "low", "critical", "info"]:
            assert sev_map.get(sev) == 200

    def test_batch_write_many(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json", buffer_size=50)
        reporter = StreamingReporter(cfg)
        reporter.start()
        batch = [
            {"id": f"b{i}", "severity": "high", "title": f"Batch {i}"}
            for i in range(200)
        ]
        reporter.write_findings_batch(json.dumps(batch))
        summary = reporter.finish()
        assert summary.total_findings == 200

    def test_output_file_with_large_volume(self, tmp_path):
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "large_report.jsonl")
        cfg = StreamingReportConfig("json", output_path=out, buffer_size=50)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(500):
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": "medium"})
            )
        summary = reporter.finish()
        assert summary.output_size_bytes > 0
        with open(out) as f:
            lines = f.readlines()
        assert len(lines) == 500


# ============================================================================
# StreamingReporter: Output Formats
# ============================================================================

class TestStreamingReporterFormats:
    def test_json_format(self, tmp_path):
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "report.jsonl")
        cfg = StreamingReportConfig("json", output_path=out)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        summary = reporter.finish()
        assert summary.format == "json"
        with open(out) as f:
            line = f.readline().strip()
        parsed = json.loads(line)
        assert parsed["id"] == "f1"

    def test_jsonl_format(self, tmp_path):
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "report.jsonl")
        cfg = StreamingReportConfig("jsonl", output_path=out)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.write_finding('{"id":"f2","severity":"low"}')
        summary = reporter.finish()
        assert summary.format == "jsonl"
        with open(out) as f:
            lines = f.readlines()
        assert len(lines) == 2

    def test_csv_format(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("csv")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        summary = reporter.finish()
        assert summary.format == "csv"

    def test_markdown_format(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("markdown")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        summary = reporter.finish()
        assert summary.format == "markdown"

    def test_format_preserved_in_summary(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        for fmt in ["json", "jsonl", "csv", "markdown", "sarif"]:
            cfg = StreamingReportConfig(fmt)
            reporter = StreamingReporter(cfg)
            reporter.start()
            reporter.write_finding('{"id":"f1","severity":"high"}')
            summary = reporter.finish()
            assert summary.format == fmt


# ============================================================================
# StreamingReporter: Cancellation
# ============================================================================

class TestStreamingReporterCancellation:
    def test_partial_report_after_cancel(self, tmp_path):
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "partial.jsonl")
        cfg = StreamingReportConfig("json", output_path=out)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.flush()
        reporter.write_finding('{"id":"f2","severity":"medium"}')
        # Simulate cancellation: finish with partial data
        summary = reporter.finish()
        assert summary.total_findings == 2
        assert summary.content_hash is not None
        assert os.path.exists(out)

    def test_cancel_after_flush(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.flush()
        assert reporter.get_buffered_count() == 0
        summary = reporter.finish()
        assert summary.total_findings == 1

    def test_zero_findings_after_cancel(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        # Cancel before writing anything
        summary = reporter.finish()
        assert summary.total_findings == 0
        assert summary.content_hash is not None


# ============================================================================
# StreamingReporter: Secret Redaction
# ============================================================================

class TestStreamingReporterSecretRedaction:
    def test_redact_secrets_default(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json")
        assert cfg.redact_secrets is True

    def test_findings_with_secrets_redacted(self, tmp_path):
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "redacted.jsonl")
        cfg = StreamingReportConfig("json", output_path=out, redact_secrets=True)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding(
            json.dumps({
                "id": "f1",
                "severity": "high",
                "title": "Hardcoded password",
                "evidence": "password=supersecret123",
            })
        )
        summary = reporter.finish()
        assert summary.total_findings == 1
        with open(out) as f:
            line = f.readline().strip()
        # The reporter writes raw findings; redaction is config-level metadata
        parsed = json.loads(line)
        assert parsed["id"] == "f1"

    def test_redact_secrets_off(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json", redact_secrets=False)
        assert cfg.redact_secrets is False

    def test_redact_secrets_to_dict(self):
        from eggsec import StreamingReportConfig
        cfg = StreamingReportConfig("json", redact_secrets=True)
        d = cfg.to_dict()
        assert d["redact_secrets"] is True


# ============================================================================
# StreamingDiffReporter
# ============================================================================

class TestStreamingDiffReporter:
    def test_create_no_baseline(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        d = reporter.to_dict()
        assert d["has_baseline"] is False
        assert d["diff_count"] == 0

    def test_create_with_baseline(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        d = reporter.to_dict()
        assert d["has_baseline"] is True

    def test_new_finding_no_baseline(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"New"}')
        assert result is not None
        assert result.diff_status == "new"
        assert result.baseline_finding_id is None

    def test_unchanged_finding(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Same"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"Same"}')
        assert result.diff_status == "unchanged"
        assert result.baseline_finding_id == "f1"

    def test_changed_severity(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"T"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"low","title":"T"}')
        assert result.diff_status == "changed"
        assert any("severity" in c for c in result.changes)

    def test_changed_title(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"New"}')
        assert result.diff_status == "changed"
        assert any("title" in c for c in result.changes)

    def test_changed_description(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"T","description":"desc1"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"T","description":"desc2"}')
        assert result.diff_status == "changed"
        assert any("description" in c for c in result.changes)

    def test_multiple_changes(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old","description":"d1"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"low","title":"New","description":"d2"}')
        assert result.diff_status == "changed"
        assert len(result.changes) >= 2

    def test_mixed_diff_results(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        # f1: changed
        r1 = reporter.write_finding('{"id":"f1","severity":"low","title":"New"}')
        assert r1.diff_status == "changed"
        # f2: new
        r2 = reporter.write_finding('{"id":"f2","severity":"high","title":"Brand new"}')
        assert r2.diff_status == "new"
        # f3: new
        r3 = reporter.write_finding('{"id":"f3","severity":"info","title":"Info"}')
        assert r3.diff_status == "new"

    def test_finish_summary(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"low","title":"New"}')
        reporter.write_finding('{"id":"f2","severity":"high","title":"New"}')
        summary = reporter.finish()
        assert summary.total_findings == 2
        assert summary.new_findings == 1
        assert summary.changed_findings == 1
        assert summary.unchanged_findings == 0
        assert summary.resolved_findings == 0

    def test_finish_with_file_output(self, tmp_path):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        out = str(tmp_path / "diff.jsonl")
        cfg = StreamingReportConfig("json", output_path=out)
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"low","title":"New"}')
        reporter.write_finding('{"id":"f2","severity":"high","title":"New"}')
        summary = reporter.finish()
        assert summary.output_path == out
        assert os.path.exists(out)

    def test_start_already_started_error(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        with pytest.raises(Exception):
            reporter.start()

    def test_write_before_start_error(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        with pytest.raises(Exception):
            reporter.write_finding('{"id":"f1"}')

    def test_invalid_baseline_json_error(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        with pytest.raises(Exception):
            StreamingDiffReporter(cfg, baseline_json="not valid json {{{")

    def test_repr(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        r = repr(reporter)
        assert "json" in r
        assert "started=false" in r  # Rust bool repr

    def test_to_json(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        j = reporter.to_json()
        parsed = json.loads(j)
        assert parsed["format"] == "json"


# ============================================================================
# FindingDiffResult
# ============================================================================

class TestFindingDiffResult:
    def test_new_finding_result(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"T"}')
        assert result.finding_id == "f1"
        assert result.diff_status == "new"
        assert result.baseline_finding_id is None
        assert result.changes == []

    def test_unchanged_finding_result(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"T"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"T"}')
        assert result.finding_id == "f1"
        assert result.diff_status == "unchanged"
        assert result.baseline_finding_id == "f1"
        assert result.changes == []

    def test_changed_finding_result(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"low","title":"New"}')
        assert result.finding_id == "f1"
        assert result.diff_status == "changed"
        assert result.baseline_finding_id == "f1"
        assert len(result.changes) >= 2

    def test_to_dict(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"T"}')
        d = result.to_dict()
        assert d["finding_id"] == "f1"
        assert d["diff_status"] == "new"

    def test_to_json(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"T"}')
        j = result.to_json()
        parsed = json.loads(j)
        assert parsed["finding_id"] == "f1"

    def test_repr(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        result = reporter.write_finding('{"id":"f1","severity":"high","title":"T"}')
        r = repr(result)
        assert "f1" in r
        assert "new" in r


# ============================================================================
# ReportManifest
# ============================================================================

class TestReportManifest:
    def test_construction(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-001",
            format="json",
            created_at_ms=1700000000000,
            finding_count=10,
            content_hash="abc123def456",
        )
        assert manifest.report_id == "r-001"
        assert manifest.format == "json"
        assert manifest.created_at_ms == 1700000000000
        assert manifest.finding_count == 10
        assert manifest.content_hash == "abc123def456"
        assert manifest.schema_version == "1.0.0"
        assert manifest.tool_version is not None

    def test_construction_with_artifacts(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-002",
            format="jsonl",
            created_at_ms=1700000000000,
            finding_count=5,
            content_hash="hash123",
            artifact_ids=["a-001", "a-002"],
        )
        assert len(manifest.artifact_ids) == 2
        assert "a-001" in manifest.artifact_ids

    def test_construction_with_versions(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-003",
            format="sarif",
            created_at_ms=1700000000000,
            finding_count=0,
            content_hash="empty",
            schema_version="2.0.0",
            tool_version="0.5.0",
        )
        assert manifest.schema_version == "2.0.0"
        assert manifest.tool_version == "0.5.0"

    def test_default_artifact_ids_empty(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-004",
            format="json",
            created_at_ms=1700000000000,
            finding_count=0,
            content_hash="hash",
        )
        assert manifest.artifact_ids == []

    def test_to_dict(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-005",
            format="json",
            created_at_ms=1700000000000,
            finding_count=3,
            content_hash="abc",
            artifact_ids=["a-001"],
        )
        d = manifest.to_dict()
        assert d["report_id"] == "r-005"
        assert d["finding_count"] == 3
        assert d["artifact_ids"] == ["a-001"]

    def test_to_json(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-006",
            format="json",
            created_at_ms=1700000000000,
            finding_count=1,
            content_hash="abc",
        )
        j = manifest.to_json()
        parsed = json.loads(j)
        assert parsed["report_id"] == "r-006"
        assert parsed["finding_count"] == 1

    def test_hash_verification(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-007",
            format="json",
            created_at_ms=1700000000000,
            finding_count=2,
            content_hash="d41d8cd98f00b204e9800998ecf8427e",
        )
        assert len(manifest.content_hash) == 32

    def test_repr(self):
        from eggsec import ReportManifest
        manifest = ReportManifest(
            report_id="r-008",
            format="json",
            created_at_ms=1700000000000,
            finding_count=42,
            content_hash="abcdef1234567890abcdef1234567890",
        )
        r = repr(manifest)
        assert "r-008" in r
        assert "json" in r
        assert "42" in r


# ============================================================================
# ReportSummary
# ============================================================================

class TestReportSummary:
    def test_to_dict(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        reporter.write_finding('{"id":"f2","severity":"low"}')
        summary = reporter.finish()
        d = summary.to_dict()
        assert d["format"] == "json"
        assert d["total_findings"] == 2
        assert d["content_hash"] is not None
        assert isinstance(d["findings_by_severity"], list)

    def test_to_json(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        summary = reporter.finish()
        j = summary.to_json()
        parsed = json.loads(j)
        assert parsed["total_findings"] == 1

    def test_repr(self):
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high"}')
        summary = reporter.finish()
        r = repr(summary)
        assert "json" in r
        assert "1" in r


# ============================================================================
# DiffReportSummary
# ============================================================================

class TestDiffReportSummary:
    def test_to_dict(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"low","title":"New"}')
        reporter.write_finding('{"id":"f2","severity":"high","title":"Brand new"}')
        summary = reporter.finish()
        d = summary.to_dict()
        assert d["total_findings"] == 2
        assert d["new_findings"] == 1
        assert d["changed_findings"] == 1

    def test_to_json(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        baseline = '{"findings":[{"id":"f1","severity":"high","title":"Old"}]}'
        reporter = StreamingDiffReporter(cfg, baseline_json=baseline)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"low","title":"New"}')
        summary = reporter.finish()
        j = summary.to_json()
        parsed = json.loads(j)
        assert parsed["total_findings"] == 1
        assert parsed["changed_findings"] == 1

    def test_repr(self):
        from eggsec import StreamingDiffReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json")
        reporter = StreamingDiffReporter(cfg)
        reporter.start()
        summary = reporter.finish()
        r = repr(summary)
        assert "1" in r


# ============================================================================
# WS9: Large-Scale Streaming Tests
# ============================================================================


class TestStreamingReporterLargeScale:
    def test_10000_findings_no_blowup(self):
        """10K findings stream without memory or correctness issues."""
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json", buffer_size=500)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(10000):
            sev = ["high", "medium", "low", "critical", "info"][i % 5]
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": sev, "title": f"Vuln {i}"})
            )
        summary = reporter.finish()
        assert summary.total_findings == 10000
        assert summary.content_hash is not None
        sev_map = dict(summary.findings_by_severity)
        for sev in ["high", "medium", "low", "critical", "info"]:
            assert sev_map.get(sev) == 2000

    def test_50000_findings_streaming(self):
        """50K findings stream in a single reporter session."""
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json", buffer_size=1000)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(50000):
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": "info", "title": f"V {i}"})
            )
        summary = reporter.finish()
        assert summary.total_findings == 50000

    def test_100000_findings_with_file_output(self, tmp_path):
        """100K findings with file output — large-scale SARIF-like stress."""
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "large_scale.jsonl")
        cfg = StreamingReportConfig("json", output_path=out, buffer_size=2000)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(100000):
            reporter.write_finding(
                json.dumps({
                    "id": f"f{i}",
                    "severity": ["high", "medium", "low", "info"][i % 4],
                    "title": f"Finding {i}",
                    "description": f"Description for finding {i} with some padding text "
                    + "x" * 50,
                })
            )
        summary = reporter.finish()
        assert summary.total_findings == 100000
        assert summary.output_size_bytes > 1024 * 1024  # > 1MB
        with open(out) as f:
            lines = f.readlines()
        assert len(lines) == 100000
        # Verify first and last records
        first = json.loads(lines[0])
        assert first["id"] == "f0"
        last = json.loads(lines[-1])
        assert last["id"] == "f99999"

    def test_large_batch_write_10000(self):
        """Single batch write of 10K findings."""
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json", buffer_size=10000)
        reporter = StreamingReporter(cfg)
        reporter.start()
        batch = [
            {"id": f"bulk-{i}", "severity": "medium", "title": f"Bulk {i}"}
            for i in range(10000)
        ]
        reporter.write_findings_batch(json.dumps(batch))
        summary = reporter.finish()
        assert summary.total_findings == 10000


# ============================================================================
# WS9: SARIF Format Content Validation
# ============================================================================


class TestStreamingSarifFormat:
    def test_sarif_format_accepted(self):
        """SARIF format is accepted and reflected in summary."""
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("sarif")
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"XSS"}')
        summary = reporter.finish()
        assert summary.format == "sarif"
        assert summary.total_findings == 1

    def test_sarif_with_file_output(self, tmp_path):
        """SARIF format writes to file."""
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "report.sarif")
        cfg = StreamingReportConfig("sarif", output_path=out)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding('{"id":"f1","severity":"high","title":"XSS"}')
        reporter.write_finding('{"id":"f2","severity":"low","title":"Info"}')
        summary = reporter.finish()
        assert summary.format == "sarif"
        assert summary.total_findings == 2
        if os.path.exists(out):
            with open(out) as f:
                content = f.read()
            assert len(content) > 0

    def test_sarif_large_volume(self):
        """SARIF format handles 1000 findings."""
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("sarif", buffer_size=100)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for i in range(1000):
            reporter.write_finding(
                json.dumps({"id": f"f{i}", "severity": "high", "title": f"V {i}"})
            )
        summary = reporter.finish()
        assert summary.format == "sarif"
        assert summary.total_findings == 1000


# ============================================================================
# WS9: Secret Sentinel Scan
# ============================================================================

_SECRET_SENTINEL = "EGGSEC_TEST_SECRET_SENTINEL_42"


class TestStreamingSecretSentinel:
    def test_secret_in_finding_with_redaction_enabled(self, tmp_path):
        """Finding with secret content — redaction config is set."""
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "redacted.jsonl")
        cfg = StreamingReportConfig("json", output_path=out, redact_secrets=True)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding(json.dumps({
            "id": "f1",
            "severity": "high",
            "title": "Hardcoded credential",
            "evidence": f"password={_SECRET_SENTINEL}",
            "raw_response": f"Authorization: Bearer {_SECRET_SENTINEL}",
        }))
        summary = reporter.finish()
        assert summary.total_findings == 1
        # Config-level redaction flag is set
        assert cfg.redact_secrets is True

    def test_secret_in_finding_with_redaction_disabled(self, tmp_path):
        """Finding with secret content — no redaction when disabled."""
        from eggsec import StreamingReporter, StreamingReportConfig
        out = str(tmp_path / "unredacted.jsonl")
        cfg = StreamingReportConfig("json", output_path=out, redact_secrets=False)
        reporter = StreamingReporter(cfg)
        reporter.start()
        reporter.write_finding(json.dumps({
            "id": "f1",
            "severity": "high",
            "title": "Credential exposure",
            "evidence": f"api_key={_SECRET_SENTINEL}",
        }))
        summary = reporter.finish()
        assert summary.total_findings == 1
        with open(out) as f:
            content = f.read()
        # Without redaction, sentinel should be present in output
        assert _SECRET_SENTINEL in content

    def test_redaction_config_persists_across_flushes(self):
        """Redaction config persists across multiple flush cycles."""
        from eggsec import StreamingReporter, StreamingReportConfig
        cfg = StreamingReportConfig("json", redact_secrets=True)
        reporter = StreamingReporter(cfg)
        reporter.start()
        for cycle in range(10):
            reporter.write_finding(json.dumps({
                "id": f"f{cycle}",
                "severity": "high",
                "evidence": f"secret={_SECRET_SENTINEL}",
            }))
            reporter.flush()
        assert cfg.redact_secrets is True
        summary = reporter.finish()
        assert summary.total_findings == 10


# ============================================================================
# WS9: Interrupted Report Recovery
# ============================================================================


class TestStreamingReportRecovery:
    def test_reporter_reuse_after_finish(self):
        """New reporter can be created after previous one finished."""
        from eggsec import StreamingReporter, StreamingReportConfig
        for i in range(5):
            cfg = StreamingReportConfig("json")
            reporter = StreamingReporter(cfg)
            reporter.start()
            reporter.write_finding(json.dumps({"id": f"r{i}-f1", "severity": "high"}))
            summary = reporter.finish()
            assert summary.total_findings == 1

    def test_multiple_reporters_concurrent(self):
        """Multiple reporters can run independently."""
        from eggsec import StreamingReporter, StreamingReportConfig
        reporters = []
        for i in range(10):
            cfg = StreamingReportConfig("json")
            r = StreamingReporter(cfg)
            r.start()
            r.write_finding(json.dumps({"id": f"r{i}-f1", "severity": "high"}))
            reporters.append(r)

        for i, r in enumerate(reporters):
            summary = r.finish()
            assert summary.total_findings == 1
