#!/usr/bin/env bash
# fetch-model.sh — download model weights into the standard HuggingFace hub cache.
#
# Files land in ~/.cache/huggingface/hub (or $HF_HUB_CACHE / $HF_HOME/hub), the
# same cache mlx_lm and transformers already populate on this machine. Nothing
# is copied into the repo.
#
# Usage:
#   ./fetch-model.sh list <repo_id>
#   ./fetch-model.sh download <repo_id> <file> [file...]
#   ./fetch-model.sh path <repo_id> [file]
#   ./fetch-model.sh [--revision <sha>] <subcommand> ...
#
# Example:
#   ./fetch-model.sh list empero-ai/Qwen3.8-9B-Distill-GGUF
#   ./fetch-model.sh download empero-ai/Qwen3.8-9B-Distill-GGUF Qwen3.8-9B-Q8_0.gguf
#
# Then reference the model in a profile by repo + file (see ADDING-A-MODEL.md):
#   hf_repo: empero-ai/Qwen3.8-9B-Distill-GGUF
#   hf_file: Qwen3.8-9B-Q8_0.gguf

set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
HELPER="$SCRIPT_DIR/hf_fetch.py"

if [[ $# -eq 0 ]]; then
    sed -n '2,20p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 0
fi

# Xet-backed repos (most new GGUF uploads) download substantially faster with
# hf_xet installed; huggingface_hub falls back to plain HTTP without it.
HF_DEPS='huggingface_hub[hf_xet]'

VENV_PYTHON="$SCRIPT_DIR/.venv/bin/python3"

if [[ -x "$VENV_PYTHON" ]] && "$VENV_PYTHON" -c "import huggingface_hub" 2>/dev/null; then
    exec "$VENV_PYTHON" "$HELPER" "$@"
fi

if command -v uv >/dev/null 2>&1; then
    # Ephemeral env — keeps the local venv free of a dependency only this
    # script needs. To make it permanent instead:
    #   cd "$SCRIPT_DIR" && uv pip install 'huggingface_hub[hf_xet]'
    exec uv run --quiet --no-project --with "$HF_DEPS" python "$HELPER" "$@"
fi

echo "Error: need huggingface_hub. Either:" >&2
echo "  cd $SCRIPT_DIR && uv venv --python 3.13 && uv pip install '$HF_DEPS'" >&2
echo "  or install uv (brew install uv) and re-run this script." >&2
exit 1
