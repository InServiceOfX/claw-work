---
name: llama-cpp-macos
description: Run local GGUF open-weight models on macOS (Apple Silicon, Metal) with llama.cpp's llama-server, weights from the HuggingFace hub cache, and connect them to Claude Code, Codex, Grok Build, Hermes Agent or pi. Use to set up, launch, switch, size or troubleshoot a local model, or to point an AI harness at it.
---

# Local GGUF models on macOS with llama.cpp

The tooling lives in the claw-work repository under
`Deployments/llama-cpp-server-macos/` (launch.sh, fetch-model.sh,
patch-chat-template.py, profiles). Locate an existing checkout first; if there is
none, ask before cloning `https://github.com/InServiceOfX/claw-work`. Its README
and ADDING-A-MODEL.md are the reference; this skill is the procedure.

Ask before editing any harness configuration, installing packages, or binding to
anything but 127.0.0.1. Back up a config file before changing it, and only add
entries; never rewrite unrelated settings. Never commit `config.yml`,
`profiles/*.yml` or `templates/` (they are gitignored and machine-local).

## 1. Prerequisites

- Apple Silicon Mac; `brew install llama.cpp uv`. Check `llama-server --version`.
- If llama-server aborts at load with `GGML_ASSERT` in `ggml_gated_delta_net`,
  Homebrew upgraded `ggml` without `llama.cpp`: run `brew upgrade llama.cpp`.

## 2. Weights

Models live in the standard HuggingFace cache (`~/.cache/huggingface/hub`,
honouring `HF_HUB_CACHE` / `HF_HOME`), shared with mlx_lm and transformers.

```sh
./fetch-model.sh list <org/repo-GGUF>              # files and exact sizes
./fetch-model.sh download <org/repo-GGUF> <file>   # into the cache
./fetch-model.sh path <org/repo-GGUF> <file>       # check, no network
```

Check the cache before downloading anything; models may already be there.

## 3. Configure this machine

1. `cp config.yml.example config.yml`. Pick the port: if
   `lsof -nP -iTCP:8080 -sTCP:LISTEN` shows another process, set `server.port`
   (e.g. 8081). Profiles inherit it; do not set a port in profiles.
2. Copy a `profiles/<name>.yml.example` to `profiles/<name>.yml`, or write a new
   one from ADDING-A-MODEL.md. Reference weights with `hf_repo` + `hf_file`,
   never a snapshot path.
3. Size it for this machine's memory (`sysctl -n hw.memsize`). The examples
   target 64 GB; on 32 GB use the README's 32 GB table (`mlock: false`,
   `threads` = performance-core count). Usable context per request is
   `ctx_size / parallel`.

## 4. Launch and smoke-test

```sh
./launch.sh                                # list profiles
./launch.sh <profile> --background         # fails fast with the log tail
./launch.sh --status                       # the profile actually loaded
curl -s http://127.0.0.1:<port>/v1/chat/completions -H 'Content-Type: application/json' \
  -d '{"messages":[{"role":"user","content":"What is 17*23? Number only."}],"max_tokens":2048}'
```

One model is loaded at a time, and llama-server ignores the requested model id:
whatever is loaded answers. Switch by re-launching another profile.

## 5. Connect a harness

Read [references/harnesses.md](references/harnesses.md) for each harness's exact
configuration, verified end to end. Summary:

| Harness | API used | Where |
|---|---|---|
| pi | Chat Completions | `~/.pi/agent/models.json` custom provider |
| Claude Code | Anthropic Messages | env vars + `--bare --strict-mcp-config`; needs the patched template |
| Codex | Responses | `~/.codex/<name>.config.toml` profile, `codex -p <name>` |
| Grok Build | Chat Completions | `[model.<name>]` in `~/.grok/config.toml` |
| Hermes Agent | Chat Completions | `model_aliases` in `~/.hermes/config.yaml` |

Claude Code sends a system message after the first user turn, which Qwen3-family
templates reject ("System message must be at the beginning"). Fix once per profile:

```sh
uv run --no-project --with gguf --with pyyaml --with jinja2 \
  python patch-chat-template.py <profile>
# then add the printed `chat_template_file:` line to profiles/<profile>.yml and re-launch
```

## 6. Verify, then report

Run each configured harness once, non-interactively, with the arithmetic prompt,
then once with a tool call (e.g. read a scratch file and repeat a codeword).
Report: profile and GGUF loaded, port, harnesses configured, files changed and
their backups, and measured prompt sizes and times from `.llama-server.log`
(`prompt eval time`).

Prefill speed is the constraint: a 9B on an M-series Mac prefills roughly
350-400 tokens/s, so a harness's system prompt and tool list (10-20K tokens
is normal; hundreds of MCP tools can reach 200K) costs seconds to minutes before
the first token. Keep tool sets lean for local models.
