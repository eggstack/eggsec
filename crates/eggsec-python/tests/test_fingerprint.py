"""Tests for service fingerprinting bindings."""

import pytest
import eggsec


def test_fingerprint_result_has_methods():
    """Test that FingerprintScanResult has the expected methods."""
    assert hasattr(eggsec.FingerprintScanResult, "to_dict")
    assert hasattr(eggsec.FingerprintScanResult, "to_json")


def test_service_fingerprint_result_has_methods():
    """Test that ServiceFingerprintResult has the expected methods."""
    assert hasattr(eggsec.ServiceFingerprintResult, "to_dict")
    assert hasattr(eggsec.ServiceFingerprintResult, "to_json")


def test_client_fingerprint_services():
    """Test that Client.fingerprint_services exists."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.Client(scope, mode="manual")
    assert hasattr(client, "fingerprint_services")


def test_async_client_fingerprint_services():
    """Test that AsyncClient.fingerprint_services exists."""
    scope = eggsec.Scope.allow_hosts(["example.com"])
    client = eggsec.AsyncClient(scope, mode="manual")
    assert hasattr(client, "fingerprint_services")


def test_convenience_fingerprint_services():
    """Test that fingerprint_services convenience function exists."""
    assert callable(eggsec.fingerprint_services)


def test_convenience_async_fingerprint_services():
    """Test that async_fingerprint_services convenience function exists."""
    assert callable(eggsec.async_fingerprint_services)


def test_fingerprint_evidence_exists():
    """Test that FingerprintEvidence class exists."""
    assert hasattr(eggsec, "FingerprintEvidence")


def test_fingerprint_confidence_exists():
    """Test that FingerprintConfidence class exists."""
    assert hasattr(eggsec, "FingerprintConfidence")
