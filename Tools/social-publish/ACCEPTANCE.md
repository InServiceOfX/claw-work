# Independent acceptance requirements

- Help and offline unittest discovery work without Google packages installed.
- Network and OAuth require explicit opt-in. Upload and publication have separate
  approvals; publication also requires an exact video ID confirmation.
- Before writes, account lookup returns exactly one channel matching the expected
  ID. Video reads also verify channel ownership.
- Upload metadata cannot select privacy. Every upload is private. Publication
  requires a protected local receipt recording private upload readback, a fresh
  private video read, and explicitly succeeded processing.
- Invalid metadata, unknown identities, foreign videos, duplicate content for a
  channel, failed processing, and nonprivate videos fail closed.
- Upload initiation is never retried automatically. A receipt is reserved before
  initiation so an ambiguous failure blocks a duplicate attempt.
- State is outside Git trees, owner-only, atomic, and rejects insecure existing
  files/directories and symlinks. Refreshes are persisted with those protections.
- Provider exceptions, OAuth values, HTTP URLs, paths, and credentials never appear
  in error output. Command results and errors are JSON.
- Tests cover negative gates, protected storage, sanitization, and a mocked happy
  path. Documentation explains permission scope, setup, ambiguity, and lack of
  live verification. No delete, scheduling, comments, or other platforms.
