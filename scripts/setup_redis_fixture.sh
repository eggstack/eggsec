#!/usr/bin/env bash
# setup_redis_fixture.sh — Start Redis fixture via Docker Compose
#
# Usage:
#   bash scripts/setup_redis_fixture.sh
#
# Starts a Redis container on port 16379 and waits for readiness.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose-db-fixtures.yml"

echo "Starting Redis fixture..."
docker compose -f "$COMPOSE_FILE" up -d redis

echo "Waiting for Redis readiness..."
for i in $(seq 1 15); do
  if docker compose -f "$COMPOSE_FILE" exec -T redis redis-cli ping 2>/dev/null | grep -q PONG; then
    echo "Redis ready on port 16379"
    exit 0
  fi
  sleep 1
done

echo "ERROR: Redis did not become ready within 15s" >&2
exit 1
