#!/usr/bin/env bash
# chat.sh — Simple interactive chat with a running llama.cpp server.
#
# Usage: ./chat.sh [--port 8080] [--no-think]
#
# Requires: curl, jq

set -euo pipefail

PORT="${LLAMA_PORT:-8080}"
HOST="${LLAMA_HOST:-localhost}"
NO_THINK=false

while [[ $# -gt 0 ]]; do
    case "$1" in
        --port) PORT="$2"; shift 2 ;;
        --host) HOST="$2"; shift 2 ;;
        --no-think) NO_THINK=true; shift ;;
        *) echo "Unknown: $1"; exit 1 ;;
    esac
done

BASE_URL="http://${HOST}:${PORT}"

# Check server
if ! curl -sf "${BASE_URL}/v1/models" >/dev/null 2>&1; then
    echo "❌ Server not responding at ${BASE_URL}"
    echo "   Start with: ./launch.sh <profile>"
    exit 1
fi

MODEL=$(curl -sf "${BASE_URL}/v1/models" | jq -r '.data[0].id // "unknown"')
echo "=== llama.cpp Chat ==="
echo "Model: ${MODEL} @ ${BASE_URL}"
echo "Type 'quit' or Ctrl-C to exit."
echo ""

MESSAGES="[]"

while true; do
    printf "\033[1;32mYou:\033[0m "
    read -r INPUT || break
    [[ "$INPUT" == "quit" || "$INPUT" == "exit" ]] && break
    [[ -z "$INPUT" ]] && continue

    MESSAGES=$(echo "$MESSAGES" | jq --arg content "$INPUT" '. + [{"role":"user","content":$content}]')

    PAYLOAD=$(jq -n \
        --argjson messages "$MESSAGES" \
        --argjson stream false \
        '{model: "default", messages: $messages, stream: $stream, max_tokens: 4096, temperature: 0.6}')

    printf "\033[1;34mAssistant:\033[0m "
    RESPONSE=$(curl -sf "${BASE_URL}/v1/chat/completions" \
        -H "Content-Type: application/json" \
        -d "$PAYLOAD")

    CONTENT=$(echo "$RESPONSE" | jq -r '.choices[0].message.content // "Error: no response"')
    echo "$CONTENT"
    echo ""

    MESSAGES=$(echo "$MESSAGES" | jq --arg content "$CONTENT" '. + [{"role":"assistant","content":$content}]')
done

echo "Bye."
