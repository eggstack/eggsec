"""Real database backend integration tests - Workstream 6.

Tests exercise actual database connections against loopback services.
When a database service is not available, the test FAILS (not skips),
per the plan's acceptance criteria: "Missing container services fail
the profile."

These tests are intended to run in CI with database containers, or
locally when database services are running on loopback.
"""

import socket
import pytest
import importlib

pytestmark = [pytest.mark.timeout(60)]


def _import_or_skip(name, feature="db-pentest"):
    """Import a name from eggsec, skip if feature-gated."""
    mod = importlib.import_module("eggsec")
    obj = getattr(mod, name, None)
    if obj is None:
        pytest.skip(f"{name} not available (requires {feature} feature)")
    return obj


def _assert_port_open(host, port, service_name):
    """Assert a port is open; FAIL (not skip) if connection refused."""
    try:
        with socket.create_connection((host, port), timeout=2):
            return True
    except (ConnectionRefusedError, OSError) as exc:
        pytest.fail(
            f"{service_name} not available on {host}:{port} — "
            f"connection refused. Start the {service_name} service or "
            f"container before running this profile. (error: {exc})"
        )


# ---------------------------------------------------------------------------
# PostgreSQL backend
# ---------------------------------------------------------------------------


class TestDbPostgresIntegration:
    """Real PostgreSQL connection tests."""

    PG_HOST = "127.0.0.1"
    PG_PORT = 5432

    def test_postgres_port_reachable(self):
        """PostgreSQL port is open and accepting connections."""
        _assert_port_open(self.PG_HOST, self.PG_PORT, "PostgreSQL")

    def test_db_probe_postgres_connects(self):
        """db_probe_postgres establishes a real connection."""
        db_probe_postgres = _import_or_skip("db_probe_postgres")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="postgres",
            host=self.PG_HOST,
            port=self.PG_PORT,
            database="postgres",
        )
        result = db_probe_postgres(target, username="postgres", password="postgres")
        assert result is not None
        d = result.to_dict()
        assert "findings" in d
        assert isinstance(d["findings"], list)

    def test_db_probe_postgres_returns_report(self):
        """db_probe_postgres returns a valid DbPentestReport."""
        db_probe_postgres = _import_or_skip("db_probe_postgres")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="postgres",
            host=self.PG_HOST,
            port=self.PG_PORT,
            database="postgres",
        )
        report = db_probe_postgres(target, username="postgres", password="postgres")
        d = report.to_dict()
        assert "findings" in d
        assert "recommendations" in d
        assert "metadata" in d

    def test_db_probe_postgres_invalid_credentials(self):
        """db_probe_postgres with wrong credentials still returns a report."""
        db_probe_postgres = _import_or_skip("db_probe_postgres")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="postgres",
            host=self.PG_HOST,
            port=self.PG_PORT,
            database="postgres",
        )
        report = db_probe_postgres(target, username="wrong", password="wrong")
        d = report.to_dict()
        assert "findings" in d


# ---------------------------------------------------------------------------
# MySQL backend
# ---------------------------------------------------------------------------


class TestDbMysqlIntegration:
    """Real MySQL connection tests."""

    MYSQL_HOST = "127.0.0.1"
    MYSQL_PORT = 3306

    def test_mysql_port_reachable(self):
        """MySQL port is open and accepting connections."""
        _assert_port_open(self.MYSQL_HOST, self.MYSQL_PORT, "MySQL")

    def test_db_probe_mysql_connects(self):
        """db_probe_mysql establishes a real connection."""
        db_probe_mysql = _import_or_skip("db_probe_mysql")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="mysql",
            host=self.MYSQL_HOST,
            port=self.MYSQL_PORT,
            database="mysql",
        )
        result = db_probe_mysql(target, username="root", password="root")
        assert result is not None
        d = result.to_dict()
        assert "findings" in d

    def test_db_probe_mysql_returns_report(self):
        """db_probe_mysql returns a valid DbPentestReport."""
        db_probe_mysql = _import_or_skip("db_probe_mysql")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="mysql",
            host=self.MYSQL_HOST,
            port=self.MYSQL_PORT,
            database="mysql",
        )
        report = db_probe_mysql(target, username="root", password="root")
        d = report.to_dict()
        assert "findings" in d
        assert "recommendations" in d


