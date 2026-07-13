from __future__ import annotations

from typing import Optional, List
from .pipeline import StepResult

class Checkpoint:
    def __init__(
        self,
        id: str,
        pipeline_name: str,
        *,
        completed_steps: Optional[List[str]] = None,
        results: Optional[List[StepResult]] = None,
        created_at_ms: int = 0,
    ) -> None: ...
    @property
    def id(self) -> str: ...
    @property
    def pipeline_name(self) -> str: ...
    @property
    def completed_steps(self) -> List[str]: ...
    @property
    def results(self) -> List[StepResult]: ...
    @property
    def created_at_ms(self) -> int: ...
    def to_dict(self) -> dict: ...
    def to_json(self) -> str: ...
    def __repr__(self) -> str: ...

class CheckpointStore:
    def __init__(self) -> None: ...
    def save(self, checkpoint: PipelineCheckpoint) -> None: ...
    def load(self, pipeline_id: str) -> Optional[CheckpointLoadResult]: ...
    def resume_from(
        self,
        pipeline_id: str,
        all_steps: List[str],
    ) -> Optional[tuple[CheckpointLoadResult, Optional[str]]]: ...
    def delete(self, pipeline_id: str) -> bool: ...
    def list_pipeline_ids(self) -> List[str]: ...
    def len(self) -> int: ...
    def is_empty(self) -> bool: ...
    def __repr__(self) -> str: ...

class PipelineCheckpoint:
    version: int
    pipeline_id: str
    pipeline_name: str
    completed_steps: List[str]
    current_step: Optional[str]
    step_results: dict
    created_at_ms: int
    updated_at_ms: int
    operation_schema_version: str
    target_set_hash: str
    scope_hash: str
    execution_profile: str
    enabled_features_hash: str
    pipeline_definition_hash: str
    artifact_store_id: Optional[str]
    def __init__(
        self,
        pipeline_id: str,
        pipeline_name: str,
        *,
        completed_steps: Optional[List[str]] = ...,
        current_step: Optional[str] = ...,
        step_results: Optional[dict] = ...,
        created_at_ms: int = ...,
        updated_at_ms: int = ...,
        operation_schema_version: Optional[str] = ...,
        target_set_hash: Optional[str] = ...,
        scope_hash: Optional[str] = ...,
        execution_profile: Optional[str] = ...,
        enabled_features_hash: Optional[str] = ...,
        pipeline_definition_hash: Optional[str] = ...,
        artifact_store_id: Optional[str] = ...,
    ) -> None: ...
    def is_current_version(self) -> bool: ...
    def next_step(self, all_steps: List[str]) -> Optional[str]: ...
    def to_dict(self) -> dict: ...
    def to_json(self) -> str: ...

class CheckpointLoadResult:
    checkpoint: PipelineCheckpoint
    migrated: bool
    original_version: int
    def to_dict(self) -> dict: ...

def create_checkpoint_store(path: Optional[str] = ...) -> CheckpointStore: ...
