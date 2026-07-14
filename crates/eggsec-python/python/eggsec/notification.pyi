# Feature-gated: notification system
from typing import Any

class WebhookEventPy:
    pass

class FindingSummaryPy:
    @property
    def severity(self) -> str: ...

class NotifyScanStatsPy:
    @property
    def total_findings(self) -> int: ...

class WebhookConfigPy:
    pass

class NotifyManagerPy:
    pass

def notify_scan_started(scan_id: str) -> None: ...
def notify_scan_complete(scan_id: str, stats: NotifyScanStatsPy) -> None: ...
def notify_findings(findings: list[FindingSummaryPy]) -> None: ...
def notify_error(scan_id: str, error: str) -> None: ...
