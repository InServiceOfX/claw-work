#!/usr/bin/env bash
# launch.sh — Launch llama.cpp server natively on macOS (Metal).
#
# Usage:
#   ./launch.sh <profile>           # Start server with profile
#   ./launch.sh --stop              # Stop running server
#   ./launch.sh --status            # Show server status
#   ./launch.sh                     # List available profiles
#
# Global settings: config.yml (copy from config.yml.example)
# Model settings:  profiles/<profile>.yml
#
# Full server docs: https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

CONFIG_FILE="$SCRIPT_DIR/config.yml"
if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "Error: config.yml not found. Run: cp config.yml.example config.yml"
    exit 1
fi

PID_FILE="$SCRIPT_DIR/.llama-server.pid"
LOG_FILE="$SCRIPT_DIR/.llama-server.log"
# Records which profile is currently serving. Only one llama-server runs at a
# time on this port, while client configs (OpenClaw, Hermes) list every profile
# as a selectable model — selecting one does not load it. --status reads this
# so "which model am I actually talking to" has an answer.
ACTIVE_FILE="$SCRIPT_DIR/.llama-server.profile"

# Port --status probes for the loaded model. Read from config.yml's server.port;
# profiles may override it, but every profile here uses the global default.
STATUS_PORT=$(awk '/^server:/{s=1;next} /^[^[:space:]#]/{s=0} s && /^[[:space:]]+port:/{gsub(/[^0-9]/,"",$2); print $2; exit}' "$CONFIG_FILE" 2>/dev/null)
STATUS_PORT="${STATUS_PORT:-8080}"

# ── handle --stop / --status with no profile ──────────────────────────────────
case "${1:-}" in
    --stop|stop)
        if [[ -f "$PID_FILE" ]]; then
            PID=$(cat "$PID_FILE")
            if kill -0 "$PID" 2>/dev/null; then
                kill "$PID"
                echo "Stopped llama-server (PID $PID)"
            else
                echo "Process $PID not running (stale PID file)"
            fi
            rm -f "$PID_FILE"
        else
            echo "No PID file found. Checking for running llama-server..."
            pkill -f "llama-server.*--port" && echo "Killed" || echo "No llama-server found"
        fi
        rm -f "$ACTIVE_FILE"
        exit 0
        ;;
    --status|status)
        # A foreground launch execs the binary and never writes a PID file, so
        # "is it running" cannot be answered from the PID file alone — fall back
        # to pgrep. The profile/model report below runs either way; keying it to
        # the PID file would hide it for exactly the foreground case.
        RUNNING=0
        if [[ -f "$PID_FILE" ]]; then
            PID=$(cat "$PID_FILE")
            if kill -0 "$PID" 2>/dev/null; then
                echo "llama-server running (PID $PID, backgrounded by this script)"
                ps -p "$PID" -o pid,comm,etime,rss | tail -1
                RUNNING=1
            else
                echo "Not running (stale PID file)"
                rm -f "$PID_FILE" "$ACTIVE_FILE"
            fi
        elif pgrep -f "llama-server" >/dev/null 2>&1; then
            echo "llama-server running (foreground, or started outside this script):"
            pgrep -af "llama-server" | cut -c1-100
            RUNNING=1
        else
            echo "No llama-server running"
            rm -f "$ACTIVE_FILE"
        fi

        if [[ "$RUNNING" == "1" ]]; then
            [[ -f "$ACTIVE_FILE" ]] && echo "Active profile: $(cat "$ACTIVE_FILE")"
            # Ask the server itself, so this reports reality rather than what
            # the state file believes.
            LOADED=$(curl -sf -m 3 "http://127.0.0.1:${STATUS_PORT}/v1/models" 2>/dev/null \
                | sed -n 's/.*"id":"\([^"]*\)".*/\1/p' | head -1)
            [[ -n "$LOADED" ]] && echo "Loaded model:   $LOADED"
            echo ""
            echo "Note: client configs list every profile as a selectable model,"
            echo "      but only the profile above is loaded. Selecting another"
            echo "      silently serves this one. Re-launch to switch."
        fi
        exit 0
        ;;
    "")
        echo "Usage: $0 <profile> [extra llama-server args...]"
        echo "       $0 --stop | stop"
        echo "       $0 --status | status"
        echo ""
        echo "Available profiles:"
        for f in "$SCRIPT_DIR/profiles/"*.yml; do
            [[ -f "$f" ]] && basename "$f" .yml | sed 's/^/  /'
        done
        exit 0
        ;;
