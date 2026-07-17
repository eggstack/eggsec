#!/usr/bin/env bash
# setup_mysql_fixture.sh — Start MySQL fixture via Docker Compose
#
# Usage:
#   bash scripts/setup_mysql_fixture.sh
#
# Starts a MySQL container on port 13306 and waits for readiness.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose-db-fixtures.yml"

echo "Starting MySQL fixture..."
docker compose -f "$COMPOSE_FILE" up -d mysql

echo "Waiting for MySQL readiness..."
for i in $(seq 1 60); do
  if docker compose -f "$COMPOSE_FILE" exec -T mysql mysqladmin ping -h 127.0.0.1 -uroot -proot --silent >/dev/null 2>&1; then
    echo "MySQL ready on port 13306"
    exit 0
  fi
  sleep 1
done

echo "ERROR: MySQL did not become ready within 60s" >&2
exit 1
