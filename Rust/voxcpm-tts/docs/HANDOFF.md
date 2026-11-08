# Handoff: voxcpm-tts (read this first)

Living status document so another session or AI agent can continue the work.
Keep it current: update **Status** and **Next steps** whenever something lands.

## Goal (from the accepted product intent)

macOS-only (Apple Silicon) voice-cloning text-to-speech around the open-weight
model `mlx-community/VoxCPM2-bf16`, for the owner's MacBook Pro (M5, 32 GB) and
Mac mini (M4). The user supplies one speaker's `.wav`/`.mp3` and a script
(text file path or typed into an editor); the tool chunks the text, synthesizes
every chunk in the cloned voice, stitches one 48 kHz `.wav`, deletes the parts,
shows progress + ETA, loads weights automatically and frees them on exit. The
CLI comes first; a native `.app` with the same features follows. **Rust first;
Python only where unavoidable.** Full request: `docs/PRODUCT_INTENT.md`.

## Key decisions (and why)

| Decision | Reason | Record |
|---|---|---|
| Location `Rust/voxcpm-tts/` in claw-work | claw-work keeps Rust products under `Rust/` (lean-agent, claw-edit) | — |
| Native engine = **candle + Metal** | No Xcode on the dev Mac ⇒ no `metal` compiler ⇒ `mlx-rs` (builds MLX from source) is impractical; candle compiles Metal kernels at runtime | `docs/RUST_RUNTIME_DECISION.md` |
| Python **mlx-audio worker** kept as engine #2 | Works today; numerical oracle (golden tensors) for the Rust port; fallback | `docs/REFERENCE_ENGINE.md` |
| Engines live in a **separate process (worker) or are dropped on unload (native)** | "Free memory when closed": process exit / drop releases all weights | `src/engine/` |
| App layer is engine-independent | Same features for CLI and GUI; testable with a mock engine | `src/job.rs` |
| PDD: intent planned+applied with `--no-sync`; prompts/stories maintained by the agent | The model port is research-heavy code verified by golden tensors, not regenerated blindly (same pattern as `Rust/lean-agent`) | `docs/PDD_EVIDENCE.md` |

## Layout

```text
Rust/voxcpm-tts/
  Cargo.toml            single crate: lib `voxcpm_tts` + bin `voxcpm-tts`
  src/text.rs           normalize, sentence split, chunk packing (tests)
  src/audio.rs          decode wav/mp3/flac/m4a, sinc resampler, WAV write, stitcher (tests)
  src/voice.rs          reference clip validation/cleanup → mono WAV (tests)
  src/eta.rs            persisted calibration + live ETA (tests)
  src/job.rs            chunk → synthesize (bad-case retry) → stitch → atomic write → cleanup (mock-engine tests)
  src/engine/mod.rs     Engine trait, EngineKind
  src/engine/worker.rs  mlx-python engine: JSON-lines child process supervisor
  src/native/           native candle engine (VoxCPM2 port)
  src/hub.rs            download weights into the HF hub cache
  src/main.rs           CLI: say | session | plan | doctor | fetch
  reference/            Python mlx-audio worker, golden dumps (venv/golden/ untracked)
  docs/                 PRODUCT_INTENT, HANDOFF, PORTING_SPEC, WEIGHTS_INVENTORY,
                        REFERENCE_ENGINE, RUST_RUNTIME_DECISION, ...
```

## Status (2026-11-07, session 2: native CLI verified on the M5)

The native engine, app layer, CLI, and Mac app bundle are implemented. Nothing
under `Rust/voxcpm-tts/` is committed yet. The branch is `feature/voxcpm2-tts-macos`.

Verified on this MacBook Pro (Apple M5, 32 GB unified memory, macOS 26.2),
using the public `test_en.wav` shipped in the cached
`mlx-community/VoxCPM2-bf16` snapshot. No private voice was used.

- [x] `voxcpm-tts doctor` finds the cached weights, the native engine, and the
      checkout's mlx-python worker.
- [x] Native `say` of one sentence (`--timesteps 7 --seed 42`): 3.82 s, mono
      48 kHz PCM, load 7.5 s cold, 3.29× real time, one chunk, parts discarded.
- [x] Native `say` of a 624-character, two-paragraph script at the default 10
      diffusion steps and `--seed 42`: 3 chunks, one attempt each, 34.68 s of
      audio, 47.1 s of generation, warm load 3.7 s, 1.36× real time. Physical
      footprint while generating was about 12 GB, with a peak of 15 GB.
      `parts_directory` was null and no chunk WAVs were left behind.
