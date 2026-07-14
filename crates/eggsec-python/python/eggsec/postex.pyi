# Feature-gated: post-exploitation simulation
from typing import Any

class PostexCategoryPy:
    pass

class PostexRiskPy:
    pass

class PostexProfilePy:
    pass

class PostexTechniquePy:
    @property
    def name(self) -> str: ...
    @property
    def description(self) -> str: ...

class PostexDetectionPy:
    @property
    def technique(self) -> PostexTechniquePy: ...
    @property
    def detected(self) -> bool: ...

class PostexSummaryPy:
    @property
    def total_techniques(self) -> int: ...
    @property
    def detected_count(self) -> int: ...

class PostexReportPy:
    @property
    def summary(self) -> PostexSummaryPy: ...
    @property
    def detections(self) -> list[PostexDetectionPy]: ...

class PostexScanConfigPy:
    pass

def postex_scan(config: PostexScanConfigPy) -> PostexReportPy: ...
async def async_postex_scan(config: PostexScanConfigPy) -> PostexReportPy: ...
def postex_list_techniques() -> list[PostexTechniquePy]: ...