esac

PROFILE="$1"; shift

PROFILE_FILE="$SCRIPT_DIR/profiles/${PROFILE}.yml"
if [[ ! -f "$PROFILE_FILE" ]]; then
    echo "Error: Profile not found: $PROFILE_FILE"
    echo "Available:"
    for f in "$SCRIPT_DIR/profiles/"*.yml; do
        [[ -f "$f" ]] && basename "$f" .yml | sed 's/^/  /'
    done
    exit 1
fi

# ── resolve a Python with PyYAML ─────────────────────────────────────────────
# A venv next to this script wins; otherwise an ephemeral uv environment. The
# directory carries no dependency on whatever repository it sits in.
VENV_PYTHON="$SCRIPT_DIR/.venv/bin/python3"

if [[ -x "$VENV_PYTHON" ]] && "$VENV_PYTHON" -c "import yaml" 2>/dev/null; then
    PYTHON=("$VENV_PYTHON")
elif command -v uv >/dev/null 2>&1; then
    PYTHON=(uv run --quiet --no-project --with pyyaml python)
else
    echo "Error: need PyYAML. Either:"
    echo "  cd $SCRIPT_DIR && uv venv --python 3.13 && uv pip install pyyaml"
    echo "  or install uv (brew install uv) and re-run this script."
    exit 1
fi

# ── build server args via Python ─────────────────────────────────────────────
eval "$("${PYTHON[@]}" - "$CONFIG_FILE" "$PROFILE_FILE" <<'PYEOF'
import json
import os
from pathlib import Path
import sys
import yaml
import shlex

cfg = yaml.safe_load(open(sys.argv[1])) or {}
prof = yaml.safe_load(open(sys.argv[2])) or {}

srv_cfg = cfg.get("server", {})
binary = srv_cfg.get("binary", "llama-server")
models_dir = cfg.get("models_dir", "")

def fail(msg):
    print(f'echo {shlex.quote("Error: " + msg)} >&2; exit 1')
    sys.exit(0)


def hf_cache_root():
    """Standard HuggingFace hub cache, matching huggingface_hub's own order."""
    explicit = cfg.get("hf_cache") or os.environ.get("HF_HUB_CACHE")
    if explicit:
        return Path(os.path.expandvars(os.path.expanduser(str(explicit))))
    hf_home = os.environ.get("HF_HOME")
    if hf_home:
        return Path(os.path.expandvars(os.path.expanduser(hf_home))) / "hub"
    return Path.home() / ".cache" / "huggingface" / "hub"


def resolve_hf_file(repo_id, filename, revision=None):
    """Find <cache>/models--org--repo/snapshots/<rev>/<filename>.

    Resolving at launch time (rather than pinning a snapshot hash in the
    profile) keeps profiles valid across `fetch-model.sh` re-runs and model
    revision bumps.
    """
    root = hf_cache_root() / ("models--" + str(repo_id).replace("/", "--"))
    hint = (f"Not in the HuggingFace cache: {repo_id}/{filename}\n"
            f"  Looked under: {root}\n"
            f"  Download it: ./fetch-model.sh download {repo_id} {filename}")
    if not root.is_dir():
        fail(hint)

    if not revision:
        ref_main = root / "refs" / "main"
        if ref_main.is_file():
            revision = ref_main.read_text(encoding="utf-8").strip()

    snapshots = root / "snapshots"
    ordered = []
    if revision and (snapshots / revision).is_dir():
        ordered.append(snapshots / revision)
    if snapshots.is_dir():
        ordered.extend(sorted(
            (p for p in snapshots.iterdir() if p.is_dir() and p not in ordered),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        ))

    for snap in ordered:
        candidate = snap / filename
        if candidate.exists():
            return str(candidate)
    fail(hint)


# Model location. A profile gives either:
#   hf_repo + hf_file  — resolved through the HuggingFace hub cache (preferred), or
#   model_path         — absolute, or relative to models_dir from config.yml.
model_path = prof.get("model_path", "")
hf_repo = prof.get("hf_repo")
hf_file = prof.get("hf_file")

