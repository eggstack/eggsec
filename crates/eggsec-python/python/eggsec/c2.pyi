# Feature-gated: C2 simulation
from typing import Any

class BeaconProtocolPy:
    Http: "BeaconProtocolPy"
    Https: "BeaconProtocolPy"
    Dns: "BeaconProtocolPy"
    Tcp: "BeaconProtocolPy"
    Custom: "BeaconProtocolPy"
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...

class TaskTypePy:
    Recon: "TaskTypePy"
    Execute: "TaskTypePy"
    Exfil: "TaskTypePy"
    Persist: "TaskTypePy"
    Lateral: "TaskTypePy"
    Evade: "TaskTypePy"
    SelfDestruct: "TaskTypePy"
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...

class TaskStatusPy:
    Completed: "TaskStatusPy"
    Failed: "TaskStatusPy"
    Simulated: "TaskStatusPy"
    Denied: "TaskStatusPy"
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...

class OpsecCategoryPy:
    ParentSpoofing: "OpsecCategoryPy"
    Timestomping: "OpsecCategoryPy"
    LogTampering: "OpsecCategoryPy"
    ProcessMasquerading: "OpsecCategoryPy"
    BurnMechanism: "OpsecCategoryPy"
    DecoyActivity: "OpsecCategoryPy"
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...

class OpsecSeverityPy:
    Info: "OpsecSeverityPy"
    Low: "OpsecSeverityPy"
    Medium: "OpsecSeverityPy"
    High: "OpsecSeverityPy"
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...
    def __hash__(self) -> int: ...

class CampaignPhasePy:
    id: str
    name: str
    def __repr__(self) -> str: ...
    def __str__(self) -> str: ...

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
