"""Reporting and output types for eggsec.

This submodule contains reporters, streaming output, baselines, and formats.

Maturity: provisional (Release 4 reporting)
"""

try:
    from .._core import (
        FindingReporter,
        SeveritySummary,
        ReportEnvelope,
        StreamingReportConfig,
        StreamingReporter,
        ReportSummary,
        StreamingDiffReporter,
        FindingDiffResult,
        DiffReportSummary,
        ReportManifest,
    )
except (AttributeError, ImportError):
    pass

# Keep export list truthful for feature-gated builds
__all__ = [name for name in dir() if not name.startswith("_")]