if model_path:
    model_path = os.path.expandvars(os.path.expanduser(str(model_path)))
    if not os.path.isabs(model_path):
        if not models_dir:
            fail("Relative model_path requires models_dir in config.yml")
        base = Path(os.path.expandvars(os.path.expanduser(str(models_dir))))
        model_path = str(base / model_path)
elif hf_repo and hf_file:
    model_path = resolve_hf_file(hf_repo, hf_file, prof.get("hf_revision"))
else:
    fail("Profile needs either model_path, or hf_repo + hf_file")

if not os.path.exists(model_path):
    fail(f"Model file not found: {model_path}")

# Server host/port: profile overrides config
srv_prof = prof.get("server", {})
host = srv_prof.get("host", srv_cfg.get("host", "127.0.0.1"))
port = str(srv_prof.get("port", srv_cfg.get("port", 8080)))

# Build llama-server args
server_args = ["-m", model_path, "--host", host, "--port", port]

def add(flag, key, src=prof):
    val = src.get(key)
    if val is not None and val != "":
        server_args.extend([flag, str(val)])

def add_bool(flag_on, flag_off, key):
    val = prof.get(key)
    if val is None:
        return
    server_args.append(flag_on if val else flag_off)

add("-ngl", "n_gpu_layers")
add("-c", "ctx_size")
add("-n", "n_predict")
add("-np", "parallel")
add("-b", "batch_size")
add("-ub", "ubatch_size")
add("-t", "threads")

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
add("--reasoning-budget", "reasoning_budget")

# Sampling defaults. These apply to requests that don't set their own values,
# so a profile can carry the model card's recommended recipe.
add("--temp", "temp")
add("--top-p", "top_p")
add("--top-k", "top_k")
add("--min-p", "min_p")
add("--presence-penalty", "presence_penalty")
add("--frequency-penalty", "frequency_penalty")
add("--repeat-penalty", "repeat_penalty")
add("--repeat-last-n", "repeat_last_n")

# Boolean flags
if prof.get("metrics"):
    server_args.append("--metrics")
if prof.get("mlock"):
    server_args.append("--mlock")
add("--prio", "priority")
add("--poll", "poll")
add("--prio-batch", "priority_batch")
add("--poll-batch", "poll_batch")
if prof.get("no_mmap"):
    server_args.append("--no-mmap")
if prof.get("jinja") is not None:
    if prof["jinja"]:
        server_args.append("--jinja")
    else:
        server_args.append("--no-jinja")
add_bool("--cont-batching", "--no-cont-batching", "cont_batching")

# RoPE / YaRN long-context extension
add("--rope-scaling", "rope_scaling")
add("--rope-scale", "rope_scale")
add("--rope-freq-base", "rope_freq_base")
add("--rope-freq-scale", "rope_freq_scale")
add("--yarn-orig-ctx", "yarn_orig_ctx")
add("--yarn-ext-factor", "yarn_ext_factor")
add("--yarn-attn-factor", "yarn_attn_factor")
add_bool("--context-shift", "--no-context-shift", "context_shift")

# API key
api_key = prof.get("api_key")
if api_key:
    server_args.extend(["--api-key", api_key])

# Multimodal projector
mmproj = prof.get("mmproj_path")
if mmproj:
    server_args.extend(["--mmproj", mmproj])

# Chat template
ct = prof.get("chat_template")
if ct:
    server_args.extend(["--chat-template", ct])
ct_kwargs = prof.get("chat_template_kwargs")
if ct_kwargs:
    if isinstance(ct_kwargs, (dict, list)):
        ct_kwargs = json.dumps(ct_kwargs, separators=(",", ":"))
    server_args.extend(["--chat-template-kwargs", str(ct_kwargs)])

# Alias
alias = prof.get("alias")
if alias:
    server_args.extend(["-a", alias])

# Speculative decoding
if prof.get("spec_default"):
    server_args.append("--spec-default")

# Server tuning
add("--timeout", "timeout")
add("--threads-http", "threads_http")
add("--cache-ram", "cache_ram")
add("--ctx-checkpoints", "ctx_checkpoints")
add("--checkpoint-min-step", "checkpoint_min_step")
add_bool("--cache-prompt", "--no-cache-prompt", "cache_prompt")
add_bool("--cache-idle-slots", "--no-cache-idle-slots", "cache_idle_slots")
add_bool("--warmup", "--no-warmup", "warmup")