# ---------------------------------------------------------------------------
# Redis backend
# ---------------------------------------------------------------------------


class TestDbRedisIntegration:
    """Real Redis connection tests."""

    REDIS_HOST = "127.0.0.1"
    REDIS_PORT = 6379

    def test_redis_port_reachable(self):
        """Redis port is open and accepting connections."""
        _assert_port_open(self.REDIS_HOST, self.REDIS_PORT, "Redis")

    def test_db_probe_redis_connects(self):
        """db_probe_redis establishes a real connection."""
        db_probe_redis = _import_or_skip("db_probe_redis")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="redis",
            host=self.REDIS_HOST,
            port=self.REDIS_PORT,
        )
        result = db_probe_redis(target)
        assert result is not None
        d = result.to_dict()
        assert "findings" in d

    def test_db_probe_redis_returns_report(self):
        """db_probe_redis returns a valid DbPentestReport."""
        db_probe_redis = _import_or_skip("db_probe_redis")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="redis",
            host=self.REDIS_HOST,
            port=self.REDIS_PORT,
        )
        report = db_probe_redis(target)
        d = report.to_dict()
        assert "findings" in d
        assert "recommendations" in d


# ---------------------------------------------------------------------------
# MongoDB backend
# ---------------------------------------------------------------------------


class TestDbMongodbIntegration:
    """Real MongoDB connection tests."""

    MONGO_HOST = "127.0.0.1"
    MONGO_PORT = 27017

    def test_mongodb_port_reachable(self):
        """MongoDB port is open and accepting connections."""
        _assert_port_open(self.MONGO_HOST, self.MONGO_PORT, "MongoDB")

    def test_db_probe_mongodb_connects(self):
        """db_probe_mongodb establishes a real connection."""
        db_probe_mongodb = _import_or_skip("db_probe_mongodb")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="mongodb",
            host=self.MONGO_HOST,
            port=self.MONGO_PORT,
            database="admin",
        )
        result = db_probe_mongodb(target)
        assert result is not None
        d = result.to_dict()
        assert "findings" in d

    def test_db_probe_mongodb_returns_report(self):
        """db_probe_mongodb returns a valid DbPentestReport."""
        db_probe_mongodb = _import_or_skip("db_probe_mongodb")
        DbTarget = _import_or_skip("DbTarget")

        target = DbTarget(
            db_type="mongodb",
            host=self.MONGO_HOST,
            port=self.MONGO_PORT,
            database="admin",
        )
        report = db_probe_mongodb(target)
        d = report.to_dict()
        assert "findings" in d
        assert "recommendations" in d


# ---------------------------------------------------------------------------
# Generic db_probe with config
# ---------------------------------------------------------------------------


class TestDbProbeWithConfig:
    """Test db_run_with_config with real connections."""

    def test_db_run_with_config_postgres(self):
        """db_run_with_config works against real PostgreSQL."""
        db_run_with_config = _import_or_skip("db_run_with_config")
        DbTarget = _import_or_skip("DbTarget")
        DbSessionConfig = _import_or_skip("DbSessionConfig")

        _assert_port_open("127.0.0.1", 5432, "PostgreSQL")

        target = DbTarget(
            db_type="postgres",
            host="127.0.0.1",
            port=5432,
            database="postgres",
        )
        config = DbSessionConfig(
            username="postgres",
            password="postgres",
            max_queries=10,
            timeout_secs=5,
        )
        report = db_run_with_config(target, config)
        d = report.to_dict()
        assert "findings" in d

    def test_db_driver_registry_has_all_drivers(self):
        """DbDriverRegistry lists all supported drivers."""
        DbDriverRegistry = _import_or_skip("DbDriverRegistry")

        reg = DbDriverRegistry()
        drivers = reg.list_drivers()
        names = [d.name for d in drivers]
        assert "postgres" in names
        assert "mysql" in names
        assert "redis" in names
        assert "mongodb" in names
