//! AppKit bridge. Objective-C stays here; AppKit runs on the initial thread.
//! Foundation/NetFS connection work uses a bounded background worker.
//! Selectors/signatures follow the macOS SDK headers. Owned
//! roots (window/delegate) live until event-loop shutdown; views/menus are owned
//! by their AppKit parents. No callback holds a model borrow across reloadData,
//! which synchronously re-enters the table's data source.
use objc2::{
    class,
    encode::{Encode, Encoding},
    msg_send,
    rc::autoreleasepool,
    runtime::{AnyClass, AnyObject, AnyProtocol, Bool, ClassBuilder, Sel},
    sel,
};
use personal_files::{
    appearance,
    core::{self, Preferences, Sort},
    model::{Browser, TabbedBrowser},
};
use std::{
    cell::{Cell, RefCell},
    ffi::{CStr, CString},
    path::{Path, PathBuf},
    ptr,
    rc::Rc,
    sync::mpsc::{self, Receiver},
    time::{Duration, Instant},
};

#[path = "native_desktop.rs"]
mod desktop_tests;
#[path = "native_tiling_test.rs"]
mod tiling_test;

type Id = *mut AnyObject;
const NIL: Id = ptr::null_mut();
#[link(name = "AppKit", kind = "framework")]
unsafe extern "C" {
    static NSAppearanceNameAqua: Id;
    static NSAppearanceNameDarkAqua: Id;
}
#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    static NSDefaultRunLoopMode: Id;
    static NSRunLoopCommonModes: Id;
}
#[link(name = "NetFS", kind = "framework")]
unsafe extern "C" {
    fn NetFSMountURLSync(
        url: Id,
        mountpath: Id,
        user: Id,
        password: Id,
        open_options: Id,
        mount_options: Id,
        mountpoints: *mut Id,
    ) -> i32;
}

#[derive(Debug, PartialEq)]
struct ServerLocation {
    mount_url: String,
    relative: PathBuf,
}

/// Foundation parses/encodes URLs; passwords are accepted only by macOS's context().ui.
fn server_location(text: &str) -> Result<ServerLocation, String> {
    let invalid = || {
        "Enter an SMB address such as smb://user@server/share/folder (without a password, query or fragment).".to_string()
    };
    if text.chars().any(char::is_control) {
        return Err(invalid());
    }
    // Older Foundation parsing can double-escape existing %XX when the same
    // input contains raw Unicode. Encode only raw spaces/non-ASCII first.
    let encoded: String = text
        .trim()
        .bytes()
        .map(|b| {
            if b >= 128 || b == b' ' {
                format!("%{b:02X}")
            } else {
                char::from(b).to_string()
            }
        })
        .collect();
    unsafe {
        let c: Id = msg_send![class!(NSURLComponents),componentsWithString:ns(&encoded)];
        if c.is_null() {
            return Err(invalid());
        }
        let scheme: Id = msg_send![c, scheme];
        let host: Id = msg_send![c, host];
        let password: Id = msg_send![c, password];
        let query: Id = msg_send![c, query];
        let fragment: Id = msg_send![c, fragment];
        if !string(scheme).eq_ignore_ascii_case("smb")
            || string(host).is_empty()
            || !password.is_null()
            || !query.is_null()
            || !fragment.is_null()
        {
            return Err(invalid());
        }
        let path: Id = msg_send![c, path];
        let bytes: usize = msg_send![path,lengthOfBytesUsingEncoding:4usize];
        let path = string(path);
        if bytes != path.len() {
            return Err(invalid());
        }
        let parts: Vec<_> = path.split('/').filter(|s| !s.is_empty()).collect();
        if parts
            .iter()
            .any(|s| *s == "." || *s == ".." || s.contains('\\') || s.chars().any(char::is_control))
        {
            return Err(invalid());
        }
        let share = parts.first().map_or(String::new(), |s| format!("/{s}"));
        let _: () = msg_send![c,setScheme:ns("smb")];
        let _: () = msg_send![c,setPath:ns(&share)];
        let url: Id = msg_send![c, URL];
        if url.is_null() {
            return Err(invalid());
        }
        let absolute: Id = msg_send![url, absoluteString];
        Ok(ServerLocation {
            mount_url: string(absolute),
            relative: parts.iter().skip(1).collect(),
        })
    }
}

fn mount_server(location: ServerLocation) -> Result<PathBuf, String> {
    autoreleasepool(|_| unsafe {
        let url: Id = msg_send![class!(NSURL),URLWithString:ns(&location.mount_url)];
        let options: Id = msg_send![class!(NSMutableDictionary), dictionary];
        let _: () = msg_send![options,setObject:ns("AllowUI"),forKey:ns("UIOption")];
        let mut points = NIL;
        // Null user/password lets NetAuth use the URL username, saved Keychain
        // credentials and its own authentication / Remember Password dialog.
        let status = NetFSMountURLSync(url, NIL, NIL, NIL, options, NIL, &mut points);
        let count: usize = if points.is_null() {
            0
        } else {
            msg_send![points, count]
        };
        let result = if (status == 0 || status == 17) && count > 0 {
            let point: Id = msg_send![points,objectAtIndex:0usize];
            Ok(PathBuf::from(string(point)).join(location.relative))
        } else if status == -128 {
            Err("Server connection cancelled.".into())
        } else {
            Err(format!("Could not connect to server (macOS error {status}). Check the address, network and login, then try again."))
        };
        // NetFS transfers ownership of the CFArray; NSArray is toll-free bridged.
        if !points.is_null() {
            let _: () = msg_send![points, release];
        }
        result
    })
}

type ConnectionReply = (u64, Result<PathBuf, String>);

#[repr(C)]
#[derive(Clone, Copy)]
struct Point {
    x: f64,
    y: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Size {
    width: f64,
    height: f64,
}
#[repr(C)]
#[derive(Clone, Copy)]
struct Rect {
    origin: Point,
    size: Size,
}
// SAFETY: repr(C), same CGFloat layout and encodings as the 64-bit SDK.
unsafe impl Encode for Point {
    const ENCODING: Encoding = Encoding::Struct("CGPoint", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for Size {
    const ENCODING: Encoding = Encoding::Struct("CGSize", &[f64::ENCODING, f64::ENCODING]);
}
unsafe impl Encode for Rect {
    const ENCODING: Encoding = Encoding::Struct("CGRect", &[Point::ENCODING, Size::ENCODING]);
}
fn rect(x: f64, y: f64, w: f64, h: f64) -> Rect {
    Rect {
        origin: Point { x, y },
        size: Size {
            width: w,
            height: h,
        },
    }
}
#[derive(Clone, Copy)]
struct Widgets {
    window: Id,
    path: Id,
    table: Id,
    sidebar: Id,
    status: Id,
    star: Id,
    back: Id,
    forward: Id,
    image: Id,
    delegate: Id,
    tabs: Id,
    new_tab: Id,
    close_tab: Id,
}
#[derive(Clone)]
struct Place {
    label: String,
    path: Option<PathBuf>,
}
#[derive(Clone, Default)]
struct Display {
    rows: Vec<[String; 3]>,
    paths: Vec<PathBuf>,
    places: Vec<Place>,
}
// Each callback selects its window's state for its entire dynamic extent.
// Nested AppKit callbacks restore the caller's state on return.
struct Slot<T>(RefCell<T>);
impl<T> Slot<T> {
    fn new(value: T) -> Self {
        Self(RefCell::new(value))
    }
    fn with_borrow<R>(&self, f: impl FnOnce(&T) -> R) -> R {
        f(&self.0.borrow())
    }
    fn with_borrow_mut<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.0.borrow_mut())
    }
}
struct WindowState {
    connection: Slot<Option<Receiver<ConnectionReply>>>,
    model: Slot<Option<TabbedBrowser>>,
    ui: Cell<Option<Widgets>>,
    rendering: Cell<bool>,
    syncing_tabs: Cell<bool>,
    displayed_location: Slot<Option<PathBuf>>,
    display: Slot<Display>,
    timer: Cell<Id>,
    closed: Cell<bool>,
    shared_dirty: Cell<bool>,
    preferences_path: PathBuf,
}
impl WindowState {
    fn new(preferences_path: PathBuf) -> Self {
        Self {
            connection: Slot::new(None),
            model: Slot::new(None),
            ui: Cell::new(None),
            rendering: Cell::new(false),
            syncing_tabs: Cell::new(false),
            displayed_location: Slot::new(None),
            display: Slot::new(Display::default()),
            timer: Cell::new(NIL),
            closed: Cell::new(false),
            shared_dirty: Cell::new(false),
            preferences_path,
        }
    }
}
thread_local! {
    static WINDOWS: RefCell<Vec<Rc<WindowState>>> = const { RefCell::new(Vec::new()) };
    static CURRENT: RefCell<Option<Rc<WindowState>>> = const { RefCell::new(None) };
    static FOCUSED: Cell<Id> = const { Cell::new(NIL) };
    // Objective-C roots retain their existing application-lifetime ownership.
    static OWNED: RefCell<Vec<Id>> = const { RefCell::new(Vec::new()) };
}
fn context() -> Rc<WindowState> {
    CURRENT.with_borrow(|c| c.clone().expect("window context initialized"))
}
fn share_preferences(from: &WindowState, to: &WindowState) {
    if std::ptr::eq(from, to) {
        return;
    }
    from.model.with_borrow(|source| {
        to.model.with_borrow_mut(|destination| {
            if let (Some(source), Some(destination)) = (source, destination) {
                let src = &source.browser.prefs.data;
                let dst = &mut destination.browser.prefs.data;
                let changed = dst.favorites != src.favorites
                    || dst.theme != src.theme
                    || dst.image != src.image
                    || dst.visits.len() != src.visits.len()
                    || src.visits.iter().any(|(path, visit)| {
                        dst.visits
                            .get(path)
                            .is_none_or(|old| old.count != visit.count || old.last != visit.last)
                    });
                if changed {
                    to.shared_dirty.set(true);
                }
                dst.favorites = src.favorites.clone();
                dst.visits = src.visits.clone();
                dst.theme = src.theme.clone();
                dst.image = src.image.clone();
            }
        })
    });
}
struct WindowGuard(Option<Rc<WindowState>>);
impl Drop for WindowGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.0.take() {
            let current = context();
            let destination = if previous.closed.get() {
                if !current.closed.get() {
                    current.clone()
                } else {
                    WINDOWS
                        .with_borrow(|windows| windows.iter().find(|s| !s.closed.get()).cloned())
                        .unwrap_or(previous)
                }
            } else {
                previous
            };
            share_preferences(&current, &destination);
            CURRENT.with_borrow_mut(|slot| *slot = Some(destination));
        }
    }
}
fn enter_window(object: &AnyObject, sender: Id) -> WindowGuard {
    let previous = CURRENT.with_borrow(Clone::clone);
    let object = object as *const AnyObject as Id;
    let menu = !sender.is_null()
        && unsafe {
            let result: Bool = msg_send![sender,isKindOfClass:class!(NSMenuItem)];
            result.as_bool()
        };
    let focused = if menu { FOCUSED.get() } else { NIL };
    let target = WINDOWS.with_borrow(|windows| {
        windows
            .iter()
            .find(|state| {
                (!menu || !state.closed.get())
                    && state.ui.get().is_some_and(|w| {
                        if menu {
                            w.delegate == focused
                        } else {
                            w.delegate == object || w.table == object
                        }
                    })
            })
            .or_else(|| {
                if menu {
                    windows.iter().find(|s| !s.closed.get())
                } else {
                    None
                }
            })
            .cloned()
    });
    if let Some(target) = target {
        if let Some(previous) = &previous {
            share_preferences(previous, &target);
        }
        CURRENT.with_borrow_mut(|slot| *slot = Some(target));
    }
    WindowGuard(previous)
}

