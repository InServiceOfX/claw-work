#!/usr/bin/env bash
# launch.sh — Launch llama.cpp CUDA server in Docker.
#
# Usage:
#   ./launch.sh <profile>           # Start server with profile
#   ./launch.sh --dry-run <profile> # Print command without changing runtime state
#   ./launch.sh --stop              # Stop running container
#   ./launch.sh --status            # Show container status
#   ./launch.sh                     # List available profiles
#
# Docker settings: config.yml (copy from config.yml.example)
# Model settings:  profiles/<profile>.yml
#
# Full server docs: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

CONFIG_FILE="$SCRIPT_DIR/config.yml"
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "❌ config.yml not found. Run: cp config.yml.example config.yml"
    exit 1
fi

DRY_RUN=false

# ── handle --stop / --status with no profile ──────────────────────────────────
case "${1:-}" in
    --dry-run)
        DRY_RUN=true
        shift
        if [[ $# -eq 0 ]]; then
            echo "Usage: $0 --dry-run <profile> [extra llama-server args...]"
            exit 1
        fi
        ;;
    --stop|stop)
        NAME=$(python3 -c "
import yaml
cfg = yaml.safe_load(open('$CONFIG_FILE'))
print(cfg.get('docker',{}).get('container_name','llama-cpp-server'))
")
        docker stop "$NAME" 2>/dev/null && docker rm "$NAME" 2>/dev/null || true
        echo "🛑 Stopped $NAME"
        exit 0
        ;;
    --status|status)
        NAME=$(python3 -c "
import yaml
cfg = yaml.safe_load(open('$CONFIG_FILE'))
print(cfg.get('docker',{}).get('container_name','llama-cpp-server'))
")
        docker ps -a --filter "name=$NAME" --format "table {{.Names}}\t{{.Status}}\t{{.Ports}}"
        exit 0
        ;;
    "")
        echo "Usage: $0 <profile> [extra llama-server args...]"
        echo "       $0 --dry-run <profile> [extra llama-server args...]"
        echo "       $0 --stop | stop"
        echo "       $0 --status | status"
        echo ""
        echo "Available profiles:"
        ls "$SCRIPT_DIR/profiles/"*.yml 2>/dev/null \
            | xargs -I{} basename {} .yml \
            | sed 's/^/  /'
        exit 0
        ;;
esac

PROFILE="$1"; shift

PROFILE_FILE="$SCRIPT_DIR/profiles/${PROFILE}.yml"
if [[ ! -f "$PROFILE_FILE" ]]; then
    echo "❌ Profile not found: $PROFILE_FILE"
    echo "Available:"
    ls "$SCRIPT_DIR/profiles/"*.yml 2>/dev/null | xargs -I{} basename {} .yml | sed 's/^/  /'
    exit 1
fi

# ── build docker run + server args via Python (same pattern as TensorRTLLMFixed) ─
eval "$(python3 - "$CONFIG_FILE" "$PROFILE_FILE" <<'PYEOF'
import sys, yaml, shlex

cfg = yaml.safe_load(open(sys.argv[1])) or {}
prof = yaml.safe_load(open(sys.argv[2])) or {}

docker = cfg.get("docker", {})
image = docker.get("image", "ghcr.io/ggml-org/llama.cpp:server-cuda")
gpu_id = str(docker.get("gpu_id", 0))
name = docker.get("container_name", "llama-cpp-server")

# Volume mounts
vol_args = []
for v in docker.get("volumes", []):
    ro = ":ro" if v.get("readonly") else ""
    vol_args += ["-v", f"{v['host']}:{v['container']}{ro}"]

# Model path (inside container)
model_path = prof.get("model_path", "")
if not model_path:
    print('echo "❌ No model_path in profile"; exit 1')
    sys.exit(0)

# Server section
srv = prof.get("server", {})
host = srv.get("host", "0.0.0.0")
port = str(srv.get("port", 8080))

# Build llama-server args
server_args = ["-m", model_path, "--host", "0.0.0.0", "--port", port]

# Core params
def add(flag, key, src=prof):
    val = src.get(key)
    if val is not None and val != "":
        server_args.extend([flag, str(val)])

add("-ngl", "n_gpu_layers")
add("-c", "ctx_size")
add("-n", "n_predict")
add("-np", "parallel")
add("-b", "batch_size")
add("-ub", "ubatch_size")
add("-t", "threads")

# Default sampling parameters. API clients can override these per request.
add("--temp", "temperature")
add("--top-p", "top_p")
add("--top-k", "top_k")
add("--min-p", "min_p")
add("--repeat-penalty", "repeat_penalty")

# Flash attention
fa = prof.get("flash_attn")
if fa is not None:
    server_args.extend(["-fa", str(fa)])

# KV cache quantization
ctk = prof.get("cache_type_k")
if ctk:
    server_args.extend(["--cache-type-k", ctk])
ctv = prof.get("cache_type_v")
if ctv:
    server_args.extend(["--cache-type-v", ctv])

# Reasoning
reasoning = prof.get("reasoning")
if reasoning is not None:
    server_args.extend(["--reasoning", str(reasoning)])
rf = prof.get("reasoning_format")
if rf:
    server_args.extend(["--reasoning-format", rf])

# Boolean flags
if prof.get("metrics"):
    server_args.append("--metrics")
if prof.get("mlock"):
    server_args.append("--mlock")
if prof.get("jinja") is not None:
    if prof["jinja"]:
        server_args.append("--jinja")
    else:
        server_args.append("--no-jinja")

# RoPE
add("--rope-scaling", "rope_scaling")
add("--rope-freq-base", "rope_freq_base")

# API key
api_key = prof.get("api_key")
if api_key:
    server_args.extend(["--api-key", api_key])

# Multimodal projector (vision encoder — enables image input)
mmproj = prof.get("mmproj_path")
if mmproj:
    server_args.extend(["--mmproj", mmproj])

# Chat template
ct = prof.get("chat_template")
if ct:
    server_args.extend(["--chat-template", ct])

# Alias
alias = prof.get("alias")
if alias:
    server_args.extend(["-a", alias])

# Emit shell vars
print(f'DOCKER_IMAGE={shlex.quote(image)}')
print(f'CONTAINER_NAME={shlex.quote(name)}')
print(f'GPU_ID={shlex.quote(gpu_id)}')
print(f'SERVE_PORT={shlex.quote(port)}')
print(f'SERVE_HOST={shlex.quote(host)}')

docker_run = ["docker", "run", "-d", "--rm",
    "--name", name,
    "--gpus", f"device={gpu_id}",
    "-p", f"{host}:{port}:{port}"]
docker_run += vol_args
docker_run += [image]
docker_run += server_args

print(f'FULL_CMD={shlex.quote(" ".join(shlex.quote(a) for a in docker_run))}')
PYEOF
)"

