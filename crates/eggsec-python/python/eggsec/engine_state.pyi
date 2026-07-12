class DispatchAuditEvent:
    event_id: str
    timestamp_ms: int
    operation_id: str
    target: str
    surface: str
    outcome: str
    allowed: bool
    decision_summary: str
    redacted: bool
    def to_dict(self) -> dict[str, object]: ...
    def to_json(self) -> str: ...
