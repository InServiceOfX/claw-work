# Edit full paths and browse SMB servers on macOS

As a macOS Personal Files user, I want to edit the complete path above the
central directory listing and press Return, so I can alternate between typing
locations and opening folders visually.

As a macOS Personal Files user, I want Finder-style Connect to Server for SMB
addresses, including an IP address and optional username, so I can browse my
network shares in Personal Files and reuse credentials saved by macOS.

Source: Ernest's 2026-11-14 request to retain the Finder Go > Connect to Server
experience (SMB by IP, saved username/password), with editable full-path
navigation and the central GUI directory view. Example below uses a synthetic
address instead of a personal server address.

## Acceptance
1. The full path remains visible and editable near the top. Command–L selects
   it; Return opens an absolute, home-relative or relative path. Clicking a
   directory updates both the listing and full path. Existing file selection,
   back/forward/up, tabs, sorting and favorites continue to work.
2. Entering `smb://tester@192.0.2.10` invokes the native macOS share picker;
   entering `smb://tester@192.0.2.10/share/folder` mounts the share and opens
   that subfolder in the central listing. Escaped spaces and Unicode work.
   If multiple shares are selected, open the first returned mount point.
3. Go > Connect to Server… / Command–K provides an address dialog. Cancel does
   nothing. The same SMB connection behavior is available through the path bar.
4. macOS NetAuth handles authentication, saved Keychain credentials and its
   native option to remember credentials. Personal Files MUST NOT implement a
   password store, read Keychain secrets, log credentials or persist SMB URL
   drafts. Password-bearing URLs are rejected without echoing their contents;
   the path field is cleared back to the current folder.
5. On success, display the actual mounted filesystem path, including its real
   mount name, so subsequent typing, folder opening and normal navigation work.
   Existing mounts are handled by macOS, without guessing `/Volumes/share`.
6. Connecting does not block the main UI thread. Allow at most one outstanding
   mount operation per window. A later navigation/tab switch invalidates its
   result; it MUST NOT replace another tab's view. A completed mount remains
   mounted even if the user has navigated away.
7. Cancellation, unavailable servers, login failure and inaccessible subfolders
   show an error while retaining the prior successful listing, history and saved
   location. Retrying is possible after the outstanding operation completes.
8. Browsing MUST NOT create, rename, move or delete documents. Ubuntu is out of
   scope. Tests use synthetic fixtures and no real servers, passwords or files.

## Evidence scope
Deterministic URL/state tests and native fixture delivery exercise routing and
view updates. Live server authentication, share selection, saved-password reuse
and reconnect remain hands-on checks, without inspecting real credentials.
No generated PDD contract or semantic validation is claimed.
<!-- pdd-story-prompts: prompts/native_rust.prompt, prompts/model_rust.prompt -->
