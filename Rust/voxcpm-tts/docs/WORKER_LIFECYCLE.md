# Worker process lifecycle verification

The Python reference engine runs in a child process. The Rust frontend owns
that process and must reclaim it and all intermediate audio on success,
failure, cancellation, unload and drop. These tests exercise those process
boundaries without a model, Metal device or network.

## Reproduced bugs and repairs

Before repair, an otherwise successful CLI job hung beyond seven seconds
when a mock worker ignored `unload`. The engine waited for that unbounded
reply before reaching its existing bounded child-process destructor.
`unload()` now drops the worker directly: shutdown gets a three-second grace
period, followed by kill and wait. Process exit is the memory-release boundary.

A second reproduction kept the interactive session open after a failed
synthesis request. The next request observed the previous partial WAV still
on disk. Each request now has its own temporary-directory guard, so its output
is removed on success, protocol failure, unreadable audio or cancellation,
independently of whether the session keeps its model loaded.

Startup and request reads now have absolute deadlines: 60 seconds for the
ready event, ten minutes for model loading and thirty minutes for a synthesis
request. Progress and unrelated messages do not extend the deadline. A timeout
kills and reaps the worker; these are final upper bounds, with ordinary user
cancellation polled every 100 ms. Short private test deadlines reproduce hung
startup/loading behavior without slow acceptance runs.

`WorkerEngine::load_cancellable` propagates the shared cancellation flag
through both worker readiness and model loading. CLI/GUI callers can therefore
cancel before synthesis begins. The ordinary `load()` interface remains
available for callers that do not have a cancellation token.

## Offline process acceptance

```sh
cargo build --offline --bin voxcpm-tts
python3 tests/test_cli_lifecycle.py
cargo test --offline --lib engine::worker::tests -- --test-threads=1
```

`VOXCPM_TTS_TEST_BINARY` can select a different built CLI. The Python tests use
only stdlib modules and generate their mock JSONL worker, synthetic PCM audio
and scripts inside temporary directories. The mock is selected with the
existing worker/interpreter environment overrides. User application support
and temporary directories are redirected to the test workspace.

The CLI tests assert observable outcomes:

1. A script path containing spaces produces multiple synthesis requests and
   one readable mono 48 kHz PCM WAV; the child and intermediate files disappear.
2. An existing output is preserved byte-for-byte without even starting a worker.
3. A reported inference error leaves no final WAV, partial output or child.
4. A worker crash leaves no final WAV, partial output or unreaped child.
5. Ctrl-C during synthesis returns exit code 130 and removes outputs/child.
6. Ctrl-C while waiting for worker readiness does the same.
7. Ctrl-C during model loading does the same.
8. A worker ignoring graceful shutdown is terminated within the bounded grace
   period, and a successful synthesis still returns success.
9. Failed-request cleanup happens before the next interactive-session job,
   while the frontend process is still alive.

Two internal Rust tests additionally assert that startup/request deadlines
terminate an unresponsive Python child. The request test explicitly checks
that the child was reaped.

These tests demonstrate process ownership and cleanup, not actual model
synthesis quality, GPU memory instrumentation or Mac mini compatibility.

Verified on 2026-11-07: nine CLI process tests passed in 6.45 seconds; both
internal deadline tests passed in 0.46 seconds. Library/test clippy with
warnings denied and scoped Rust formatting checks also passed.
