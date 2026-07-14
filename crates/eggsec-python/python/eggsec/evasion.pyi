# Feature-gated: evasion detection
from typing import Any

class EvasionTargetTypePy:
    pass

class EvasionCategoryPy:
    pass

class EvasionRiskPy:
    pass

class EvasionTechniquePy:
    @property
    def name(self) -> str: ...
    @property
    def description(self) -> str: ...

class EvasionDetectionPy:
    @property
    def technique(self) -> EvasionTechniquePy: ...
    @property
    def detected(self) -> bool: ...

class EvasionSummaryPy:
    @property
    def total_techniques(self) -> int: ...
    @property
    def detected_count(self) -> int: ...

class EvasionReportPy:
    @property
    def summary(self) -> EvasionSummaryPy: ...
    @property
    def detections(self) -> list[EvasionDetectionPy]: ...

class EvasionScanConfigPy:
    pass

def evasion_scan(config: EvasionScanConfigPy) -> EvasionReportPy: ...
async def async_evasion_scan(config: EvasionScanConfigPy) -> EvasionReportPy: ...
def evasion_list_techniques() -> list[EvasionTechniquePy]: ...
