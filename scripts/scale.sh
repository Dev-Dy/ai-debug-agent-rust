#!/bin/bash
set -euo pipefail

COUNT=$1

if ! [[ "$COUNT" =~ ^[0-9]+$ ]] || [[ "$COUNT" -lt 1 ]]; then
  echo "Usage: ./scripts/scale.sh <positive-worker-count>" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

docker compose up -d --scale worker="$COUNT" --no-recreate

echo "Scaled workers to $COUNT"
