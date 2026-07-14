# Feature-gated: C2 simulation
from typing import Any

class BeaconProtocolPy:
    pass

class TaskTypePy:
    pass

class TaskStatusPy:
    pass

class OpsecCategoryPy:
    pass

class OpsecSeverityPy:
    pass

class CampaignPhasePy:
    pass

class C2CampaignPy:
    @property
    def name(self) -> str: ...

class BeaconResultPy:
    @property
    def success(self) -> bool: ...

class C2TaskResultPy:
    @property
    def task_type(self) -> TaskTypePy: ...

class OpsecFindingPy:
    @property
    def category(self) -> OpsecCategoryPy: ...

class OpsecAssessmentPy:
    @property
    def findings(self) -> list[OpsecFindingPy]: ...

class C2SummaryPy:
    @property
    def total_tasks(self) -> int: ...

class C2ReportPy:
    @property
    def summary(self) -> C2SummaryPy: ...

class C2ScanConfigPy:
    pass

def c2_scan(config: C2ScanConfigPy) -> C2ReportPy: ...
async def async_c2_scan(config: C2ScanConfigPy) -> C2ReportPy: ...
def c2_get_campaign(name: str) -> C2CampaignPy: ...
