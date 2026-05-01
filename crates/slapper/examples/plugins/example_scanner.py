"""
Slapper Python Plugin Example

This example demonstrates how to create custom security scanning plugins
for Slapper using Python.

To use this plugin:
1. Enable Python plugin support: cargo build --features python-plugins
2. Copy this file to ~/.config/slapper/plugins/
3. Run: ./slapper scan example.com --plugin example_scanner
"""

from typing import Dict, List, Optional, Any
from dataclasses import dataclass
import json

@dataclass
class Finding:
    """Represents a security finding discovered by the plugin."""
    severity: str  # critical, high, medium, low, info
    finding_type: str  # e.g., "sql_injection", "xss", "custom_vuln"
    description: str
    location: str
    evidence: Optional[str] = None
    references: Optional[List[str]] = None
    cvss_score: Optional[float] = None

@dataclass  
class ScanResult:
    """Result from a plugin scan."""
    target: str
    findings: List[Finding]
    metadata: Dict[str, Any]
    success: bool
    error: Optional[str] = None


class Plugin:
    """Base class for Slapper plugins."""
    
    @property
    def name(self) -> str:
        """Plugin name."""
        raise NotImplementedError
    
    @property
    def version(self) -> str:
        """Plugin version."""
        raise NotImplementedError
    
    def run(self, target: str, config: Dict[str, Any]) -> ScanResult:
        """
        Execute the plugin scan.
        
        Args:
            target: The target URL or hostname
            config: Plugin configuration from slapper.toml
            
        Returns:
            ScanResult with findings and metadata
        """
        raise NotImplementedError


class ExampleScanner(Plugin):
    """
    Example security scanner plugin.
    
    This plugin demonstrates how to:
    - Define configuration options
    - Make HTTP requests
    - Detect vulnerabilities
    - Report findings
    """
    
    @property
    def name(self) -> str:
        return "example_scanner"
    
    @property
    def version(self) -> str:
        return "1.0.0"
    
    def run(self, target: str, config: Dict[str, Any]) -> ScanResult:
        """Run the security scan."""
        findings = []
        
        # Example: Check for common security misconfigurations
        findings.extend(self._check_security_headers(target, config))
        findings.extend(self._check_exposed_endpoints(target, config))
        findings.extend(self._check_cookie_security(target, config))
        
        return ScanResult(
            target=target,
            findings=findings,
            metadata={
                "plugin": self.name,
                "version": self.version,
                "checks_performed": 3
            },
            success=True
        )
    
    def _check_security_headers(self, target: str, config: Dict[str, Any]) -> List[Finding]:
        """Check for missing security headers."""
        findings = []
        
        # In a real plugin, you would make actual HTTP requests
        # This is a simplified example
        
        required_headers = [
            ("X-Frame-Options", "medium"),
            ("X-Content-Type-Options", "medium"),
            ("Strict-Transport-Security", "high"),
            ("Content-Security-Policy", "high"),
            ("X-XSS-Protection", "low"),
        ]
        
        # Simulated check (replace with actual HTTP request)
        # response = self._http_get(target)
        # for header, severity in required_headers:
        #     if header not in response.headers:
        #         findings.append(Finding(...))
        
        return findings
    
    def _check_exposed_endpoints(self, target: str, config: Dict[str, Any]) -> List[Finding]:
        """Check for commonly exposed sensitive endpoints."""
        findings = []
        
        sensitive_paths = [
            "/.env",
            "/.git/config",
            "/config.php",
            "/backup.sql",
            "/admin",
            "/debug",
            "/phpinfo.php",
        ]
        
        # In a real plugin:
        # for path in sensitive_paths:
        #     response = self._http_get(f"{target}{path}")
        #     if response.status_code == 200:
        #         findings.append(Finding(...))
        
        return findings
    
    def _check_cookie_security(self, target: str, config: Dict[str, Any]) -> List[Finding]:
        """Check for insecure cookie configurations."""
        findings = []
        
        # Check for:
        # - Missing HttpOnly flag
        # - Missing Secure flag
        # - SameSite not set
        
        return findings


class SQLiScanner(Plugin):
    """
    Custom SQL Injection scanner plugin.
    
    Demonstrates a more specialized security plugin.
    """
    
    @property
    def name(self) -> str:
        return "sqli_scanner"
    
    @property
    def version(self) -> str:
        return "1.0.0"
    
    def run(self, target: str, config: Dict[str, Any]) -> ScanResult:
        findings = []
        
        # Load custom payloads from config
        payloads = config.get("payloads", self._default_payloads())
        
        # Get parameters to test
        params = config.get("parameters", ["id", "search", "user"])
        
        for param in params:
            for payload in payloads:
                result = self._test_payload(target, param, payload)
                if result:
                    findings.append(result)
        
        return ScanResult(
            target=target,
            findings=findings,
            metadata={"payloads_tested": len(payloads) * len(params)},
            success=True
        )
    
    def _default_payloads(self) -> List[str]:
        return [
            "' OR '1'='1",
            "' OR 1=1--",
            "\" OR \"\"=\"",
            "1' AND '1'='1",
            "1; DROP TABLE users--",
        ]
    
    def _test_payload(self, target: str, param: str, payload: str) -> Optional[Finding]:
        """Test a single payload and return a finding if vulnerable."""
        # Implementation would:
        # 1. Make request with payload
        # 2. Analyze response for SQL error messages
        # 3. Check for time-based delays
        # 4. Return Finding if vulnerable
        return None


# Plugin registration
# Slapper will automatically discover plugins that define PLUGINS
PLUGINS = [
    ExampleScanner,
    SQLiScanner,
]


# Optional: Plugin configuration schema
CONFIG_SCHEMA = {
    "example_scanner": {
        "check_headers": {"type": "bool", "default": True},
        "check_cookies": {"type": "bool", "default": True},
        "timeout": {"type": "int", "default": 30},
    },
    "sqli_scanner": {
        "payloads": {"type": "list", "default": None},
        "parameters": {"type": "list", "default": ["id"]},
        "blind_detection": {"type": "bool", "default": True},
    }
}
