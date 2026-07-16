"""Storage and repository types for eggsec.

This submodule contains finding/assessment repositories and artifact stores.

Maturity: provisional (Release 4 storage contracts)
"""

try:
    from .._core import (
        # Finding workflow
        FindingState,
        WorkflowTransition,
        Suppression,
        FindingWorkflow,
        # Repository abstraction
        FindingRepository,
        Assessment,
        AssessmentRepository,
        # Baselines and comparisons
        FindingCorrelation,
        FindingDiff,
        AssessmentDiff,
        BaselineComparator,
        # SQLite repository
        SqliteFindingRepository,
        SqliteAssessmentRepository,
        SqliteMigration,
        SqliteMigrationResult,
        # JSONL repository
        JsonlFindingRepository,
        JsonlAssessmentRepository,
        # Content-addressed artifact store
        ContentAddressedArtifactStore,
        DirectoryArtifactStore,
        ArtifactInfo,
        ArtifactData,
        IntegrityResult,
        ArtifactQuery,
    )
except (AttributeError, ImportError):
    pass