# Escape hatch for newly-added llama-server flags.
extra_args = prof.get("extra_args") or []
if isinstance(extra_args, str):
    extra_args = shlex.split(extra_args)
server_args.extend(str(arg) for arg in extra_args)

# Emit shell vars
print(f'LLAMA_BINARY={shlex.quote(binary)}')
print(f'SERVE_PORT={shlex.quote(port)}')
print(f'SERVE_HOST={shlex.quote(host)}')
print(f'SERVER_ARGS=({" ".join(shlex.quote(a) for a in server_args)})')
PYEOF
)"

# ── stop existing if managed ─────────────────────────────────────────────────
if [[ -f "$PID_FILE" ]]; then
    OLD_PID=$(cat "$PID_FILE")
    if kill -0 "$OLD_PID" 2>/dev/null; then
        echo "Stopping existing llama-server (PID $OLD_PID)..."
        kill "$OLD_PID"
        sleep 1
    fi
    rm -f "$PID_FILE"
fi

# ── launch ────────────────────────────────────────────────────────────────────
echo "=== llama.cpp Metal Server ==="
echo "Profile  : ${PROFILE}"
echo "Binary   : ${LLAMA_BINARY}"
echo "Endpoint : http://${SERVE_HOST}:${SERVE_PORT}"
echo ""
echo "Command:"
echo "  ${LLAMA_BINARY} ${SERVER_ARGS[*]}" "$@"
echo ""

# Run in foreground by default (Ctrl-C to stop)
# Use --background flag to daemonize
if [[ "${1:-}" == "--background" || "${1:-}" == "-bg" ]]; then
    shift
    # Redirect to a log file rather than inheriting stdout: a backgrounded
    # server otherwise writes into whatever pipe launched it, which makes the
    # startup log (KV self size, n_ctx_per_seq, load errors) unreadable after
    # the fact.
    : > "$LOG_FILE"
    "${LLAMA_BINARY}" "${SERVER_ARGS[@]}" "$@" >>"$LOG_FILE" 2>&1 &
    SERVER_PID=$!
    echo "$SERVER_PID" > "$PID_FILE"
    echo "$PROFILE" > "$ACTIVE_FILE"
    echo "Started in background (PID $SERVER_PID)"
    echo "Log: $LOG_FILE"
    echo ""

    # Wait for ready. Generous headroom, but it exits the moment the server
    # answers: a 9B with mlock and a 262144-token KV cache is listening in
    # ~3.4s, so this only matters for far larger models or cold-cache loads.
    echo "Waiting for server on http://${SERVE_HOST}:${SERVE_PORT} ..."
    for i in $(seq 1 300); do
        if curl -sf "http://${SERVE_HOST}:${SERVE_PORT}/v1/models" >/dev/null 2>&1; then
            echo ""
            echo "Server ready at http://${SERVE_HOST}:${SERVE_PORT}"
            echo "  OpenAI API: http://${SERVE_HOST}:${SERVE_PORT}/v1/chat/completions"
            echo "  Web UI:     http://${SERVE_HOST}:${SERVE_PORT}"
            echo "  Log:        $LOG_FILE"
            echo "  Stop:       $0 --stop"
            exit 0
        fi
        # A load failure (bad flag, assert, out of memory) kills the process
        # within seconds; report it now rather than polling for 10 minutes.
        if ! kill -0 "$SERVER_PID" 2>/dev/null; then
            echo ""
            echo "Error: llama-server exited during startup. Last lines of $LOG_FILE:"
            tail -n 20 "$LOG_FILE"
            rm -f "$PID_FILE" "$ACTIVE_FILE"
            exit 1
        fi
        printf "."
        sleep 2
    done
    echo ""
    echo "Warning: Server did not respond after 10 min. Check: ps -p $SERVER_PID"
    echo "         Log: $LOG_FILE"
else
    echo "$PROFILE" > "$ACTIVE_FILE"
    exec "${LLAMA_BINARY}" "${SERVER_ARGS[@]}" "$@"
fi
