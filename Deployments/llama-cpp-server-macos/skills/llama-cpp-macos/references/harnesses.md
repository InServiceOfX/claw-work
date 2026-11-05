# Connecting harnesses to llama-server

Every snippet below was verified end to end against llama-server 0.9.0
(build 12110) serving Qwen3.8-9B-Distill Q4_K_M on a 32 GB MacBook Pro M5:
the arithmetic prompt answered correctly in each harness, and a file-reading
tool call round-tripped in pi and Claude Code. Measured times are first-turn
wall clock, dominated by prefilling the harness's own prompt.

| Harness (version tested) | Prompt tokens | Time |
|---|---:|---:|
| pi 1.5.0 | small | ~15 s with a tool call |
| Claude Code 2.1.282, `--bare` | ~1.4K | 8 s; 13 s with a tool call |
| Codex 0.166.0 | ~9.9K | 20-29 s |
| Hermes Agent 0.21.3 | ~16.9K | 56 s |
| Grok Build 1.0.40 | ~19.8K | 60 s |

Placeholders: `PORT` is `server.port` from `config.yml`; `PROFILE` is the
launched profile name; `CTX` is that profile's `ctx_size / parallel`. The model
id is a label only — llama-server serves whatever profile is loaded — but use
the profile name so `--status` and the harness agree.

Back up each file before editing it, and add entries rather than replacing the
file.

## pi

`~/.pi/agent/models.json` (create it if absent; merge into `providers` if not):

```json
{
  "providers": {
    "llama-cpp-local": {
      "name": "llama.cpp (local)",
      "baseUrl": "http://127.0.0.1:PORT/v1",
      "api": "openai-completions",
      "apiKey": "local",
      "models": [
        {
          "id": "PROFILE",
          "name": "PROFILE (local)",
          "reasoning": true,
          "contextWindow": CTX,
          "maxTokens": 32768,
          "cost": { "input": 0, "output": 0, "cacheRead": 0, "cacheWrite": 0 }
        }
      ]
    }
  }
}
```

```sh
pi --list-models llama-cpp
pi --model llama-cpp-local/PROFILE                 # interactive
pi --no-session --model llama-cpp-local/PROFILE -p "What is 17*23? Number only."
```

pi's built-in `llama.cpp` provider (`/login llama.cpp`, `/llama`) targets the
multi-model router (`llama serve`), not single-model `llama-server`.

## Claude Code

Claude Code speaks the Anthropic Messages API, which llama-server serves at
`/v1/messages`. Two requirements:

1. **Patched chat template.** Claude Code sends a system message after the
   first user turn; Qwen3-family templates raise on that. Run
   `patch-chat-template.py PROFILE` and add the printed `chat_template_file:`
   line to the profile, then re-launch.
2. **`--bare --strict-mcp-config`.** Without them the request carries every
   configured MCP server's tools, plugins, hooks and CLAUDE.md files: one
   measured request was 855 KB with 228 tools, around 200K tokens, which is many
   minutes of prefill per turn on a 9B. Bare mode sends three tools (Bash, Edit,
   Read) and ~1.4K tokens. It also skips CLAUDE.md discovery; add context
   explicitly with `--append-system-prompt-file` or `--add-dir`.

Bare mode authenticates only with `ANTHROPIC_API_KEY`, so set it for this
command only; that leaves the normal Claude login untouched. A shell function in
`~/.zshrc`:

```sh
claude-local() {
  ANTHROPIC_BASE_URL=http://127.0.0.1:PORT \
  ANTHROPIC_API_KEY=local \
  CLAUDE_CODE_MAX_CONTEXT_TOKENS=CTX \
  CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC=1 \
  claude --bare --strict-mcp-config --model PROFILE "$@"
}
```

```sh
claude-local                                       # interactive
claude-local -p "What is 17*23? Number only."
```

`CLAUDE_CODE_MAX_CONTEXT_TOKENS` tells Claude Code the real window; it does not
know the model and otherwise assumes 200K. The `unrecognized_model` notice it
prints is expected. With `-p`, write `--allowedTools=Read` with `=`: the
space-separated form is variadic and swallows the prompt.

## Codex

Codex speaks the Responses API, served at `/v1/responses`. Use a profile file,
`~/.codex/local.config.toml` (layered over `config.toml`; Codex 0.166 rejects
the legacy `[profiles.<name>]` table):

```toml
model_provider = "llama-cpp-local"
model = "PROFILE"

[model_providers.llama-cpp-local]
name = "llama.cpp (local)"
base_url = "http://127.0.0.1:PORT/v1"
wire_api = "responses"
```

```sh
codex -p local                                      # interactive
codex exec --skip-git-repo-check -p local "What is 17*23? Number only."
```

No OpenAI login is needed for this provider. The warning "Model metadata for
`PROFILE` not found. Defaulting to fallback metadata" is expected. One-off
equivalent without a file:
`-c 'model_providers.llama-cpp-local={name="llama.cpp (local)",base_url="http://127.0.0.1:PORT/v1",wire_api="responses"}' -c model_provider=llama-cpp-local -m PROFILE`.

## Grok Build

Custom models must be in `~/.grok/config.toml`; the `GROK_CONFIG` overlay drops
`[model.*]` tables.

```toml
[model.local]
model = "PROFILE"
base_url = "http://127.0.0.1:PORT/v1"
name = "PROFILE (local llama.cpp)"
api_key = "local"
api_backend = "chat_completions"
context_window = CTX
max_completion_tokens = 32768
```

Set `api_key` so Grok does not send your Grok session token to the local
server. `context_window` drives auto-compaction (default otherwise 200K).

```sh
grok models | grep local
grok -m local                                       # interactive
grok -m local -p "What is 17*23? Number only."
```

## Hermes Agent

`~/.hermes/config.yaml`, one alias per profile:

```yaml
model_aliases:
  local-qwen:
    model: PROFILE
    provider: custom
    base_url: http://127.0.0.1:PORT/v1
providers:
  custom:
    request_timeout_seconds: 5400
```

Merge into existing `model_aliases:` / `providers:` keys rather than adding
duplicates. The raised timeout matters: Hermes's 1800 s default is shorter than
a deep-context prefill.

```sh
hermes -m local-qwen                                # interactive
hermes chat -m local-qwen -q "What is 17*23? Number only." --oneshot -Q
```

## Other OpenAI-compatible clients

Base URL `http://127.0.0.1:PORT/v1`, any non-empty API key (unless the profile
sets `api_key`), model id `PROFILE`. llama-server also serves `/v1/messages`
(Anthropic) and `/v1/responses` (OpenAI Responses), plus a browser chat UI at
`http://127.0.0.1:PORT`.
