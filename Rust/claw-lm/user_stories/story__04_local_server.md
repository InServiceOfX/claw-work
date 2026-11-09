# 04 — Serve dependable local chat

As a user, I want a separate Rust server with a familiar local chat API so clients
can reuse a loaded model and receive responses and useful errors.

Status: implemented; scoped evidence and limits in [VERIFICATION.md](../docs/VERIFICATION.md).

- An explicit model location starts a loopback server with health, model listing
  and OpenAI-compatible chat completions.
- Non-streaming and SSE preserve text, response identity, finish reason and usage;
  SSE emits its terminal marker. Stream only committed contiguous text.
- Tentative diffusion tokens, partial UTF-8, hidden reasoning and partial stop
  strings do not leak into ordinary content output.
- Malformed input, unknown models, unsupported modalities/options and excessive
  budgets fail before expensive generation.
- Admission is bounded; excess work receives a defined busy response. Requests
  do not share generation state.
- Disconnection/cancellation ends generation; clean shutdown releases model and
  socket resources.

Evidence: synthetic HTTP/SSE tests then real-model smoke tests, fragmented Unicode
and stops, busy admission, independent conversations, cancellation and shutdown.
