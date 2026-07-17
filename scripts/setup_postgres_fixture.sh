#!/usr/bin/env bash
# setup_postgres_fixture.sh — Start PostgreSQL fixture via Docker Compose
#
# Usage:
#   bash scripts/setup_postgres_fixture.sh
#
# Starts a PostgreSQL container on port 15432 and waits for readiness.
# Sets PGHOST=127.0.0.1 PGPORT=15432 PGUSER=postgres PGPASSWORD=postgres.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose-db-fixtures.yml"

echo "Starting PostgreSQL fixture..."
docker compose -f "$COMPOSE_FILE" up -d postgres

echo "Waiting for PostgreSQL readiness..."
for i in $(seq 1 30); do
  if docker compose -f "$COMPOSE_FILE" exec -T postgres pg_isready -U postgres >/dev/null 2>&1; then
    echo "PostgreSQL ready on port 15432"
    export PGHOST=127.0.0.1
    export PGPORT=15432
    export PGUSER=postgres
    export PGPASSWORD=postgres
    exit 0
  fi
  sleep 1
done

echo "ERROR: PostgreSQL did not become ready within 30s" >&2
exit 1
