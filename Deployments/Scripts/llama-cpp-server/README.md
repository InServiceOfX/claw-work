# llama.cpp CUDA server deployment

A profile-driven Docker deployment for local GGUF models and an OpenAI-compatible
API. The existing Bash launcher and chat client were moved from Monoclaw into
claw-work; they were not reimplemented. Machine-local model paths remain in
ignored YAML files. Model weights and private repository history are not copied.

## Requirements

- Docker with the NVIDIA Container Toolkit and a supported NVIDIA GPU/driver.
- Bash, Python 3 and PyYAML for the launcher; curl and jq for `chat.sh`.
- Existing complete GGUF model files. A small Git LFS pointer is not a model.
- `rustc` only for the optional code-generation benchmark.

The model files are mounted read-only. The launcher does not download weights.
Docker can pull its configured image if it is not cached.

## Configure and run

```bash
cp config.yml.example config.yml
cp profiles/qwen38-9b-distill-q4.yml.example profiles/qwen38-9b-distill-q4.yml
# Edit config.yml: GPU selection and host model-volume paths.
# Edit the local profile: container model path and context/output limits.
python3 audit_profiles.py
./launch.sh --dry-run qwen38-9b-distill-q4
./launch.sh qwen38-9b-distill-q4
./launch.sh --status
./chat.sh --host 127.0.0.1 --port 8080
./launch.sh --stop
```

`config.yml` and every `profiles/*.yml` are ignored. Commit only `.yml.example`
templates. If the model is on a different drive, edit the volume mapping rather
than moving or duplicating the weights. `model_path` and `mmproj_path` refer to
paths inside the container. `host_model_path` documents the host file; the
launcher does not consume that field. The audit checks whether it agrees with
the actual Docker mapping.

The profile's `server.host` selects Docker's host bind address. The server binds
all interfaces inside its container. Use `127.0.0.1` for local Pi use.

## Model profiles

Templates cover Qwen3.5 distilled Q4/Q5/Q8, Qwen3.8 distilled Q4/Q5, Qwythos
Q4/Q5/Q6 and Q5 with a vision projector, plus Gemma E4B Q5/Q6. Availability
is host-specific: templates do not imply that the weights are installed.
`audit_profiles.py` reports missing files, LFS pointers, malformed GGUF headers,
unmapped model paths and projector availability without loading weights.

A model's advertised context is not its practical GPU allocation. Start with a
conservative context and increase only after a real load and generation check:

```bash
./launch.sh qwen38-9b-distill-q4 -c 32768 --reasoning-budget 512 --jinja
```

`ctx_size` is prompt plus history plus output room. `n_predict` is a default
output cap, and an API client's `max_tokens` can override it. A reasoning budget
is separate from the output cap. Extra command-line arguments after the profile
are forwarded to llama-server, preserving the existing launcher interface.

## Benchmark and Pi integration

See [BENCHMARKS.md](BENCHMARKS.md) for the measured local comparison, runtime
version, tested settings, recommendation and limitations. It is a small smoke
suite for this workflow, not a general model leaderboard.

[MODE_COMPARISON.md](MODE_COMPARISON.md) compares Qwythos Q5 with all three
native Nemotron modes on the same GPU, including first-text latency, sampled
VRAM, normal-profile headroom and a larger-model candidate. `compare_modes.py`
records streamed end-to-end token rates from an already-running local server.

```bash
python3 -m unittest discover -s tests -v
python3 benchmark.py --url http://127.0.0.1:8080 --out /tmp/local-model-benchmark
```

The suite checks a one-sentence greeting, exactly 20 primes, a Rust merge
function with authored tests, six independent Rust cases, and an addition-tool
round trip. It uses seeded sampling and reports response latency and server
throughput. Raw responses and compiler logs must stay outside Git; the script
rejects a result directory inside a Git working tree. No model-generated tool
commands are executed; the tool probe performs only the requested integer sum.
Generated Rust is restricted to the pure merge task before bounded compilation.

The measured recommendation is Qwythos Q5. Restart it with
`./run-recommended.sh`, merge the single-model `examples/pi/` configuration,
then use `/reload` and `/model qwythos` in Pi. The per-model default is low
thinking; global/cloud defaults need not change.

The selected model's portable Pi configuration and restart command are recorded
with the benchmark results. Use the ordinary `/model` picker and keep existing
provider/authentication settings when merging a profile into Pi.

## Layout

- `launch.sh`, `chat.sh`: moved Bash deployment and interactive client.
- `config.yml.example`, `profiles/*.yml.example`: public configuration templates.
- `audit_profiles.py`: local mount/header audit.
- `benchmark.py`, `compare_modes.py`, `tests/`: bounded acceptance, streaming speed and grading checks.
- `run-recommended.sh`, `examples/pi/`: measured single-model restart/client setup.
- `vision_smoke.py`: synthetic image transport check for projector profiles.
- `PROGRESS.md`: continuation state and verification scope.

## Runtime reference

The launcher uses [llama.cpp server](https://github.com/ggml-org/llama.cpp/tree/master/tools/server).
Model-card guidance is linked from the corresponding profile. The tested image
revision and digest are pinned in the benchmark receipt; do not assume results
transfer unchanged to another image, quantization, GPU or context size.
