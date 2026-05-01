# Name: test_plugin
# Version: 0.1.0
# Description: Test plugin for unit tests
# Author: Test

def register_checks():
    return [
        {"name": "test_check", "type": "scan", "description": "A test check"},
    ]

def run_check(check_name, target):
    if check_name == "test_check":
        return ['{"title":"Test finding","severity":"low","description":"Found on ' + target + '","location":"' + target + '"}']
    return []
