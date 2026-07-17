#!/usr/bin/env bash
# setup_mongodb_fixture.sh — Start MongoDB fixture via Docker Compose
#
# Usage:
#   bash scripts/setup_mongodb_fixture.sh
#
# Starts a MongoDB container on port 127017 and waits for readiness.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
COMPOSE_FILE="$SCRIPT_DIR/docker-compose-db-fixtures.yml"

echo "Starting MongoDB fixture..."
docker compose -f "$COMPOSE_FILE" up -d mongodb

echo "Waiting for MongoDB readiness..."
for i in $(seq 1 30); do
  if docker compose -f "$COMPOSE_FILE" exec -T mongodb mongosh --eval 'db.runCommand({ping:1})' --quiet >/dev/null 2>&1; then
    echo "MongoDB ready on port 127017"
    exit 0
  fi
  sleep 1
done

echo "ERROR: MongoDB did not become ready within 30s" >&2
exit 1
