#!/usr/bin/env bash
# Restart the measured recommendation with bounded reasoning; no model download.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
case "${1:-}" in
  --stop|stop|--status|status)
    exec "$SCRIPT_DIR/launch.sh" "$@"
    ;;
  --dry-run)
    shift
    exec "$SCRIPT_DIR/launch.sh" --dry-run qwythos-9b-q5 --reasoning-budget 512 --jinja "$@"
    ;;
  *)
    exec "$SCRIPT_DIR/launch.sh" qwythos-9b-q5 --reasoning-budget 512 --jinja "$@"
    ;;
esac
