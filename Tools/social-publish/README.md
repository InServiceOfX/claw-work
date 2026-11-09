# YouTube publishing CLI

A small Python 3.9+ CLI callable from Pi, Hermes, Claude Code, Codex, Grok, or any
shell-capable harness. It supports desktop OAuth, channel identity, private upload,
video status, and explicitly confirmed public publication. No browser automation,
delete, scheduling, comments, or other platforms are implemented.

**No live OAuth, upload, or publication has been verified.** The implementation is
covered by offline mocks. Upload, update, desktop OAuth, token-expiry and audit
references were checked on 2026-11-06; this is not a claim of live API conformance.
See [platform feasibility and TikTok trade-offs](../../docs/SOCIAL_PUBLISHING_FEASIBILITY.md).

## Local setup (operator performed)

Run from this directory. Use Python **3.11 or newer** for a supported Google SDK
runtime (the CLI's offline logic also tests on 3.9). Dependency installation is a
separate operator action:

```sh
python3.11 -m venv .venv
.venv/bin/python -m pip install -r requirements.txt
.venv/bin/python youtube_publish.py --help
```

The Google packages and their transitive authentication/HTTP dependencies are lazy
imports; help and tests need only Python. Dependency ranges are not a lockfile.
Protected storage currently requires POSIX permissions (macOS/Linux); Windows
account operations fail closed pending an ACL implementation. The default state
path is `~/Library/Application Support/social-publish` on macOS and
`${XDG_CONFIG_HOME:-$HOME/.config}/social-publish` on Linux. An explicit `--state-dir`
may be used. State and client configuration must be outside all Git trees.

1. In a Google Cloud project, enable **YouTube Data API v3**.
2. Configure the OAuth consent screen/audience and add the intended account as a
   test user when the app is in testing. Select only the scopes you need.
3. Create an OAuth client of type **Desktop app**, and download its JSON directly
   into a private directory outside every repository. Never copy it into this
   repository, including ignored directories. This CLI accepts Google's desktop
   auth/token endpoints only.
4. Use owner-only directory/file permissions: `0700` for the selected state
   directory and `0600` for the downloaded client JSON. State files are created
   atomically as `0600`. Insecure existing state directories/files, hard-linked
   state files, symlinks (including path ancestors), and writable parent directories
   without a sticky bit are rejected. On systems with aliases such as `/tmp`, use
   the actual nonsymlink path. Keep all ancestors under trusted ownership; this is
   local filesystem protection, not a defense against a compromised OS or account.
5. Obtain the intended channel's actual ID independently. Supply it on **every**
   account command. The CLI refuses zero, multiple, paginated, or mismatched
   `channels.list(mine=true)` results. An unexpected account or Brand Account choice
   requires another explicit login with the correct selection.

Official references: [Python quickstart](https://developers.google.com/youtube/v3/quickstart/python),
[desktop OAuth](https://developers.google.com/identity/protocols/oauth2/native-app),
[OAuth consent setup](https://developers.google.com/workspace/guides/configure-oauth-consent),
[channels.list](https://developers.google.com/youtube/v3/docs/channels/list).

## Permissions and login

All network operations require `--allow-network`. This also approves token refresh
and protected token persistence. `auth` additionally requires `--allow-auth`;
`upload` requires `--allow-upload`; `publish` requires `--allow-publish` and an
exact matching `--confirm-video-id`. A harness must obtain the user's approval
before adding those flags. The flags cannot establish whether a human approved.

Auth permission choices accumulate these OAuth scopes:

| Permission | Scopes | Capability granted to the OAuth client |
| --- | --- | --- |
| `readonly` (default) | `youtube.readonly` | Read account/channel/video information |
| `upload` | readonly + `youtube.upload` | Also upload/manage videos as allowed by the scope |
| `publish` | readonly + upload + `youtube.force-ssl` | Broader editing, deletion and comment capabilities at Google; this CLI exposes publication only |

Use the smallest grant needed. A read-only grant cannot upload; an upload-only
grant cannot publish through this CLI. Reauthorize with `--permission publish`
when publication is intended. Login opens the system browser and receives the
callback on an ephemeral `127.0.0.1` loopback port, forces account selection and
consent, then checks channel identity before saving credentials. Login replaces
the token for this state directory; keep receipts when reauthorizing. Each state
directory has one active token. Never print or pass token values in command lines.

OAuth refresh tokens for external apps in **Testing** can expire after seven days
for these scopes; other expiry/revocation conditions also apply. Reauthorize if
needed. OAuth consent verification is distinct from the **YouTube API compliance
audit**. On 2026-11-06, the official `videos.insert` reference explicitly states
that videos uploaded from unverified API projects are **not** restricted to
private viewing. Older blanket private-only audit guidance is not current on
that page. This CLI still explicitly uploads privately for safety. OAuth consent,
account restrictions, quota, API compliance and copyright remain separate; a
successful login or upload is not a guarantee of publication eligibility.

References: [scopes](https://developers.google.com/identity/protocols/oauth2/scopes#youtube),
[token expiration](https://developers.google.com/identity/protocols/oauth2#expiration),
[videos.insert restrictions](https://developers.google.com/youtube/v3/docs/videos/insert),
[compliance audits](https://developers.google.com/youtube/v3/guides/quota_and_compliance_audits).

## Harness-neutral examples

IDs below are synthetic; replace them with independently verified real IDs. Paths
refer to an operator-controlled private directory outside repositories. The CLI
does not create or copy a client config or source video. Review metadata for
private information and select the correct made-for-kids designation yourself.
All five metadata fields are required; unknown fields (including privacy fields)
are rejected. Category IDs are numeric strings, validated locally for shape;
YouTube determines whether a category is valid for the account/region.

```sh
.venv/bin/python youtube_publish.py auth --allow-network --allow-auth \
  --permission upload --expected-channel-id UCaaaaaaaaaaaaaaaaaaaaaa \
  --client-config "$HOME/private-social/client.json" \
  --state-dir "$HOME/private-social/state"

.venv/bin/python youtube_publish.py channel --allow-network \
  --expected-channel-id UCaaaaaaaaaaaaaaaaaaaaaa \
  --state-dir "$HOME/private-social/state"

.venv/bin/python youtube_publish.py upload --allow-network --allow-upload \
  --expected-channel-id UCaaaaaaaaaaaaaaaaaaaaaa \
  --state-dir "$HOME/private-social/state" \
  --file "$HOME/private-social/demo.mp4" \
  --metadata "$HOME/private-social/metadata.json"

.venv/bin/python youtube_publish.py status --allow-network \
  --expected-channel-id UCaaaaaaaaaaaaaaaaaaaaaa --video-id aaaaaaaaaaa \
  --state-dir "$HOME/private-social/state"

# After processing succeeds and reauthorizing with --permission publish:
.venv/bin/python youtube_publish.py publish --allow-network --allow-publish \
  --expected-channel-id UCaaaaaaaaaaaaaaaaaaaaaa --video-id aaaaaaaaaaa \
  --confirm-video-id aaaaaaaaaaa --state-dir "$HOME/private-social/state"
```

Commands return a JSON object with `ok` and `result` or a sanitized `error` code;
failures exit 1. Help is normal argparse text and exits 0. Provider messages,
request URLs, paths, credentials, and tracebacks are withheld; third-party output
and Python logging are suppressed during operations. Results contain only IDs,
a canonical watch URL and allowlisted status values. Treat results and receipts as
private account data. If saving output, use an owner-only file outside repositories.
There is no CLI debug mode or persistent diagnostic log.

Upload always sends `privacyStatus=private`, irrespective of metadata. Success
returns the video ID, URL, upload/processing state and privacy after channel-bound
readback. Publication requires a local receipt recording private upload readback,
a fresh matching video owned by the expected channel, private visibility,
`processingStatus=succeeded` and `uploadStatus=processed`. It changes only privacy
to public, preserves the other writable status properties, and reads it back.
Already-scheduled videos require manual review instead of implicitly cancelling
their schedule. Status never changes video privacy. There is no
polling or automatic publication. Missing processing details fail closed.

References: [video resource](https://developers.google.com/youtube/v3/docs/videos),
[videos.list](https://developers.google.com/youtube/v3/docs/videos/list),
[videos.update](https://developers.google.com/youtube/v3/docs/videos/update).

## Duplicate and failure handling

A channel + SHA256 receipt is reserved **before** starting a resumable upload. It
records acceptance and private readback in successive atomic writes. Repeating
identical bytes for that channel is refused, regardless of filename or metadata.
There is no override, receipt import, or automatic recovery command. Use immutable
input files; the same open media stream is hashed and uploaded, but editing its
contents in place during upload is unsupported.

Upload initiation/chunks have no automatic API retries; transport replay after a
401 is disabled too. A network error or interruption can occur **after Google has
accepted the upload**. Inspect YouTube Studio before retrying. Do not blindly
remove a reserved receipt or use another state directory to bypass it. If acceptance
or readback fails, the receipt intentionally prevents publication through this CLI,
even if a video exists. Resolve such cases manually in Studio. Publication failure
can also be ambiguous: use status/Studio before another attempt.

A per-state exclusive lock serializes authentication, refresh, and account
operations. A process crash may leave `operation.lock`; only after confirming that
no process is active should the operator remove that lock. Receipts remain intact.
Keep this state directory backed up privately: duplicate protection is local to it,
not global. Copies of content with different bytes are different hashes. Do not
share tokens/receipts across untrusted users. Local filesystem storage is not
encrypted by this tool. No guarantees are made about changes another client makes
between API checks and writes.

## Offline acceptance

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s . -v
PYTHONDONTWRITEBYTECODE=1 python3 youtube_publish.py --help
```

Tests use only temporary synthetic data and mocked services. They exercise approval
and identity gates, metadata, duplicate handling, private upload/publication,
processing, ownership, storage permissions/symlinks, atomic failure, and error
sanitization. They do not validate actual Google responses, account eligibility,
OAuth consent, resumable uploads, or audit status. Independent review and an
operator-approved private live test are required before relying on this tool.
