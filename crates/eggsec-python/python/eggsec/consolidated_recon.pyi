"""Type stubs for consolidated reconnaissance module."""

from typing import List, Optional

class ConsolidatedReconConfig:
    """Configuration for consolidated reconnaissance."""
    run_dns: bool
    run_ssl: bool
    run_tech_detect: bool
    run_subdomain: bool
    run_whois: bool
    run_cors: bool
    run_wayback: bool
    run_js: bool
    run_content: bool
    run_email: bool
    def __init__(
        self,
        *,
        run_dns: bool = True,
        run_ssl: bool = True,
        run_tech_detect: bool = True,
        run_subdomain: bool = True,
        run_whois: bool = True,
        run_cors: bool = True,
        run_wayback: bool = True,
        run_js: bool = True,
        run_content: bool = True,
        run_email: bool = True,
    ) -> None: ...

class ReconModuleResult:
    """Result from a single recon module."""
    module: str
    success: bool
    data: Optional[str]
    error: Optional[str]

class ConsolidatedReconReport:
    """Complete consolidated reconnaissance report."""
    target: str
    modules: List[ReconModuleResult]
    modules_run: int
    modules_succeeded: int
    modules_failed: int

def run_consolidated_recon(
    target: str, config: ConsolidatedReconConfig
) -> ConsolidatedReconReport:
    """Run consolidated reconnaissance against a target."""
    ...

async def async_run_consolidated_recon(
    target: str, config: ConsolidatedReconConfig
) -> ConsolidatedReconReport:
    """Run consolidated reconnaissance against a target (async)."""
    ...