# Append any extra args the user passed after the profile name
if [[ $# -gt 0 ]]; then
    for arg in "$@"; do
        FULL_CMD+=" $(printf '%q' "$arg")"
    done
fi

# ── command preview ───────────────────────────────────────────────────────────
echo "=== llama.cpp CUDA Server ==="
echo "Profile  : ${PROFILE}"
echo "Image    : ${DOCKER_IMAGE}"
echo "GPU      : device=${GPU_ID}"
echo "Port     : ${SERVE_PORT}"
echo ""

echo "Full docker command:"
echo "${FULL_CMD}"
echo ""

if [[ "$DRY_RUN" == true ]]; then
    echo "✅ Dry run complete; no container was started or stopped."
    exit 0
fi

# ── stop existing container ───────────────────────────────────────────────────
if docker ps -q --filter "name=${CONTAINER_NAME}" 2>/dev/null | grep -q .; then
    echo "⚠️  Stopping existing container: ${CONTAINER_NAME}"
    docker stop "${CONTAINER_NAME}" >/dev/null 2>&1
    docker rm "${CONTAINER_NAME}" >/dev/null 2>&1 || true
fi

# ── launch ────────────────────────────────────────────────────────────────────
eval "${FULL_CMD}"

# ── wait for ready ─────────────────────────────────────────────────────────────
echo "Waiting for server on http://${SERVE_HOST}:${SERVE_PORT} ..."
for i in $(seq 1 180); do   # increased to 6 minutes for larger/slower models
    if curl -sf "http://${SERVE_HOST}:${SERVE_PORT}/v1/models" >/dev/null 2>&1; then
        echo ""
        echo "✅ Server ready at http://${SERVE_HOST}:${SERVE_PORT}"
        echo "   OpenAI API: http://${SERVE_HOST}:${SERVE_PORT}/v1/chat/completions"
        echo "   Web UI: http://${SERVE_HOST}:${SERVE_PORT}"
        echo "   Logs: docker logs -f ${CONTAINER_NAME}"
        echo "   Stop: $0 --stop"
        exit 0
    fi
    printf "."
    sleep 2
done

echo ""
echo "⚠️  Server did not respond after 6 min."
echo "   Container name : ${CONTAINER_NAME}"
echo "   Check logs     : docker logs -f ${CONTAINER_NAME}   (or docker logs ${CONTAINER_NAME} for past output)"
echo "   Live logs      : docker logs -f ${CONTAINER_NAME}"
echo "   Status         : docker ps -a --filter name=${CONTAINER_NAME}"
echo "   Stop           : $0 --stop"
