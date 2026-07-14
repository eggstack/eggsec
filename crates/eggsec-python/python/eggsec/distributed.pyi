# Feature-gated: distributed computing
from typing import Any

class DistributedTaskTypePy:
    pass

class WorkerStatusPy:
    pass

class WorkerRegistrationPy:
    @property
    def worker_id(self) -> str: ...

class HeartbeatPy:
    @property
    def worker_id(self) -> str: ...

class DistributedTaskPy:
    @property
    def task_id(self) -> str: ...

class DistributedTaskResultPy:
    @property
    def task_id(self) -> str: ...
    @property
    def success(self) -> bool: ...

def distributed_task_types() -> list[DistributedTaskTypePy]: ...
def distributed_generate_psk() -> str: ...
