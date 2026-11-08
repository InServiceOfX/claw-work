# Packaging and installation evidence

## Artifacts

- `scripts/install.sh`: build/install CLI and optional `.app`; optional Python fallback.
- `scripts/bundle-app.sh`: make an ad-hoc signed app from a release build or `--binary`.
- `packaging/Info.plist`: portable app identity and macOS requirements.
- `reference/requirements.txt`: verified pinned MLX environment.
- `scripts/test_packaging.py`: overwrite, staging, metadata and signature boundaries.

Default installation builds Rust release binaries. Python and model downloads are
explicit installer options. The app bundle contains only its native executable,
Info.plist and the generated code-signing manifest. It does not bundle a venv,
model, script, reference clip, checkout, cache or a private build configuration.
The native engine uses the shared Hugging Face model cache. Python fallback
installs into the app-support directory (override `VOXCPM_TTS_HOME`).

Custom `--prefix` and `--app-dir` allow staging without touching home-directory
installs. `--support-dir` allows optional Python staging too; it requires the
matching runtime environment override. Existing selected artifacts fail before
installation changes unless `--force` is supplied. Temp staging directories are
removed on success/failure. `--force` intentionally replaces the selected CLI,
app and, when requested, Python worker. Keep unrelated files elsewhere.

The Python environment is created with `uv venv --relocatable` before moving
it to its final support path. It still uses the local Python installation; run
the installer independently on another Mac. Do not copy the venv between Macs.

## Checks completed 2026-11-07

- Shell syntax check and `plutil -lint` passed.
- Workspace-local debug CLI and `.app` staged under ignored `dist/stage/`.
- CLI `--version` returned `voxcpm-tts 0.1.0`.
- `codesign --verify --deep --strict` passed for the staged `.app`.
- Four packaging tests passed, including overwrite refusal with unchanged
  sentinels, explicit `--force`, paths with spaces, exact bundle file allowlist,
  valid bundle executable metadata/signature, and rejection of `--offline --fetch`
  before writes. The test uses temporary destinations and existing debug binaries.
- Optional fallback installed from existing uv cache with networking disabled
  into ignored `dist/python-stage/support/reference`. All 38 pinned packages
  installed successfully. After the relocatable venv moved, its interpreter
  reported `mlx-audio 0.6.4` and `mlx 0.32.4`; the installed worker emitted ready,
  ping (`loaded:false`) and shutdown, exiting zero.
- No global installation was performed by these packaging checks.

These checks establish packaging and process discovery; they do not establish
GUI interaction, audible quality, or M4 compatibility. A final release build,
launch check and desired home-directory deployment remain integration steps.
The app is ad-hoc signed, not notarized. `--offline` covers installer Cargo/uv
operations, not application runtime. Installer `--offline --fetch` is deliberately rejected. Cache weights before a
disconnected run and set `HF_HUB_OFFLINE=1`; both runtime engines honor that
setting and require complete cached model files.
