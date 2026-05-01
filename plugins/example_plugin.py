"""
Example Slapper Plugin

This plugin demonstrates how to create custom security checks
that extend Slapper's capabilities.

To use: Place in ~/.slapper/plugins/ or ./plugins/
"""

def register_checks():
    """
    Register custom checks with Slapper.
    Returns a list of check definitions.
    """
    return [
        {
            "name": "check_s3_bucket",
            "type": "endpoint",
            "target": "aws",
            "description": "Check for exposed S3 bucket endpoints"
        },
        {
            "name": "check_debug_endpoints",
            "type": "endpoint",
            "target": "any",
            "description": "Check for debug endpoints that may expose sensitive info"
        },
        {
            "name": "check_api_versioning",
            "type": "endpoint",
            "target": "any",
            "description": "Enumerate API versions"
        }
    ]

def run_check(check_name: str, target: str) -> list:
    """
    Execute a custom check.
    
    Args:
        check_name: Name of the check to run
        target: Target URL or host
        
    Returns:
        List of JSON-serializable result objects
    """
    import json
    
    results = []
    
    if check_name == "check_s3_bucket":
        s3_paths = [
            "/.s3",
            "/s3",
            "/bucket",
            "/storage",
            "/files",
        ]
        
        for path in s3_paths:
            results.append(json.dumps({
                "type": "endpoint",
                "path": path,
                "check": "s3_bucket",
                "target": target,
                "severity": "medium",
                "description": f"Potential S3 bucket endpoint: {path}"
            }))
    
    elif check_name == "check_debug_endpoints":
        debug_paths = [
            "/debug",
            "/debug/pprof",
            "/debug/vars",
            "/debug/metrics",
            "/__debug__",
            "/.debug",
        ]
        
        for path in debug_paths:
            results.append(json.dumps({
                "type": "endpoint",
                "path": path,
                "check": "debug_endpoints",
                "target": target,
                "severity": "high",
                "description": f"Debug endpoint: {path}"
            }))
    
    elif check_name == "check_api_versioning":
        api_versions = ["v1", "v2", "v3", "v4", "beta", "alpha", "internal"]
        
        for version in api_versions:
            results.append(json.dumps({
                "type": "endpoint",
                "path": f"/api/{version}",
                "check": "api_versioning",
                "target": target,
                "severity": "low",
                "description": f"API version endpoint: /api/{version}"
            }))
    
    return results


def on_result(result_type: str, result_data: dict):
    """
    Callback when Slapper produces a result.
    Can be used for custom processing or logging.
    
    Args:
        result_type: Type of result (load_test, port_scan, endpoint_scan, fingerprint)
        result_data: The result data as a dictionary
    """
    pass


def custom_request(url: str, method: str = "GET", headers: dict = None, body: str = None) -> dict:
    """
    Make a custom HTTP request.
    
    Args:
        url: Target URL
        method: HTTP method
        headers: Request headers
        body: Request body
        
    Returns:
        Response data as dictionary
    """
    import urllib.request
    import json
    
    headers = headers or {}
    
    req = urllib.request.Request(url, method=method)
    for key, value in headers.items():
        req.add_header(key, value)
    
    try:
        with urllib.request.urlopen(req, timeout=10) as response:
            return {
                "status": response.status,
                "headers": dict(response.headers),
                "body": response.read().decode('utf-8', errors='ignore')
            }
    except Exception as e:
        return {
            "error": str(e)
        }