fn widgets() -> Widgets {
    context().ui.get().expect("AppKit initialized")
}
fn tabs<T>(f: impl FnOnce(&mut TabbedBrowser) -> T) -> T {
    context()
        .model
        .with_borrow_mut(|m| f(m.as_mut().expect("model initialized")))
}
fn model<T>(f: impl FnOnce(&mut Browser) -> T) -> T {
    context()
        .model
        .with_borrow_mut(|m| f(&mut m.as_mut().expect("model initialized").browser))
}
/// Own an Objective-C +1 result until app shutdown. Autoreleased values are not
/// put here. Parent views retain children independently.
fn own(object: Id) -> Id {
    assert!(!object.is_null(), "AppKit allocation failed");
    OWNED.with_borrow_mut(|v| v.push(object));
    object
}
fn ns(text: &str) -> Id {
    let c = CString::new(text.replace('\0', "�")).unwrap();
    unsafe { msg_send![class!(NSString), stringWithUTF8String:c.as_ptr()] }
}
fn string(object: Id) -> String {
    if object.is_null() {
        return String::new();
    }
    unsafe {
        let p: *const std::ffi::c_char = msg_send![object, UTF8String];
        if p.is_null() {
            String::new()
        } else {
            CStr::from_ptr(p).to_string_lossy().into_owned()
        }
    }
}
fn value(field: Id) -> String {
    unsafe {
        let s: Id = msg_send![field, stringValue];
        string(s)
    }
}
fn set_value(field: Id, text: &str) {
    unsafe {
        let _: () = msg_send![field,setStringValue:ns(text)];
    }
}
fn array(objects: &[Id]) -> Id {
    unsafe { msg_send![class!(NSArray),arrayWithObjects:objects.as_ptr(),count:objects.len()] }
}
fn app() -> Id {
    unsafe { msg_send![class!(NSApplication), sharedApplication] }
}
fn file_url(path: &Path) -> Id {
    unsafe { msg_send![class!(NSURL),fileURLWithPath:ns(&path.to_string_lossy())] }
}
fn frame(view: Id, r: Rect, mask: usize) {
    unsafe {
        let _: () = msg_send![view,setFrame:r];
        let _: () = msg_send![view,setAutoresizingMask:mask];
    }
}
fn add(parent: Id, child: Id) {
    unsafe {
        let _: () = msg_send![parent,addSubview:child];
    }
}
fn label(text: &str) -> Id {
    unsafe { msg_send![class!(NSTextField),labelWithString:ns(text)] }
}
fn button(parent: Id, title: &str, action: Sel, r: Rect, mask: usize) -> Id {
    unsafe {
        let b: Id = msg_send![class!(NSButton),buttonWithTitle:ns(title),target:widgets().delegate,action:action];
        frame(b, r, mask);
        let _: () = msg_send![b,setBezelStyle:1usize];
        add(parent, b);
        b
    }
}
fn item(menu: Id, title: &str, action: Sel, key: &str, modifiers: usize, target: Id) -> Id {
    unsafe {
        let i = own(msg_send![class!(NSMenuItem), alloc]);
        let i: Id = msg_send![i,initWithTitle:ns(title),action:action,keyEquivalent:ns(key)];
        let _: () = msg_send![i,setTarget:target];
        let _: () = msg_send![i,setKeyEquivalentModifierMask:modifiers];
        let _: () = msg_send![menu,addItem:i];
        i
    }
}
fn new_menu(title: &str) -> Id {
    unsafe {
        let m: Id = msg_send![class!(NSMenu), alloc];
        own(msg_send![m,initWithTitle:ns(title)])
    }
}
fn submenu(root: Id, title: &str) -> Id {
    let m = new_menu(title);
    let i = item(root, title, sel!(noop:), "", 0, NIL);
    unsafe {
        // A submenu wrapper is not a command. A bogus action makes AppKit's
        // validation disable it and can swallow otherwise valid key equivalents.
        let _: () = msg_send![i,setAction:None::<Sel>];
        let _: () = msg_send![i,setSubmenu:m];
    }
    m
}
fn menu_bar(delegate: Id) {
    const CMD: usize = 1 << 20;
    const SHIFT: usize = 1 << 17;
    const ALT: usize = 1 << 19;
    let root = new_menu("Personal Files");
    let application = submenu(root, "Personal Files");
    item(
        application,
        "About Personal Files",
        sel!(about:),
        "",
        0,
        delegate,
    );
    item(
        application,
        "Quit Personal Files",
        sel!(terminate:),
        "q",
        CMD,
        app(),
    );
    let file = submenu(root, "File");
    item(file, "New Window", sel!(newWindow:), "n", CMD, delegate);
    item(file, "New Tab", sel!(newTab:), "t", CMD, delegate);
    item(file, "Open Selection", sel!(open:), "o", CMD, delegate);
    item(
        file,
        "Pin / Unpin Current Folder",
        sel!(favorite:),
        "d",
        CMD,
        delegate,
    );
    item(file, "Close Tab", sel!(closeTab:), "w", CMD, delegate);
    let edit = submenu(root, "Edit");
    // Native responder chain preserves text-field editing/copying.
    item(edit, "Cut", sel!(cut:), "x", CMD, NIL);
    item(edit, "Copy", sel!(copy:), "c", CMD, NIL);
    item(edit, "Paste", sel!(paste:), "v", CMD, NIL);
    item(edit, "Select All", sel!(selectAll:), "a", CMD, NIL);
    item(
        edit,
        "Copy Shell-Quoted Path",
        sel!(copyShell:),
        "c",
        CMD | SHIFT,
        delegate,
    );
    let go = submenu(root, "Go");
    item(
        go,
        "Connect to Server…",
        sel!(connectServer:),
        "k",
        CMD,
        delegate,
    );
    item(go, "Focus Path", sel!(focus:), "l", CMD, delegate);
    item(go, "Back", sel!(back:), "[", CMD, delegate);
    item(go, "Forward", sel!(forward:), "]", CMD, delegate);
    item(go, "Parent Folder", sel!(up:), "\u{f700}", CMD, delegate);
    item(
        go,
        "Back (Option–Left)",
        sel!(back:),
        "\u{f702}",
        ALT,
        delegate,
    );
    item(
        go,
        "Forward (Option–Right)",
        sel!(forward:),
        "\u{f703}",
        ALT,
        delegate,
    );
    item(go, "Refresh", sel!(refresh:), "r", CMD, delegate);
    let appearance = submenu(root, "Appearance");
    for (title, tag) in [
        ("Silver", 0isize),
        ("Midnight", 1),
        ("Aurora", 2),
        ("Automatic", 3),
    ] {
        let i = item(appearance, title, sel!(theme:), "", 0, delegate);
        unsafe {
            let _: () = msg_send![i,setTag:tag];
        }
    }
    item(
        appearance,
        "Choose Background Image…",
        sel!(chooseImage:),
        "",
        0,
        delegate,
    );
    item(
        appearance,
        "Remove Background Image",
        sel!(removeImage:),
        "",
        0,
        delegate,
    );
    item(
        appearance,
        "Clear Visit History",
        sel!(clearHistory:),
        "",
        0,
        delegate,
    );
    let window = submenu(root, "Window");
    item(
        window,
        "Next Tab",
        sel!(nextTab:),
        "\t",
        1usize << 18,
        delegate,
    );
    item(
        window,
        "Previous Tab",
        sel!(previousTab:),
        "\t",
        (1usize << 18) | SHIFT,
        delegate,
    );
    item(window, "Minimize", sel!(performMiniaturize:), "m", CMD, NIL);
    item(window, "Zoom", sel!(performZoom:), "", 0, NIL);
    item(
        window,
        "Bring All to Front",
        sel!(arrangeInFront:),
        "",
        0,
        app(),
    );
    let help = submenu(root, "Help");
    item(
        help,
        "Keyboard Shortcuts",
        sel!(shortcuts:),
        "",
        0,
        delegate,
    );
    unsafe {
        let _: () = msg_send![app(),setMainMenu:root];
        // AppKit injects its OS-version-specific Fill/Center/Move & Resize
        // items and Fn-Control key equivalents only into the registered menu.
        let _: () = msg_send![app(),setWindowsMenu:window];
    }
}
fn table(parent: Id, r: Rect, sidebar: bool, delegate: Id) -> Id {
    unsafe {
        let sc: Id = msg_send![class!(NSScrollView), alloc];
        let sc = own(msg_send![sc,initWithFrame:r]);
        frame(sc, r, if sidebar { 16 } else { 2 | 16 });
        let _: () = msg_send![sc,setHasVerticalScroller:Bool::YES];
        let _: () = msg_send![sc,setHasHorizontalScroller:Bool::new(!sidebar)];
        let _: () = msg_send![sc,setAutohidesScrollers:Bool::YES];
        let _: () = msg_send![sc,setDrawsBackground:Bool::NO];
        let cls = if sidebar {
            class!(NSTableView)
        } else {
            AnyClass::get(c"PFFileTable").unwrap()
        };
        let t: Id = msg_send![cls, alloc];
        let t = own(msg_send![t,initWithFrame:rect(0.,0.,r.size.width,r.size.height)]);
        let _: () = msg_send![t,setDelegate:delegate];
        let _: () = msg_send![t,setDataSource:delegate];
        let _: () = msg_send![t,setRowHeight:27f64];
        let _: () = msg_send![t,setAllowsMultipleSelection:Bool::new(!sidebar)];
        let _: () = msg_send![t,setColumnAutoresizingStyle:1usize];
        let _: () = msg_send![t,setTarget:delegate];
        if sidebar {
            let _: () = msg_send![t,setHeaderView:NIL];
            let _: () = msg_send![t,setAction:sel!(sidebar:)];
            let _: () = msg_send![t,setStyle:3isize];
        } else {
            let _: () = msg_send![t,setDoubleAction:sel!(open:)];
        }
        for (id, title, width) in if sidebar {
            vec![("place", "Place", 220.)]
        } else {
            vec![
                ("name", "Name", 430.),
                ("modified", "Modified", 180.),
                ("size", "Size", 100.),
            ]
        } {
            let col: Id = msg_send![class!(NSTableColumn), alloc];
            let col = own(msg_send![col,initWithIdentifier:ns(id)]);
            let _: () = msg_send![col,setTitle:ns(title)];
            let _: () = msg_send![col,setWidth:width];
            let _: () = msg_send![col,setMinWidth:if id=="name"{180f64}else{80f64}];
            let _: () = msg_send![t,addTableColumn:col];
        }
        let _: () = msg_send![sc,setDocumentView:t];
        add(parent, sc);
        if sidebar {
            let _: () = msg_send![t,registerForDraggedTypes:array(&[ns("public.file-url")])];
        }
        let _: () = msg_send![t,setDraggingSourceOperationMask:1usize,forLocal:Bool::NO];
        t
    }
}
fn setup(prefs: Preferences, start: Option<PathBuf>) -> Id {
    register_classes();
    let state = Rc::new(WindowState::new(prefs.path.clone()));
    WINDOWS.with_borrow_mut(|windows| windows.push(state.clone()));
    CURRENT.with_borrow_mut(|slot| *slot = Some(state));
    unsafe {
        let policy: isize = msg_send![app(), activationPolicy];
        if policy != 0 {
            let active: Bool = msg_send![app(),setActivationPolicy:0isize];
            assert!(active.as_bool(), "Could not activate desktop application. A logged-in macOS graphical session is required.");
        }
        let delegate = own(msg_send![AnyClass::get(c"PFDelegate").unwrap(), new]);
        let w: Id = msg_send![class!(NSWindow), alloc];
        let w = own(
            msg_send![w,initWithContentRect:rect(0.,0.,1120.,720.),styleMask:15usize,backing:2usize,defer:Bool::NO],
        );
        let _: () = msg_send![w,setReleasedWhenClosed:Bool::NO];
        let _: () = msg_send![w,setTitle:ns("Personal Files")];
        let _: () = msg_send![w,setDelegate:delegate];
        let _: () = msg_send![w,setContentMinSize:Size{width:480.,height:280.}];
        let _: () = msg_send![w,setCollectionBehavior:(1usize<<7)|(1usize<<11)];
        // Our in-window NSTabView owns folder tabs, not automatic NSWindow tabs.
        let _: () = msg_send![w,setTabbingMode:2isize];
        let content: Id = msg_send![w, contentView];
        context().ui.set(Some(Widgets {
            window: w,
            path: NIL,
            table: NIL,
            sidebar: NIL,
            status: NIL,
            star: NIL,
            back: NIL,
            forward: NIL,
            image: NIL,
            delegate,
            tabs: NIL,
            new_tab: NIL,
            close_tab: NIL,
        }));
        let tab_view: Id = msg_send![class!(NSTabView), alloc];
        let tab_view = own(msg_send![tab_view,initWithFrame:rect(8.,682.,1024.,30.)]);
        let _: () = msg_send![tab_view,setAllowsTruncatedLabels:Bool::YES];
        let _: () = msg_send![tab_view,setDelegate:delegate];
        add(content, tab_view);
        let new_tab = button(content, "＋", sel!(newTab:), rect(1044., 684., 30., 28.), 0);
        let close_tab = button(
            content,
            "×",
            sel!(closeTab:),
            rect(1078., 684., 30., 28.),
            0,
        );
        // Native macOS density, system font and source-list selection. The path
        // bar is intentionally prominent rather than hidden behind a Go menu.
        let back = button(content, "‹", sel!(back:), rect(12., 672., 34., 30.), 8);
        let forward = button(content, "›", sel!(forward:), rect(49., 672., 34., 30.), 8);
        let up = button(content, "↑", sel!(up:), rect(86., 672., 34., 30.), 8);
        let _: () = msg_send![up,setTag:101isize];
        let refresh = button(content, "↻", sel!(refresh:), rect(123., 672., 34., 30.), 8);
        let _: () = msg_send![refresh,setTag:102isize];
        let path: Id = msg_send![class!(NSTextField), alloc];
        let path = own(msg_send![path,initWithFrame:rect(169.,674.,728.,26.)]);
        frame(path, rect(169., 674., 728., 26.), 2 | 8);
        let _: () = msg_send![path,setTarget:delegate];
        let _: () = msg_send![path,setAction:sel!(navigate:)];
        let _: () = msg_send![path,setPlaceholderString:ns("Enter a folder, file path or smb://server/share")];
        let _: () = msg_send![path,setAccessibilityLabel:ns("Folder path or SMB address")];
        add(content, path);
        let star = button(
            content,
            "☆",
            sel!(favorite:),
            rect(905., 672., 42., 30.),
            1 | 8,
        );
        let copy = button(
            content,
            "Copy",
            sel!(copyPath:),
            rect(953., 672., 108., 30.),
            1 | 8,
        );
        let _: () = msg_send![copy,setTag:103isize];
        let appearance = button(
            content,
            "◐",
            sel!(appearance:),
            rect(1067., 672., 40., 30.),
            1 | 8,
        );
        let _: () = msg_send![appearance,setTag:104isize];
        let image: Id = msg_send![class!(NSImageView), alloc];
        let image = own(msg_send![image,initWithFrame:rect(252.,36.,856.,624.)]);
        frame(image, rect(252., 36., 856., 624.), 2 | 16);
        let _: () = msg_send![image,setImageScaling:3usize];
        let _: () = msg_send![image,setAlphaValue:0.22f64];
        add(content, image);
        let sidebar = table(content, rect(8., 36., 236., 624.), true, delegate);
        let files = table(content, rect(252., 36., 856., 624.), false, delegate);
        let status = label("");
        frame(status, rect(14., 8., 1092., 20.), 2);
        add(content, status);
        let font: Id = msg_send![class!(NSFont),systemFontOfSize:11f64];
        let _: () = msg_send![status,setFont:font];
        context().ui.set(Some(Widgets {
            window: w,
            path,
            table: files,
            sidebar,
            status,
            star,
            back,
            forward,
            image,
            delegate,
            tabs: tab_view,
            new_tab,
            close_tab,
        }));
        layout();
        let fm = new_menu("Files");
        for (title, action) in [
            ("Open", sel!(open:)),
            ("Copy Full Path", sel!(copyPath:)),
            ("Copy Shell-Quoted Path", sel!(copyShell:)),
            ("Add Folders to Favorites", sel!(addFavorites:)),
        ] {
            item(fm, title, action, "", 0, delegate);
        }
        let _: () = msg_send![files,setMenu:fm];
        let sm = new_menu("Places");
        item(
            sm,
            "Remove Favorite",
            sel!(removeFavorite:),
            "",
            0,
            delegate,
        );
        let _: () = msg_send![sidebar,setMenu:sm];
        if WINDOWS.with_borrow(|windows| windows.len()) == 1 {
            menu_bar(delegate);
        }
        FOCUSED.set(delegate);
        context()
            .model
            .with_borrow_mut(|m| *m = Some(TabbedBrowser::new(Browser::new(prefs, start))));
        render();
        let _: () = msg_send![w, center];
        let running: Bool = msg_send![app(), isRunning];
        if !running.as_bool() {
            let _: () = msg_send![app(), finishLaunching];
        }
        let _: () = msg_send![app(),activateIgnoringOtherApps:Bool::YES];
        let _: () = msg_send![w,makeKeyAndOrderFront:NIL];
        let _: () = msg_send![w, makeMainWindow];
        let _: () = msg_send![w, makeKeyWindow];
        let _: Bool = msg_send![w,makeFirstResponder:files];
        let timer: Id = msg_send![class!(NSTimer),scheduledTimerWithTimeInterval:0.10f64,target:delegate,selector:sel!(tick:),userInfo:NIL,repeats:Bool::YES];
        // Track while a context menu or window drag is active as well.
        let loop_: Id = msg_send![class!(NSRunLoop), mainRunLoop];
        let _: () = msg_send![loop_,addTimer:timer,forMode:NSRunLoopCommonModes];
        context().timer.set(timer);
        timer
    }
}
/// Resize only views, never reload/navigate; tiling must preserve editor drafts
/// and file selection. At 480pt the sidebar contracts and columns can scroll.
fn layout() {
    let Some(w) = context().ui.get() else {
        return;
    };
    if w.tabs.is_null() {
        return;
    }
    unsafe {
        let content: Id = msg_send![w.window, contentView];
        let bounds: Rect = msg_send![content, bounds];
        let width = bounds.size.width;
        let height = bounds.size.height;
        frame(w.tabs, rect(8., height - 37., width - 88., 30.), 0);
        frame(w.new_tab, rect(width - 76., height - 35., 30., 28.), 0);
        frame(w.close_tab, rect(width - 42., height - 35., 30., 28.), 0);
        let y = height - 75.;
        frame(w.back, rect(8., y, 30., 28.), 0);
        frame(w.forward, rect(42., y, 30., 28.), 0);
        for (tag, x) in [(101isize, 76.), (102, 110.)] {
            let control: Id = msg_send![content,viewWithTag:tag];
            frame(control, rect(x, y, 30., 28.), 0);
        }
        frame(w.path, rect(148., y + 2., width - 292., 26.), 0);
        frame(w.star, rect(width - 136., y, 30., 28.), 0);
        let copy: Id = msg_send![content,viewWithTag:103isize];
        frame(copy, rect(width - 102., y, 62., 28.), 0);
        let appearance: Id = msg_send![content,viewWithTag:104isize];
        frame(appearance, rect(width - 36., y, 28., 28.), 0);
        let side_width = if width < 800. { 160. } else { 220. };
        let body = height - 116.;
        let side: Id = msg_send![w.sidebar, enclosingScrollView];
        frame(side, rect(8., 32., side_width, body), 0);
        let files: Id = msg_send![w.table, enclosingScrollView];
        frame(
            files,
            rect(side_width + 20., 32., width - side_width - 32., body),
            0,
        );
        frame(
            w.image,
            rect(side_width + 20., 32., width - side_width - 32., body),
            0,
        );
        frame(w.status, rect(14., 8., width - 28., 20.), 0);
    }
}
fn sync_tabs() {
    let w = widgets();
    let (labels, active) = tabs(|t| (t.labels(), t.active()));
    context().syncing_tabs.set(true);
    unsafe {
        let count: isize = msg_send![w.tabs, numberOfTabViewItems];
        if count as usize != labels.len() {
            for i in (0..count).rev() {
                let item: Id = msg_send![w.tabs,tabViewItemAtIndex:i];
                let _: () = msg_send![w.tabs,removeTabViewItem:item];
            }
            for i in 0..labels.len() {
                let item: Id = msg_send![class!(NSTabViewItem), alloc];
                let item: Id = msg_send![item,initWithIdentifier:ns(&i.to_string())];
                let _: () = msg_send![w.tabs,addTabViewItem:item];
                let _: () = msg_send![item, release];
            }
        }
        for (i, label) in labels.iter().enumerate() {
            let item: Id = msg_send![w.tabs,tabViewItemAtIndex:i as isize];
            let _: () = msg_send![item,setLabel:ns(label)];
        }
        let _: () = msg_send![w.tabs,selectTabViewItemAtIndex:active as isize];
    }
    context().syncing_tabs.set(false);
}
fn activate_tab(index: usize) {
    sync_selection(false);
    let draft = value(widgets().path);
    let new = tabs(|t| t.select(index, draft));
    if let Some(draft) = new {
        unsafe {
            let _: Bool = msg_send![widgets().window,makeFirstResponder:widgets().table];
        }
        render();
        set_value(widgets().path, &draft);
    }
}
extern "C" fn tab_selected(this: &AnyObject, _: Sel, tab_view: Id, item: Id) {
    let _window = enter_window(this, tab_view);
    if context().closed.get() {
        return;
    }
    if context().syncing_tabs.get() || context().rendering.get() || item.is_null() {
        return;
    }
    let index: isize = unsafe { msg_send![tab_view,indexOfTabViewItem:item] };
    if index >= 0 {
        activate_tab(index as usize);
    }
}
extern "C" fn new_tab(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    sync_selection(false);
    let draft = value(widgets().path);
    let draft = tabs(|t| t.open(draft));
    unsafe {
        let _: Bool = msg_send![widgets().window,makeFirstResponder:widgets().table];
    }
    render();
    set_value(widgets().path, &draft);
}
extern "C" fn close_tab(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    if let Some(draft) = tabs(|t| t.close()) {
        unsafe {
            let _: Bool = msg_send![widgets().window,makeFirstResponder:widgets().table];
        }
        render();
        set_value(widgets().path, &draft);
    } else {
        unsafe {
            let _: () = msg_send![widgets().window,performClose:NIL];
        }
    }
}
extern "C" fn next_tab(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    let index = tabs(|t| (t.active() + 1) % t.count());
    activate_tab(index);
}
extern "C" fn previous_tab(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    let index = tabs(|t| (t.active() + t.count() - 1) % t.count());
    activate_tab(index);
}
extern "C" fn resized(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    layout();
}
extern "C" fn cancel_test_menu(_: &AnyObject, _: Sel, timer: Id) {
    unsafe {
        let menu: Id = msg_send![timer, userInfo];
        let _: () = msg_send![menu, cancelTracking];
    }
}
fn places(m: &Browser) -> Vec<Place> {
    let mut p = vec![Place {
        label: "Places".into(),
        path: None,
    }];
    for (label, path) in [
        ("Home", core::home()),
        ("Desktop", core::home().join("Desktop")),
        ("Downloads", core::home().join("Downloads")),
        ("Computer", PathBuf::from("/")),
    ] {
        if label == "Home" || label == "Computer" || path.exists() {
            p.push(Place {
                label: label.into(),
                path: Some(path),
            });
        }
    }
    for (section, paths) in [
        ("Favorites", m.prefs.data.favorites.clone()),
        ("Recent", m.prefs.recent(7)),
        ("Frequent", m.prefs.frequent(5)),
    ] {
        p.push(Place {
            label: section.into(),
            path: None,
        });
        for path in paths {
            p.push(Place {
                label: path
                    .file_name()
                    .map(|s| s.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "/".into()),
                path: Some(path),
            });
        }
    }
    p
}
fn render() {
    context().shared_dirty.set(false);
    let w = widgets();
    context().rendering.set(true);
    let (
        display,
        current,
        selection,
        status,
        pinned,
        back,
        forward,
        theme,
        image,
        sort,
        descending,
    ) = model(|m| {
        let rows=m.items.iter().map(|e|{
            let modified=if e.modified==0{"—".into()}else{unsafe{
                let date:Id=msg_send![class!(NSDate),dateWithTimeIntervalSince1970:e.modified as f64];
                let text:Id=msg_send![class!(NSDateFormatter),localizedStringFromDate:date,dateStyle:1usize,timeStyle:1usize];string(text)
            }};
            [format!("{}{}",if e.is_dir{"▸  "}else{""},e.name()),modified,core::size_label(if e.is_dir{None}else{e.size})]
        }).collect();
        (
            Display {
                rows,
                paths: m.items.iter().map(|e| e.path.clone()).collect(),
                places: places(m),
            },
            m.current.clone(),
            m.selected.clone(),
            m.status.clone(),
            m.prefs.data.favorites.contains(&m.current),
            m.cursor > 0,
            m.cursor + 1 < m.history.len(),
            appearance::effective_theme(&m.prefs.data.theme, &m.items),
            m.prefs.data.image.clone(),
            m.prefs.data.sort,
            m.prefs.data.descending,
        )
    });
    let count = display.rows.len();
    let paths = display.paths.clone();
    context().display.with_borrow_mut(|d| *d = display);
    unsafe {
        let changed = context()
            .displayed_location
            .with_borrow(|last| last.as_ref() != Some(&current));
        if changed {
            set_value(w.path, &current.to_string_lossy());
        }
        context()
            .displayed_location
            .with_borrow_mut(|last| *last = Some(current.clone()));
        let _: () = msg_send![w.star,setTitle:ns(if pinned { "★" } else { "☆" })];
        let _: () = msg_send![w.back,setEnabled:Bool::new(back)];
        let _: () = msg_send![w.forward,setEnabled:Bool::new(forward)];
        let _: () = msg_send![w.window,setTitle:ns(&format!("{} — Personal Files",current.file_name().unwrap_or(current.as_os_str()).to_string_lossy()))];
        let _: () = msg_send![w.table, reloadData];
        let _: () = msg_send![w.sidebar, reloadData];
        let indexes: Id = msg_send![class!(NSMutableIndexSet), indexSet];
        for (index, path) in paths.iter().enumerate() {
            if selection.contains(path) {
                let _: () = msg_send![indexes,addIndex:index];
            }
        }
        let _: () = msg_send![w.table,selectRowIndexes:indexes,byExtendingSelection:Bool::NO];
        let columns: Id = msg_send![w.table, tableColumns];
        for (index, (key, title)) in [
            (Sort::Name, "Name"),
            (Sort::Modified, "Modified"),
            (Sort::Size, "Size"),
        ]
        .iter()
        .enumerate()
        {
            let column: Id = msg_send![columns,objectAtIndex:index];
            let _: () = msg_send![column,setTitle:ns(&format!("{title}{}",if *key==sort {if descending{" ↓"}else{" ↑"}}else{""}))];
        }
        let appearance: Id = msg_send![class!(NSAppearance),appearanceNamed:if theme=="silver"{NSAppearanceNameAqua}else{NSAppearanceNameDarkAqua}];
        let _: () = msg_send![w.window,setAppearance:appearance];
        let (r, g, b) = match theme.as_str() {
            "aurora" => (0.08f64, 0.19f64, 0.23f64),
            "midnight" => (0.08, 0.11, 0.17),
            _ => (0.96, 0.97, 0.98),
        };
        let color: Id = msg_send![class!(NSColor),colorWithSRGBRed:r,green:g,blue:b,alpha:1f64];
        let _: () = msg_send![w.window,setBackgroundColor:color];
        let clear: Id = msg_send![class!(NSColor), clearColor];
        let _: () =
            msg_send![w.table,setBackgroundColor:if image.as_os_str().is_empty(){color}else{clear}];
        let img = if image.as_os_str().is_empty() {
            NIL
        } else {
            load_image(&image)
        };
        let _: () = msg_send![w.image,setImage:img];
        if !img.is_null() {
            let _: () = msg_send![img, release];
        }
        set_value(
            w.status,
            &if status.is_empty() {
                format!("{count} items · Hidden files shown · {}", current.display())
            } else {
                status
            },
        );
    }
    sync_tabs();
    context().rendering.set(false);
}
fn sync_selection(from_context_menu: bool) {
    let table = widgets().table;
    unsafe {
        let mut indexes: Id = msg_send![table, selectedRowIndexes];
        let clicked: isize = msg_send![table, clickedRow];
        if from_context_menu && clicked >= 0 {
            let included: Bool = msg_send![indexes,containsIndex:clicked as usize];
            if !included.as_bool() {
                indexes = msg_send![class!(NSIndexSet),indexSetWithIndex:clicked as usize];
            }
        }
        let paths = context().display.with_borrow(|d| d.paths.clone());
        let mut selected = std::collections::BTreeSet::new();
        for (i, path) in paths.into_iter().enumerate() {
            let yes: Bool = msg_send![indexes,containsIndex:i];
            if yes.as_bool() {
                selected.insert(path);
            }
        }
        model(|m| m.selected = selected);
    }
}
fn copy(shell: bool, context: bool) {
    sync_selection(context);
    let text = model(|m| m.copy_paths(shell));
    unsafe {
        let p: Id = msg_send![class!(NSPasteboard), generalPasteboard];
        let _: isize = msg_send![p, clearContents];
        let _: Bool = msg_send![p,setString:ns(&text),forType:ns("public.utf8-plain-text")];
    }
}
fn is_context(sender: Id) -> bool {
    unsafe {
        !sender.is_null() && {
            let key: Id = msg_send![sender, keyEquivalent];
            string(key).is_empty()
        }
    }
}
fn context_sender(sender: Id) -> bool {
    unsafe {
        let is_menu: Bool = msg_send![sender,isKindOfClass:class!(NSMenuItem)];
        is_menu.as_bool() && is_context(sender)
    }
}
fn load_image(path: &Path) -> Id {
    unsafe {
        let i: Id = msg_send![class!(NSImage), alloc];
        let image: Id = msg_send![i,initWithContentsOfFile:ns(&path.to_string_lossy())];
        if !image.is_null() {
            let valid: Bool = msg_send![image, isValid];
            if !valid.as_bool() {
                let _: () = msg_send![image, release];
                return NIL;
            }
        }
        image
    }
}
fn set_image(path: &Path) -> bool {
    let image = load_image(path);
    if image.is_null() {
        model(|m| {
            m.status =
                "That file is not a readable image. The previous background is unchanged.".into()
        });
        return false;
    }
    unsafe {
        let _: () = msg_send![image, release];
    }
    model(|m| {
        m.prefs.data.image = path.to_path_buf();
        m.save();
    });
    true
}
fn dropped_paths(pasteboard: Id) -> Vec<PathBuf> {
    unsafe {
        let items: Id = msg_send![pasteboard, pasteboardItems];
        if items.is_null() {
            return vec![];
        }
        let count: usize = msg_send![items, count];
        let mut paths = vec![];
        for i in 0..count {
            let item: Id = msg_send![items,objectAtIndex:i];
            let raw: Id = msg_send![item,stringForType:ns("public.file-url")];
            if raw.is_null() {
                continue;
            }
            let url: Id = msg_send![class!(NSURL),URLWithString:raw];
            if url.is_null() {
                continue;
            }
            let local: Bool = msg_send![url, isFileURL];
            if !local.as_bool() {
                continue;
            }
            let host: Id = msg_send![url, host];
            let host = string(host);
            if !host.is_empty() && host != "localhost" {
                continue;
            }
            let path: Id = msg_send![url, path];
            let path = PathBuf::from(string(path));
            if path.is_absolute() && path.is_dir() {
                paths.push(path);
            }
        }
        paths
    }
}
fn accept_paths(paths: &[PathBuf]) -> bool {
    if paths.is_empty() {
        return false;
    }
    model(|m| {
        for path in paths {
            m.prefs.favorite(path);
        }
        m.save();
    });
    true
}

