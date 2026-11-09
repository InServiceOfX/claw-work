# 05 — Connect a harness and complete a tool cycle

As a user, I want to point Pi or Hermes at the local server so I can use the model
through my preferred agent interface.

Status: implemented; scoped evidence and limits in [VERIFICATION.md](../docs/VERIFICATION.md).

- Portable instructions give base URL, model ID and per-client settings without
  embedding credentials or changing global harness configuration automatically.
- At least one actual installed target harness receives a real-model response.
  Label each named harness verified or unverified independently.
- A short no-tool reply with workspace instructions loaded receives a useful
  output budget rather than truncating after one token because of a conservative
  client estimate. The server still rejects an actual context overflow.
- A harmless synthetic tool definition reaches the model in its intended format.
  Generated calls become OpenAI tool calls with names, typed arguments and IDs.
- A follow-up request carries tool results and obtains the model's answer. The
  server translates calls; the harness executes them.
- Malformed calls never become fabricated successful invocations or arbitrary
  execution by the model server.

Evidence: actual-client and real-weight chat plus repeatable synthetic tool round
trip; fragmented/malformed parser cases, typed arguments and multiple calls.

- As conversation history grows across mode switches, Pi uses appropriate
  per-model compaction settings. A true overflow is recognizable by Pi's
  normal recovery flow and includes actual token counts. No server-side
  truncation silently discards user instructions or tool history.
- Test mode switches in one persistent harness session, not only separate
  minimal-prompt processes. Distinguish real inference from protocol fixtures.
