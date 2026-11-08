# Personal Files

A native **Rust + GTK 3 application for Ubuntu**, inspired by Finder. Your
favorite, recent, and frequent directories are one click away. Hidden files
are always visible.

## Run

Open **Personal Files** from Ubuntu's application menu after installation, or:

```bash
~/.local/bin/personal-files
~/.local/bin/personal-files /path/to/a/folder
```

From this directory, `./launch.sh` runs the cached release build and builds it
if missing. `./install.sh` builds and installs it for the current user. Restart an already open Personal Files window after upgrading to use the new
version. The installer atomically replaces the executable, so running copies
can finish normally. No sudo
is needed for installation, and it does not replace Ubuntu's default Files app.

## Use

- **Ctrl+T** or **+** opens the current folder in a new tab. Right-click a folder
  or sidebar place for **Open in New Tab**. Each tab keeps its own folder,
  selection, sorting and back/forward history. **Ctrl+Tab / Ctrl+Shift+Tab**
  cycles tabs. **× / Ctrl+W** closes a tab, retaining at least one. Tab order,
  folders, sorting and the active tab restore on restart; runtime selections
  and back/forward history are not restored across restarts.
- Paste a path into the always-visible bar and press **Enter**. Absolute paths,
  `~/…`, and paths relative to the current folder work. A file path opens its
  parent and selects the file. **Ctrl+L** focuses the bar.
- Click **☆** or press **Ctrl+D** to pin/unpin the current folder. Right-click
  folders for **Add to Favorites**, or drag folders into the sidebar. Click a
  saved place to open it; hover to see its full path.
- **Recent** automatically remembers your latest seven places; **Frequent**
  shows five places ranked by visit count and recency. Both survive restart.
  Automatic history never silently changes your manual Favorites.
- Click **Name**, **Modified**, or **Size** to sort; click again to reverse.
  Folders remain first. Their size is shown as “—” rather than a recursive total.
- Select one or more files/folders and use **Ctrl+C**, the Copy button or
  **Copy Full Path** in the context menu. With nothing selected, the Copy button
  copies the current directory. **Ctrl+Shift+C** / **Copy Shell-Quoted Path**
  safely quotes each path for a POSIX terminal. Text copying within the path bar
  retains normal text-entry behavior.
- **Alt+Left/Right** goes back/forward; **Alt+Up** opens the parent.
  **Ctrl+R** or **F5** refreshes. Local folder changes also refresh automatically.
- Double-click or press **Enter** to open a folder/document. Documents use
  Ubuntu's registered default application.
- **Appearance** offers Silver, Midnight, Aurora and Automatic skins, plus a
  background image picker. Automatic uses file extensions, not document content:
  mostly pictures → Aurora; mostly code → Midnight; otherwise Silver.
- **Appearance → Clear Visit History** clears automatic places and keeps your
  explicit favorites. The shortcuts menu provides the keyboard reference.

State is local at `$XDG_CONFIG_HOME/personal-files/preferences.json` (normally
`~/.config/personal-files/preferences.json`). It contains visited paths, favorites,
sort and appearance settings; it is saved atomically with mode 0600. Invalid
state is backed up before recovery. No telemetry, content uploads, or background
network service. Browsing and favoriting do not rename, move or delete files.

## Build and verify

Requires Rust/Cargo and the Ubuntu GTK 3 runtime. A fresh build also needs:

```bash
sudo apt install libgtk-3-dev pkg-config build-essential
./build.sh
cargo test --locked
cargo test --locked desktop_story -- --ignored --nocapture
```

The last command needs a graphical display (or `xvfb-run`). Its fixture windows
and preferences are temporary. Ordinary `cargo test` intentionally ignores that
test; it must be run explicitly for desktop evidence. Physical pointer-driven
drag interoperability is separate from the tested URI/drop handlers.

`build.sh` defaults build output to `~/.cache/personal-files/target` and respects
`CARGO_TARGET_DIR`. On the development machine it can also use GTK development
files extracted under `~/.cache/personal-files/gtk-dev`, without system package
installation. For direct Cargo commands on that machine, export:

```bash
export PKG_CONFIG_PATH="$HOME/.cache/personal-files/gtk-dev/usr/lib/$(gcc -print-multiarch)/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"
export CARGO_TARGET_DIR="$HOME/.cache/personal-files/target"
```

## PDD

The reviewed story is in `user_stories/story__personal_places.md`; versioned
`.prompt` files and `architecture.json` map the implementation. See
`docs/PRODUCT_INTENT.md` for decisions and `docs/VERIFICATION.md` for exact
verification scope and the interrupted automated PDD run. The initial Python
reference was used for porting and is not a maintained project or dependency.

This first iteration targets Ubuntu only. A macOS version is a separate project.