- [x] Local Whisper large-v3-turbo (the cached MLX snapshot, not the empty
      Homebrew test model) transcribed that WAV back to the script. The only
      differences were spelling and "48 kHz" for the spoken "forty-eight
      kilohertz".
- [x] Interactive `session` then `:unload`: footprint fell from 12 GB to
      539 MB (compiled Metal pipelines and allocator arenas; same residual
      described in `RUST_RUNTIME_DECISION.md`). `:quit` exited 0. A one-shot
      `say` unloads and the process exits, which releases the rest.
- [x] Python worker protocol tests: 12 passed. CLI process-lifecycle tests:
      9 passed. Packaging tests: 4 passed.
- [x] Installed from the tested release binaries with
      `scripts/install.sh --skip-build --offline`: `~/.local/bin/voxcpm-tts`
      (already on PATH) and an ad-hoc signed `~/Applications/VoxCPM TTS.app`
      (`codesign --verify --deep --strict` passed). The Python fallback still
      resolves to this checkout's virtualenv because that path is compiled in.
      `scripts/install.sh --with-python-engine` copies it into Application
      Support when the checkout should not be required.
- [x] PDD artifacts exist: `.pddrc`, `architecture.json`, three prompts,
      `user_stories/story__voice_script_to_wav.md`, product intent. They were
      authored by the previous session, not regenerated here. Do not blind-sync
      the numerical port.
- [x] `cargo test --offline --all-targets --features gui`: 63 library tests
      passed, including the golden tokenizer, transformer, VAE, and T18
      full-model checks. CLI and GUI binaries have no unit tests of their own.
- [ ] The Mac app window was not click-tested. The GUI binary is what the
      bundle launches; it calls the same `job::run_job` path as the CLI.
- [ ] Mac mini M4 has not been run. Install there with `scripts/install.sh`
      (do not copy the virtualenv). Weights can be copied or fetched.
- [x] The crate is committed on `feature/voxcpm2-tts-macos` after the privacy
      gate. Golden tensors, the virtualenv, `dist/`, `target/`, and generated
      WAVs stay untracked. Do not push unless asked.

## Refined user stories (draft for approval)

1. **Clone a voice:** given one clean `.wav`/`.mp3`/`.flac`/`.m4a` clip of a single
   speaker (2–30 s; 5–20 s ideal; an optional exact transcript improves similarity),
   speech is generated in that voice. Clips that are too short, silent, or in an
   unsupported format are rejected with a clear message. Long clips are trimmed.
2. **Script in:** text comes from a file path (CLI), inline `--text`, `$EDITOR`
   (`--edit`, `:edit` in a session), or the GUI editor (type, paste, open, or drop a file).
3. **Automatic chunking and stitching:** long text splits at sentence boundaries
   (clauses or words only when one sentence is too long) and each chunk is synthesized.
   Runaway or truncated chunks are retried with a new seed. Parts are joined into
   one 48 kHz WAV with trimmed edges, matched loudness, pause-aware gaps, and no clicks.
   Parts are always deleted, including on error, Ctrl-C, or crash (stale sweep);
   `--keep-parts` is opt-in. Existing outputs are never overwritten without `--force`.
4. **Progress and ETA:** shows the plan up front (chunks, expected time, `plan`
   dry-run), then a live bar with chunk i/n, audio produced, elapsed time, and remaining
   time. Calibration persists per engine and setting, so estimates improve over use.
5. **Memory lifecycle:** weights load automatically (CLI command start, GUI
   launch, `session` start) and are freed on exit, window close, `:unload`, or the GUI
   "Free memory" button.
6. **Parity:** the CLI and the `.app` share one library and offer the same features.
7. **Rust first:** the native candle/Metal engine is the default once verified.
   The Python mlx-audio worker remains a fallback and the numerical oracle.

## Next steps

1. On the Mac mini M4, run `scripts/install.sh`, then `voxcpm-tts doctor` and
   one short `say` with a voice the user is allowed to clone.
2. Open `VoxCPM TTS.app` once and generate a short script, then use Free memory
   and quit. Confirm the window's progress line and that the process is gone.
3. Push only if Ernest asks. He merges `feature/voxcpm2-tts-macos` himself.

How to repeat the narration check, from the crate, with the cached public sample:

```bash
HF_HUB_OFFLINE=1 ~/.local/bin/voxcpm-tts say /path/to/script.txt \
  --voice /path/to/one-speaker.wav --engine native --seed 42 -o /tmp/narration.wav
```

`plan` does not load weights. `say` loads, speaks, stitches, deletes the parts,
and exits. `session` keeps weights loaded until `:unload` or `:quit`.

## How to verify

```bash
cd Rust/voxcpm-tts
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test
```