// Objective-C callback signatures below match NSTableViewDataSource/Delegate,
// NSWindowDelegate, NSObject target/action and NSTimer. All are main-thread only.
extern "C" fn rows(this: &AnyObject, _: Sel, table: Id) -> isize {
    let _window = enter_window(this, table);
    if context().closed.get() {
        return 0;
    }
    context().display.with_borrow(|d|if table==widgets().sidebar{d.places.len()}else{d.rows.len()} as isize)
}
extern "C" fn cell(this: &AnyObject, _: Sel, table: Id, column: Id, row: isize) -> Id {
    let _window = enter_window(this, table);
    if context().closed.get() {
        return NIL;
    }
    autoreleasepool(|_| {
        let (text, tooltip, section) = context().display.with_borrow(|d| {
            if table == widgets().sidebar {
                d.places
                    .get(row as usize)
                    .map(|p| {
                        (
                            p.label.clone(),
                            p.path
                                .as_ref()
                                .map(|p| p.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                            p.path.is_none(),
                        )
                    })
                    .unwrap_or_default()
            } else {
                let id = unsafe {
                    let id: Id = msg_send![column, identifier];
                    string(id)
                };
                let index = match id.as_str() {
                    "modified" => 1,
                    "size" => 2,
                    _ => 0,
                };
                (
                    d.rows
                        .get(row as usize)
                        .map(|r| r[index].clone())
                        .unwrap_or_default(),
                    d.paths
                        .get(row as usize)
                        .map(|p| p.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                    false,
                )
            }
        });
        unsafe {
            let field = label(&text);
            let _: () = msg_send![field,setToolTip:ns(&tooltip)];
            let _: () = msg_send![field,setLineBreakMode:3usize];
            let font: Id = if section {
                msg_send![class!(NSFont),boldSystemFontOfSize:11f64]
            } else {
                msg_send![class!(NSFont),systemFontOfSize:13f64]
            };
            let _: () = msg_send![field,setFont:font];
            // Retain/autorelease outside this nested pool before returning.
            let _: Id = msg_send![field, retain];
            field
        }
    })
    .pipe_autorelease()
}
// Balance the callback's nested pool retain in the caller's surrounding pool.
trait Autorelease {
    fn pipe_autorelease(self) -> Self;
}
impl Autorelease for Id {
    fn pipe_autorelease(self) -> Self {
        unsafe { msg_send![self, autorelease] }
    }
}
extern "C" fn selectable(this: &AnyObject, _: Sel, table: Id, row: isize) -> Bool {
    let _window = enter_window(this, table);
    if context().closed.get() {
        return Bool::NO;
    }
    Bool::new(
        table != widgets().sidebar
            || context()
                .display
                .with_borrow(|d| d.places.get(row as usize).is_some_and(|p| p.path.is_some())),
    )
}
extern "C" fn selection_changed(this: &AnyObject, _: Sel, notification: Id) {
    let _window = enter_window(this, notification);
    if context().closed.get() {
        return;
    }
    if context().rendering.get() {
        return;
    }
    let table: Id = unsafe { msg_send![notification, object] };
    if table == widgets().table {
        sync_selection(false);
    }
}
extern "C" fn header(this: &AnyObject, _: Sel, table: Id, column: Id) {
    let _window = enter_window(this, table);
    if context().closed.get() {
        return;
    }
    if table == widgets().sidebar {
        return;
    }
    unsafe {
        let id: Id = msg_send![column, identifier];
        model(|m| {
            m.sort(match string(id).as_str() {
                "modified" => Sort::Modified,
                "size" => Sort::Size,
                _ => Sort::Name,
            })
        });
    }
    render();
}
extern "C" fn writer(this: &AnyObject, _: Sel, table: Id, row: isize) -> Id {
    let _window = enter_window(this, table);
    if context().closed.get() {
        return NIL;
    }
    context().display.with_borrow(|d| {
        let path = if table == widgets().sidebar {
            d.places.get(row as usize).and_then(|p| p.path.as_ref())
        } else {
            d.paths.get(row as usize)
        };
        path.map_or(NIL, |p| file_url(p))
    })
}
extern "C" fn validate_drop(
    this: &AnyObject,
    _: Sel,
    table: Id,
    info: Id,
    _row: isize,
    _operation: usize,
) -> usize {
    let _window = enter_window(this, table);
    if context().closed.get() {
        return 0;
    }
    if table != widgets().sidebar {
        return 0;
    }
    let pb: Id = unsafe { msg_send![info, draggingPasteboard] };
    if dropped_paths(pb).is_empty() {
        0
    } else {
        1
    }
}
extern "C" fn accept_drop(
    this: &AnyObject,
    _: Sel,
    table: Id,
    info: Id,
    _row: isize,
    _operation: usize,
) -> Bool {
    let _window = enter_window(this, table);
    if context().closed.get() {
        return Bool::NO;
    }
    if table != widgets().sidebar {
        return Bool::NO;
    }
    let pb: Id = unsafe { msg_send![info, draggingPasteboard] };
    let accepted = accept_paths(&dropped_paths(pb));
    render();
    Bool::new(accepted)
}
extern "C" fn tick(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    autoreleasepool(|_| {
        let reply = context().connection.with_borrow_mut(|slot| {
            let reply = slot.as_ref().and_then(|rx| rx.try_recv().ok());
            if reply.is_some() {
                *slot = None;
            }
            reply
        });
        let connected =
            reply.is_some_and(|(token, result)| model(|m| m.finish_connection(token, result)));
        if connected {
            // A reconnect may resolve to the already displayed folder. Restore
            // the POSIX path even when the directory itself has not changed.
            context()
                .displayed_location
                .with_borrow_mut(|last| *last = None);
        }
        if model(|m| m.tick()) || connected || context().shared_dirty.get() {
            render();
        }
    });
}
extern "C" fn noop(_: &AnyObject, _: Sel, _: Id) {}
extern "C" fn closed(this: &AnyObject, _: Sel, _: Id) {
    let _window = enter_window(this, NIL);
    let state = context();
    if state.closed.replace(true) {
        return;
    }
    state.model.with_borrow_mut(|m| {
        if let Some(m) = m {
            m.browser.save();
        }
    });
    // Transfer shared settings before dropping this window's model.
    WINDOWS.with_borrow(|windows| {
        for other in windows.iter().filter(|s| !s.closed.get()) {
            share_preferences(&state, other);
        }
    });
    unsafe {
        let _: () = msg_send![state.timer.get(), invalidate];
    }
    state
        .connection
        .with_borrow_mut(|connection| *connection = None);
    state.model.with_borrow_mut(|model| *model = None);
    if FOCUSED.get() == widgets().delegate {
        FOCUSED.set(NIL);
    }
}
extern "C" fn became_key(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if !context().closed.get() {
        FOCUSED.set(widgets().delegate);
        render();
    }
}
extern "C" fn new_window(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    let state = context();
    let start = state
        .model
        .with_borrow(|m| m.as_ref().map(|m| m.browser.current.clone()));
    let mut prefs = Preferences::load(state.preferences_path.clone());
    state.model.with_borrow(|m| {
        if let Some(m) = m {
            prefs.data = m.browser.prefs.data.clone();
        }
    });
    setup(prefs, start);
}

fn start_connection(text: &str) {
    let location = match server_location(text) {
        Ok(location) => location,
        Err(message) => {
            // Never leave an accidentally pasted password in the path draft.
            set_value(
                widgets().path,
                &model(|m| m.current.to_string_lossy().into_owned()),
            );
            model(|m| m.status = message);
            return;
        }
    };
    if context().connection.with_borrow(|slot| slot.is_some()) {
        model(|m| {
            m.status = "A server connection is still in progress. Complete or cancel its macOS dialog before trying again.".into()
        });
        return;
    }
    let token = model(|m| m.begin_connection());
    let (tx, rx) = mpsc::channel();
    match std::thread::Builder::new()
        .name("server-connection".into())
        .spawn(move || {
            let _ = tx.send((token, mount_server(location)));
        }) {
        Ok(_) => context()
            .connection
            .with_borrow_mut(|slot| *slot = Some(rx)),
        Err(_) => {
            model(|m| {
                m.finish_connection(
                    token,
                    Err("Could not start server connection worker.".into()),
                )
            });
        }
    }
}
extern "C" fn connect_server(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    unsafe {
        let alert: Id = msg_send![class!(NSAlert), new];
        let _: () = msg_send![alert,setMessageText:ns("Connect to Server")];
        let _: () = msg_send![alert,setInformativeText:ns("Enter an SMB address. macOS handles your login and saved Keychain credentials.")];
        let _: Id = msg_send![alert,addButtonWithTitle:ns("Connect")];
        let _: Id = msg_send![alert,addButtonWithTitle:ns("Cancel")];
        let field: Id = msg_send![class!(NSTextField), alloc];
        let field: Id = msg_send![field,initWithFrame:rect(0.,0.,420.,26.)];
        set_value(field, "smb://");
        let _: () = msg_send![alert,setAccessoryView:field];
        let window: Id = msg_send![alert, window];
        let _: () = msg_send![window,setInitialFirstResponder:field];
        let response: isize = msg_send![alert, runModal];
        let text = value(field);
        let _: () = msg_send![field, release];
        let _: () = msg_send![alert, release];
        if response == 1000 {
            set_value(widgets().path, &text);
            start_connection(&text);
            render();
        }
    }
}
extern "C" fn navigate(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    let text = value(widgets().path);
    if text.trim().to_ascii_lowercase().starts_with("smb:") {
        start_connection(&text);
    } else {
        model(|m| m.navigate(&text));
    }
    unsafe {
        let _: Bool = msg_send![widgets().window,makeFirstResponder:widgets().table];
    }
    render();
}
extern "C" fn sidebar(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    let row: isize = unsafe { msg_send![widgets().sidebar, selectedRow] };
    let path = context()
        .display
        .with_borrow(|d| d.places.get(row as usize).and_then(|p| p.path.clone()));
    if let Some(path) = path {
        model(|m| m.navigate(&path.to_string_lossy()));
    }
}
extern "C" fn back(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    model(|m| m.back());
}
extern "C" fn forward(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    model(|m| m.forward());
}
extern "C" fn up(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    model(|m| m.up());
}
extern "C" fn refresh(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    sync_selection(false);
    model(|m| m.refresh());
}
extern "C" fn favorite(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    model(|m| m.toggle_favorite());
    render();
}
extern "C" fn focus(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    unsafe {
        let _: Bool = msg_send![widgets().window,makeFirstResponder:widgets().path];
        let _: () = msg_send![widgets().path,selectText:NIL];
    }
}
extern "C" fn copy_path(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    copy(false, context_sender(sender));
}
extern "C" fn copy_shell(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    copy(true, context_sender(sender));
}
extern "C" fn table_copy(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    copy(false, false);
}
extern "C" fn key_down(this: &AnyObject, _: Sel, event: Id) {
    let _window = enter_window(this, event);
    if context().closed.get() {
        return;
    }
    unsafe {
        let chars: Id = msg_send![event, charactersIgnoringModifiers];
        let chars = string(chars);
        if chars == "\r" || chars == "\u{3}" {
            open(this, sel!(open:), NIL);
        } else if chars == "\u{f708}" {
            refresh(this, sel!(refresh:), NIL);
        } else {
            let _: () = msg_send![super(this,class!(NSTableView)),keyDown:event];
        }
    }
}
extern "C" fn open(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    sync_selection(!sender.is_null() && context_sender(sender));
    let paths = model(|m| m.selected.iter().cloned().collect::<Vec<_>>());
    for path in paths {
        if path.is_dir() {
            model(|m| m.navigate(&path.to_string_lossy()));
            break;
        } else {
            unsafe {
                let workspace: Id = msg_send![class!(NSWorkspace), sharedWorkspace];
                let ok: Bool = msg_send![workspace,openURL:file_url(&path)];
                if !ok.as_bool() {
                    model(|m| m.status = "No application could open that document.".into());
                    render();
                }
            }
        }
    }
}
extern "C" fn add_favorites(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    sync_selection(true);
    model(|m| {
        for path in m.selected.clone() {
            if path.is_dir() {
                m.prefs.favorite(&path);
            }
        }
        m.save();
    });
    render();
}
extern "C" fn remove_favorite(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    let row: isize = unsafe { msg_send![widgets().sidebar, clickedRow] };
    let path = context()
        .display
        .with_borrow(|d| d.places.get(row as usize).and_then(|p| p.path.clone()));
    if let Some(path) = path {
        model(|m| {
            m.prefs.unfavorite(&path);
            m.save();
        });
        render();
    }
}
extern "C" fn theme(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    let tag: isize = unsafe { msg_send![sender, tag] };
    if let Some(theme) = ["silver", "midnight", "aurora", "auto"].get(tag as usize) {
        model(|m| {
            m.prefs.data.theme = (*theme).into();
            m.save();
        });
        render();
    }
}
extern "C" fn choose_image(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    unsafe {
        let panel: Id = msg_send![class!(NSOpenPanel), openPanel];
        let _: () = msg_send![panel,setCanChooseDirectories:Bool::NO];
        let _: () = msg_send![panel,setAllowsMultipleSelection:Bool::NO];
        let _: () = msg_send![panel,setTitle:ns("Choose a background image")];
        let result: isize = msg_send![panel, runModal];
        if result == 1 {
            let url: Id = msg_send![panel, URL];
            let path: Id = msg_send![url, path];
            set_image(Path::new(&string(path)));
            render();
        }
    }
}
extern "C" fn remove_image(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    model(|m| {
        m.prefs.data.image = PathBuf::new();
        m.save();
    });
    render();
}
extern "C" fn clear_history(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    model(|m| m.clear_visits());
    render();
}
fn alert(title: &str, text: &str) {
    unsafe {
        let alert: Id = msg_send![class!(NSAlert), new];
        let _: () = msg_send![alert,setMessageText:ns(title)];
        let _: () = msg_send![alert,setInformativeText:ns(text)];
        let _: isize = msg_send![alert, runModal];
        let _: () = msg_send![alert, release];
    }
}
extern "C" fn shortcuts(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    alert("Keyboard Shortcuts","⌘L Edit full path or SMB address\n⌘K Connect to Server\n⌘D Pin/unpin current folder\n⌘C Copy selected/current paths (native copy in path field)\n⇧⌘C Copy shell-quoted paths\n⌘[ / ⌘] Back / Forward\n⌘↑ Parent folder\n⌘R or F5 Refresh\nReturn or ⌘O Open selection\n⌘A Select all\n⌘N New window\n⌘T New tab · ⌘W Close tab (last closes window)\nControl–Tab / Control–Shift–Tab Next / previous tab\nFn–Control–F/C/R Fill / Center / Restore\nFn–Control–arrows Tile (see native Window menu)\n⌘Q Quit");
}
extern "C" fn about(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    alert("Personal Files","A native Rust macOS browser for your familiar places.\nFavorites stay yours. Recent and Frequent remember the rest.\nLocal preferences only. No telemetry. No destructive file operations.");
}
extern "C" fn appearance_menu(this: &AnyObject, _: Sel, sender: Id) {
    let _window = enter_window(this, sender);
    if context().closed.get() {
        return;
    }
    let menu = new_menu("Appearance");
    let d = widgets().delegate;
    for (title, tag) in [
        ("Silver", 0isize),
        ("Midnight", 1),
        ("Aurora", 2),
        ("Automatic", 3),
    ] {
        let i = item(menu, title, sel!(theme:), "", 0, d);
        unsafe {
            let _: () = msg_send![i,setTag:tag];
        }
    }
    item(
        menu,
        "Choose Background Image…",
        sel!(chooseImage:),
        "",
        0,
        d,
    );
    item(
        menu,
        "Remove Background Image",
        sel!(removeImage:),
        "",
        0,
        d,
    );
    unsafe {
        let _: Bool =
            msg_send![menu,popUpMenuPositioningItem:NIL,atLocation:Point{x:0.,y:0.},inView:sender];
    }
}
fn register_classes() {
    if AnyClass::get(c"PFDelegate").is_some() {
        return;
    }
    unsafe {
        let mut b = ClassBuilder::new(c"PFDelegate", class!(NSObject)).unwrap();
        for p in [
            c"NSTableViewDataSource",
            c"NSTableViewDelegate",
            c"NSWindowDelegate",
        ] {
            if let Some(p) = AnyProtocol::get(p) {
                b.add_protocol(p);
            }
        }
        for (s, f) in [
            (
                sel!(noop:),
                noop as extern "C" fn(&'static AnyObject, Sel, Id),
            ),
            (sel!(newWindow:), new_window),
            (sel!(windowDidBecomeKey:), became_key),
            (sel!(newTab:), new_tab),
            (sel!(closeTab:), close_tab),
            (sel!(nextTab:), next_tab),
            (sel!(previousTab:), previous_tab),
            (sel!(windowDidResize:), resized),
            (sel!(cancelTestMenu:), cancel_test_menu),
            (sel!(executeFixture:), execute_fixture),
            (sel!(windowWillClose:), closed),
            (sel!(tick:), tick),
            (sel!(navigate:), navigate),
            (sel!(connectServer:), connect_server),
            (sel!(sidebar:), sidebar),
            (sel!(back:), back),
            (sel!(forward:), forward),
            (sel!(up:), up),
            (sel!(refresh:), refresh),
            (sel!(favorite:), favorite),
            (sel!(focus:), focus),
            (sel!(copyPath:), copy_path),
            (sel!(copyShell:), copy_shell),
            (sel!(open:), open),
            (sel!(addFavorites:), add_favorites),
            (sel!(removeFavorite:), remove_favorite),
            (sel!(theme:), theme),
            (sel!(chooseImage:), choose_image),
            (sel!(removeImage:), remove_image),
            (sel!(clearHistory:), clear_history),
            (sel!(shortcuts:), shortcuts),
            (sel!(about:), about),
            (sel!(appearance:), appearance_menu),
        ] {
            b.add_method(s, f);
        }
        b.add_method(
            sel!(asyncTilingStep:),
            tiling_test::step as extern "C" fn(&'static AnyObject, Sel, Id),
        );
        b.add_method(
            sel!(tabView:didSelectTabViewItem:),
            tab_selected as extern "C" fn(&'static AnyObject, Sel, Id, Id),
        );
        b.add_method(
            sel!(numberOfRowsInTableView:),
            rows as extern "C" fn(&'static AnyObject, Sel, Id) -> isize,
        );
        b.add_method(
            sel!(tableView:viewForTableColumn:row:),
            cell as extern "C" fn(&'static AnyObject, Sel, Id, Id, isize) -> Id,
        );
        b.add_method(
            sel!(tableView:shouldSelectRow:),
            selectable as extern "C" fn(&'static AnyObject, Sel, Id, isize) -> Bool,
        );
        b.add_method(
            sel!(tableViewSelectionDidChange:),
            selection_changed as extern "C" fn(&'static AnyObject, Sel, Id),
        );
        b.add_method(
            sel!(tableView:didClickTableColumn:),
            header as extern "C" fn(&'static AnyObject, Sel, Id, Id),
        );
        b.add_method(
            sel!(tableView:pasteboardWriterForRow:),
            writer as extern "C" fn(&'static AnyObject, Sel, Id, isize) -> Id,
        );
        b.add_method(
            sel!(tableView:validateDrop:proposedRow:proposedDropOperation:),
            validate_drop as extern "C" fn(&'static AnyObject, Sel, Id, Id, isize, usize) -> usize,
        );
        b.add_method(
            sel!(tableView:acceptDrop:row:dropOperation:),
            accept_drop as extern "C" fn(&'static AnyObject, Sel, Id, Id, isize, usize) -> Bool,
        );
        b.register();
        let mut t = ClassBuilder::new(c"PFFileTable", class!(NSTableView)).unwrap();
        t.add_method(
            sel!(copy:),
            table_copy as extern "C" fn(&'static AnyObject, Sel, Id),
        );
        t.add_method(
            sel!(keyDown:),
            key_down as extern "C" fn(&'static AnyObject, Sel, Id),
        );
        t.register();
    }
}
fn cleanup(_timer: Id) {
    let windows = WINDOWS.with_borrow_mut(std::mem::take);
    for state in windows {
        if let Some(w) = state.ui.get() {
            unsafe {
                let _: () = msg_send![state.timer.get(), invalidate];
                let _: () = msg_send![w.window,setDelegate:NIL];
                let _: () = msg_send![w.window,orderOut:NIL];
            }
        }
        state.model.with_borrow_mut(|m| *m = None);
    }
    CURRENT.with_borrow_mut(|slot| *slot = None);
    FOCUSED.set(NIL);
    OWNED.with_borrow_mut(|owned| {
        for p in owned.drain(..).rev() {
            unsafe {
                let _: () = msg_send![p, release];
            }
        }
    });
}

pub fn run(start: Option<PathBuf>, fixture: bool) -> Result<(), String> {
    // AppKit must not be initialized by a Rust test worker thread.
    if unsafe {
        let thread: Id = msg_send![class!(NSThread), currentThread];
        let main: Bool = msg_send![thread, isMainThread];
        !main.as_bool()
    } {
        return Err("AppKit requires the initial thread.".into());
    }
    autoreleasepool(|_| {
        if fixture {
            register_classes();
            unsafe {
                let driver = own(msg_send![AnyClass::get(c"PFDelegate").unwrap(), new]);
                let _: Id = msg_send![class!(NSTimer),scheduledTimerWithTimeInterval:0.05f64,target:driver,selector:sel!(executeFixture:),userInfo:NIL,repeats:Bool::NO];
                let _: () = msg_send![app(), run];
            }
            return Err("Native fixture event loop stopped before completion.".into());
        }
        let timer = setup(Preferences::load(Preferences::default_path()), start);
        unsafe {
            let _: () = msg_send![app(), run];
        }
        cleanup(timer);
        Ok(())
    })
}

fn pump() {
    unsafe {
        let until: Id = msg_send![class!(NSDate),dateWithTimeIntervalSinceNow:0.02f64];
        let loop_: Id = msg_send![class!(NSRunLoop), mainRunLoop];
        let _: Bool = msg_send![loop_,runMode:NSDefaultRunLoopMode,beforeDate:until];
        let now: Id = msg_send![class!(NSDate), date];
        for _ in 0..32 {
            let event: Id = msg_send![app(),nextEventMatchingMask:usize::MAX,untilDate:now,inMode:NSDefaultRunLoopMode,dequeue:Bool::YES];
            if event.is_null() {
                break;
            }
            let _: () = msg_send![app(),sendEvent:event];
        }
        let _: () = msg_send![app(), updateWindows];
    }
    if !context().closed.get() && model(|m| m.tick()) {
        render();
    }
}
fn settle() {
    let start = Instant::now();
    while model(|m| m.is_loading()) {
        pump();
        assert!(start.elapsed() < Duration::from_secs(8), "scan timed out");
        std::thread::sleep(Duration::from_millis(5));
    }
}
/// Fixture-only clipboard backup. Materialize every advertised representation
/// in memory, never log or persist it. Do not overwrite a concurrent user copy.
struct ClipboardRestore {
    items: Vec<Id>,
    expected_count: isize,
}
impl ClipboardRestore {
    fn snapshot() -> Self {
        unsafe {
            let pb: Id = msg_send![class!(NSPasteboard), generalPasteboard];
            let old: Id = msg_send![pb, pasteboardItems];
            let count: usize = if old.is_null() {
                0
            } else {
                msg_send![old, count]
            };
            let mut items = vec![];
            for i in 0..count {
                let item: Id = msg_send![old,objectAtIndex:i];
                let copy: Id = msg_send![class!(NSPasteboardItem), new];
                let types: Id = msg_send![item, types];
                let n: usize = msg_send![types, count];
                for j in 0..n {
                    let kind: Id = msg_send![types,objectAtIndex:j];
                    let data: Id = msg_send![item,dataForType:kind];
                    if !data.is_null() {
                        let _: Bool = msg_send![copy,setData:data,forType:kind];
                    }
                }
                items.push(copy);
            }
            Self {
                items,
                expected_count: msg_send![pb, changeCount],
            }
        }
    }
    fn capture_change_count(&mut self) {
        unsafe {
            let pb: Id = msg_send![class!(NSPasteboard), generalPasteboard];
            self.expected_count = msg_send![pb, changeCount];
        }
    }
}
impl Drop for ClipboardRestore {
    fn drop(&mut self) {
        unsafe {
            let pb: Id = msg_send![class!(NSPasteboard), generalPasteboard];
            let count: isize = msg_send![pb, changeCount];
            if count == self.expected_count {
                let _: isize = msg_send![pb, clearContents];
                if !self.items.is_empty() {
                    let _: Bool = msg_send![pb,writeObjects:array(&self.items)];
                }
            }
            for item in self.items.drain(..) {
                let _: () = msg_send![item, release];
            }
        }
    }
}
extern "C" fn execute_fixture(_: &AnyObject, _: Sel, _: Id) {
    // Keyboard routing needs NSApplication.run, not just a Foundation timer loop.
    if std::env::var_os("PERSONAL_FILES_ASYNC_TILING_TEST").is_some() {
        tiling_test::start();
        return;
    }
    let result = std::panic::catch_unwind(|| {
        if std::env::var_os("PERSONAL_FILES_NEW_WINDOWS_TEST").is_some() {
            desktop_tests::new_windows_fixture()
        } else if std::env::var_os("PERSONAL_FILES_SAVED_TABS_TEST").is_some() {
            desktop_tests::saved_tabs_fixture()
        } else {
            desktop_fixture()
        }
    });
    let code = match result {
        Ok(Ok(())) => 0,
        Ok(Err(e)) => {
            eprintln!("{e}");
            1
        }
        Err(_) => 1,
    };
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}
fn desktop_fixture() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let root = temp.path().join("places");
    let other = temp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&other).unwrap();
    std::fs::create_dir(root.join("folder")).unwrap();
    std::fs::write(root.join(".hidden"), "unchanged").unwrap();
    std::fs::write(root.join("a b's $notes.txt"), "unchanged").unwrap();
    let prefs = temp.path().join("state/preferences.json");
    let timer = setup(Preferences::load(prefs.clone()), Some(root.clone()));
    settle();
    let w = widgets();
    unsafe {
        let menu: Id = msg_send![app(), windowsMenu];
        let min: Size = msg_send![w.window, contentMinSize];
        let screen: Id = msg_send![w.window, screen];
        let visible: Rect = msg_send![screen, visibleFrame];
        eprintln!("tiling configuration: window_menu_registered={}, content_min={}x{}, display_visible={}x{}",
            !menu.is_null(),min.width,min.height,visible.size.width,visible.size.height);
        assert!(!menu.is_null(),"Regression: native Window menu is not registered; AppKit cannot install tiling menu hotkeys.");
        let _: () = msg_send![w.window, makeKeyWindow];
        let _: () = msg_send![w.window, makeMainWindow];
        let key: Id = msg_send![app(), keyWindow];
        let can_key: Bool = msg_send![w.window, canBecomeKeyWindow];
        let is_key: Bool = msg_send![w.window, isKeyWindow];
        let active: Bool = msg_send![app(), isActive];
        eprintln!(
            "native focus: key={key:p} eligible={} is_key={} app_active={}",
            can_key.as_bool(),
            is_key.as_bool(),
            active.as_bool()
        );
        // Standard tiling items are lazily populated when the Window menu opens.
        let timer: Id = msg_send![class!(NSTimer),scheduledTimerWithTimeInterval:0.2f64,target:w.delegate,selector:sel!(cancelTestMenu:),userInfo:menu,repeats:Bool::NO];
        let loop_: Id = msg_send![class!(NSRunLoop), mainRunLoop];
        let _: () = msg_send![loop_,addTimer:timer,forMode:NSRunLoopCommonModes];
        let content: Id = msg_send![w.window, contentView];
        let _: Bool = msg_send![menu,popUpMenuPositioningItem:NIL,atLocation:Point{x:10.,y:10.},inView:content];
        desktop_tests::exercise_tiling(menu);
        assert!(
            min.width <= visible.size.width / 2. - 16.
                && min.height <= visible.size.height / 2. - 40.,
            "Regression: minimum content size excludes desktop half/quarter tiling."
        );
    }
    let count: isize = unsafe { msg_send![w.table, numberOfRows] };
    assert_eq!(count, 3);
    assert!(context()
        .display
        .with_borrow(|d| d.paths.contains(&root.join(".hidden"))));
    unsafe {
        let _: () = msg_send![w.star,performClick:NIL];
    }
    assert!(Preferences::load(prefs.clone())
        .data
        .favorites
        .contains(&root));
    set_value(w.path, &other.to_string_lossy());
    unsafe {
        let _: Bool = msg_send![app(),sendAction:sel!(navigate:),to:w.delegate,from:w.path];
    }
    settle();
    assert_eq!(model(|m| m.current.clone()), other);
    back(unsafe { &*w.delegate }, sel!(back:), NIL);
    settle();
    assert_eq!(model(|m| m.current.clone()), root);
    // Real table selection and real clipboard, on a named temporary pasteboard
    // for drop tests. General clipboard is snapshotted/restored by the caller.
    let file = root.join("a b's $notes.txt");
    let index = context()
        .display
        .with_borrow(|d| d.paths.iter().position(|p| p == &file).unwrap());
    unsafe {
        let indexes: Id = msg_send![class!(NSIndexSet),indexSetWithIndex:index];
        let _: () = msg_send![w.table,selectRowIndexes:indexes,byExtendingSelection:Bool::NO];
    }
    let mut clipboard = ClipboardRestore::snapshot();
    copy(false, false);
    clipboard.capture_change_count();
    unsafe {
        let pb: Id = msg_send![class!(NSPasteboard), generalPasteboard];
        let text: Id = msg_send![pb,stringForType:ns("public.utf8-plain-text")];
        assert_eq!(string(text), file.to_string_lossy());
    }
    copy(true, false);
    clipboard.capture_change_count();
    unsafe {
        let pb: Id = msg_send![class!(NSPasteboard), generalPasteboard];
        let text: Id = msg_send![pb,stringForType:ns("public.utf8-plain-text")];
        assert_eq!(string(text), core::shell_quote(&file.to_string_lossy()));
    }
    // Native responder-chain Copy must copy editor text, not selected paths.
    for _ in 0..5 {
        pump();
    }
    focus(unsafe { &*w.delegate }, sel!(focus:), NIL);
    set_value(w.path, "native text editing");
    unsafe {
        let editor: Id = msg_send![w.path, currentEditor];
        assert!(!editor.is_null());
        let _: () = msg_send![editor,selectAll:NIL];
        let sent: Bool = msg_send![app(),sendAction:sel!(copy:),to:NIL,from:NIL];
        if !sent.as_bool() {
            // A background harness may be denied foreground activation. Keep
            // the strict check opt-in; do not claim it passed in that session.
            assert!(
                std::env::var_os("PERSONAL_FILES_REQUIRE_FOREGROUND").is_none(),
                "Foreground Copy routing requires this app to own the Mac key window."
            );
            let direct: Bool = msg_send![app(),sendAction:sel!(copy:),to:editor,from:NIL];
            assert!(direct.as_bool());
            println!("UNVERIFIED foreground menu routing: background session has no key window; native editor Copy tested directly");
        }
        clipboard.capture_change_count();
        let pb: Id = msg_send![class!(NSPasteboard), generalPasteboard];
        let text: Id = msg_send![pb,stringForType:ns("public.utf8-plain-text")];
        assert_eq!(string(text), "native text editing");
        let _: Bool = msg_send![w.window,makeFirstResponder:w.table];
    }
    drop(clipboard);
    let original = model(|m| m.current.clone());
    set_value(w.path, "missing-child");
    navigate(unsafe { &*w.delegate }, sel!(navigate:), NIL);
    settle();
    assert_eq!(model(|m| m.current.clone()), original);
    model(|m| {
        m.status.clear();
        m.navigate(root.join(".hidden").to_str().unwrap());
    });
    settle();
    assert!(model(|m| m.selected.contains(&root.join(".hidden"))));
    // Native table header callback sorts both directions without moving folders.
    unsafe {
        let cols: Id = msg_send![w.table, tableColumns];
        let col: Id = msg_send![cols,objectAtIndex:2usize];
        let _: () = msg_send![w.delegate,tableView:w.table,didClickTableColumn:col];
        let _: () = msg_send![w.delegate,tableView:w.table,didClickTableColumn:col];
    }
    assert!(model(|m| m.items[0].is_dir));
    let valid_image = temp.path().join("background.icns");
    std::fs::copy(
        "/System/Library/CoreServices/CoreTypes.bundle/Contents/Resources/GenericFolderIcon.icns",
        &valid_image,
    )
    .unwrap();
    assert!(set_image(&valid_image));
    assert!(!set_image(&file));
    assert_eq!(model(|m| m.prefs.data.image.clone()), valid_image);
    for theme in ["silver", "midnight", "aurora", "auto"] {
        model(|m| {
            m.prefs.data.theme = theme.into();
            m.save();
        });
        render();
    }
    // URL decoding accepts local directories; remote URLs and files are ignored.
    unsafe {
        let pb: Id = msg_send![class!(NSPasteboard), pasteboardWithUniqueName];
        let _: Bool = msg_send![pb,writeObjects:array(&[file_url(&other),file_url(&file)])];
        let paths = dropped_paths(pb);
        assert_eq!(paths, vec![other.clone()]);
        assert!(accept_paths(&paths));
        let _: () = msg_send![pb, releaseGlobally];
    }
    // Reject a remote file host and non-file URL, rather than interpreting text.
    unsafe {
        let pb: Id = msg_send![class!(NSPasteboard), pasteboardWithUniqueName];
        for raw in [
            "https://example.invalid/folder",
            "file://remote.invalid/tmp/folder",
            "$(touch SHOULD_NOT_EXIST)",
        ] {
            let _: isize = msg_send![pb, clearContents];
            let _: Bool = msg_send![pb,setString:ns(raw),forType:ns("public.file-url")];
            assert!(dropped_paths(pb).is_empty());
        }
        let _: () = msg_send![pb, releaseGlobally];
    }
    assert!(!temp.path().join("SHOULD_NOT_EXIST").exists());
    let selected_before = model(|m| m.selected.clone());
    let visits = model(|m| m.prefs.data.visits[&root].count);
    std::fs::write(root.join("new"), "fixture").unwrap();
    let start = Instant::now();
    while model(|m| m.items.len()) != 4 {
        pump();
        std::thread::sleep(Duration::from_millis(20));
        assert!(start.elapsed() < Duration::from_secs(8));
    }
    assert_eq!(model(|m| m.prefs.data.visits[&root].count), visits);
    assert_eq!(model(|m| m.selected.clone()), selected_before);
    model(|m| {
        m.navigate(other.to_str().unwrap());
        m.navigate(root.to_str().unwrap());
    });
    settle();
    assert_eq!(model(|m| m.current.clone()), root);
    desktop_tests::exercise_tabs(&root, &other);
    clear_history(unsafe { &*w.delegate }, sel!(clearHistory:), NIL);
    let saved = Preferences::load(prefs.clone());
    assert!(saved.data.visits.is_empty());
    assert_eq!(saved.data.favorites.len(), 2);
    assert_eq!(std::fs::read_to_string(file).unwrap(), "unchanged");
    assert_eq!(
        std::fs::read_to_string(root.join(".hidden")).unwrap(),
        "unchanged"
    );
    // Exercise connection delivery through the real timer/render path without
    // network access, Keychain inspection or a synthetic authentication dialog.
    let token = model(|m| m.begin_connection());
    let (tx, rx) = mpsc::channel();
    context()
        .connection
        .with_borrow_mut(|slot| *slot = Some(rx));
    tx.send((token, Ok(other.clone()))).unwrap();
    tick(unsafe { &*w.delegate }, sel!(tick:), NIL);
    settle();
    assert_eq!(model(|m| m.current.clone()), other);
    assert_eq!(value(w.path), other.to_string_lossy());
    back(unsafe { &*w.delegate }, sel!(back:), NIL);
    settle();
    let prior = model(|m| m.current.clone());
    set_value(w.path, "smb://server.invalid/already-mounted");
    let token = model(|m| m.begin_connection());
    let (tx, rx) = mpsc::channel();
    context()
        .connection
        .with_borrow_mut(|slot| *slot = Some(rx));
    tx.send((token, Ok(prior.clone()))).unwrap();
    tick(unsafe { &*w.delegate }, sel!(tick:), NIL);
    settle();
    assert_eq!(value(w.path), prior.to_string_lossy());
    set_value(w.path, "smb://fixture:DO_NOT_STORE@server.invalid/share");
    navigate(unsafe { &*w.delegate }, sel!(navigate:), NIL);
    assert_eq!(model(|m| m.current.clone()), prior);
    assert!(!value(w.path).contains("DO_NOT_STORE"));
    assert!(!std::fs::read_to_string(&prefs)
        .unwrap()
        .contains("DO_NOT_STORE"));
    println!("PASS native SMB routing: injected mount delivery, mounted path display, reconnect to same folder, password URL rejection without persistence (no live network/authentication)");
    // Optional off-screen capture contains synthetic paths only, never user data.
    if let Some(path) = std::env::var_os("PERSONAL_FILES_FIXTURE_SCREENSHOT") {
        model(|m| {
            m.status.clear();
            m.prefs.data.image = PathBuf::new();
            m.prefs.data.theme = "silver".into();
        });
        render();
        unsafe {
            let view: Id = msg_send![w.window, contentView];
            let _: () = msg_send![view, displayIfNeeded];
            let bounds: Rect = msg_send![view, bounds];
            let rep: Id = msg_send![view,bitmapImageRepForCachingDisplayInRect:bounds];
            let _: () = msg_send![view,cacheDisplayInRect:bounds,toBitmapImageRep:rep];
            let props: Id = msg_send![class!(NSDictionary), dictionary];
            let data: Id = msg_send![rep,representationUsingType:4usize,properties:props];
            let written: Bool = msg_send![data,writeToFile:ns(&PathBuf::from(path).to_string_lossy()),atomically:Bool::YES];
            assert!(written.as_bool());
        }
    }
    // Save two actual native tabs with independent sorts, then reopen the window.
    let first_sort = model(|m| (m.prefs.data.sort, m.prefs.data.descending));
    unsafe {
        let _: () = msg_send![w.new_tab,performClick:NIL];
    }
    settle();
    set_value(w.path, &other.to_string_lossy());
    navigate(unsafe { &*w.delegate }, sel!(navigate:), NIL);
    settle();
    unsafe {
        let cols: Id = msg_send![w.table, tableColumns];
        let col: Id = msg_send![cols,objectAtIndex:1usize];
        let _: () = msg_send![w.delegate,tableView:w.table,didClickTableColumn:col];
    }
    let second_sort = model(|m| (m.prefs.data.sort, m.prefs.data.descending));
    unsafe {
        let _: () = msg_send![w.tabs,selectTabViewItemAtIndex:0isize];
    }
    settle();
    assert_eq!(
        model(|m| (m.prefs.data.sort, m.prefs.data.descending)),
        first_sort
    );
    cleanup(timer);
    // A fresh native window restores tabs, active index, sidebar and settings.
    let restarted = setup(Preferences::load(prefs), None);
    settle();
    assert_eq!(model(|m| m.current.clone()), root);
    assert_eq!(tabs(|t| (t.count(), t.active())), (2, 0));
    assert!(model(|m| m.selected.is_empty()));
    assert_eq!(
        model(|m| (m.prefs.data.sort, m.prefs.data.descending)),
        first_sort
    );
    unsafe {
        let count: isize = msg_send![widgets().tabs, numberOfTabViewItems];
        assert_eq!(count, 2);
        let _: () = msg_send![widgets().tabs,selectTabViewItemAtIndex:1isize];
    }
    settle();
    assert_eq!(model(|m| m.current.clone()), other);
    assert_eq!(
        model(|m| (m.prefs.data.sort, m.prefs.data.descending)),
        second_sort
    );
    unsafe {
        let _: () = msg_send![widgets().tabs,selectTabViewItemAtIndex:0isize];
    }
    settle();
    println!("PASS native saved tabs: two restored NSTabView items, active folder, independent sorting, no restored selection");
    assert_eq!(model(|m| m.prefs.data.favorites.len()), 2);
    let row = context().display.with_borrow(|d| {
        d.places
            .iter()
            .position(|p| p.path.as_ref() == Some(&other))
            .unwrap()
    });
    unsafe {
        let indexes: Id = msg_send![class!(NSIndexSet),indexSetWithIndex:row];
        let _: () =
            msg_send![widgets().sidebar,selectRowIndexes:indexes,byExtendingSelection:Bool::NO];
        let sent: Bool =
            msg_send![app(),sendAction:sel!(sidebar:),to:widgets().delegate,from:widgets().sidebar];
        assert!(sent.as_bool());
    }
    settle();
    assert_eq!(model(|m| m.current.clone()), other);
    cleanup(restarted);
    println!("PASS native desktop fixture: widgets, navigation, sidebar state, hidden files, selection, clipboard, sort callbacks, appearance, drop URL handler, monitoring, stale scans, persistence, no document mutations");
    Ok(())
}

#[cfg(test)]
mod server_tests {
    use super::*;

    #[test]
    fn smb_addresses_preserve_identity_and_decode_subfolders() {
        autoreleasepool(|_| {
            let location =
                server_location("smb://tester@192.0.2.10/Shared%20Files/sub%20folder/日本語")
                    .unwrap();
            assert_eq!(location.mount_url, "smb://tester@192.0.2.10/Shared%20Files");
            assert_eq!(location.relative, PathBuf::from("sub folder/日本語"));
            assert_eq!(
                server_location("SMB://tester@[::1]:445/share")
                    .unwrap()
                    .mount_url,
                "smb://tester@[::1]:445/share"
            );
            assert_eq!(
                server_location("smb://tester@192.0.2.10").unwrap().relative,
                PathBuf::new()
            );
        });
    }

    #[test]
    fn passwords_non_smb_and_unsafe_paths_are_rejected_without_echoing_input() {
        autoreleasepool(|_| {
            for text in [
                "smb://fixture:SECRET@server/share",
                "smb://fixture:%53ECRET@server/share",
                "smb://server/share?password=SECRET",
                "smb://server/share#SECRET",
                "https://server/share",
                "smb://",
                "smb:///share",
                "smb://server/share/../escape",
                "smb://server/share/%2e%2e/escape",
                "smb://server/share/%00",
                "smb://server/share\n",
            ] {
                let error = server_location(text).expect_err(text);
                assert!(!error.contains("SECRET"));
            }
        });
    }
}
