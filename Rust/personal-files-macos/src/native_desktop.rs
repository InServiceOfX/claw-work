//! Independent native acceptance helpers, exercised only by --self-test.
use super::*;

fn find_menu_item(menu: Id, title: &str) -> Id {
    unsafe {
        let items: Id = msg_send![menu, itemArray];
        let n: usize = msg_send![items, count];
        for i in 0..n {
            let item: Id = msg_send![items,objectAtIndex:i];
            let text: Id = msg_send![item, title];
            if string(text) == title {
                return item;
            }
            let submenu: Id = msg_send![item, submenu];
            if !submenu.is_null() {
                let found = find_menu_item(submenu, title);
                if !found.is_null() {
                    return found;
                }
            }
        }
        NIL
    }
}
fn key(key: &str, flags: usize, keycode: u16) {
    unsafe {
        let number: isize = msg_send![widgets().window, windowNumber];
        let event: Id = msg_send![class!(NSEvent),keyEventWithType:10usize,location:Point{x:0.,y:0.},modifierFlags:flags,timestamp:0f64,windowNumber:number,context:NIL,characters:ns(key),charactersIgnoringModifiers:ns(key),isARepeat:Bool::NO,keyCode:keycode];
        let menu: Id = msg_send![app(), mainMenu];
        let handled: Bool = msg_send![menu,performKeyEquivalent:event];
        assert!(
            handled.as_bool(),
            "native menu did not dispatch key {key:?} flags {flags:#x}"
        );
    }
    for _ in 0..4 {
        pump();
    }
}
pub(super) fn exercise_tabs(root: &Path, other: &Path) {
    let w = widgets();
    let selection = model(|m| m.selected.clone());
    let visits = model(|m| m.prefs.data.visits[root].count);
    focus(unsafe { &*w.delegate }, sel!(focus:), NIL);
    set_value(w.path, "unfinished / path draft");
    key("t", 1 << 20, 17);
    settle();
    assert_eq!(tabs(|t| t.count()), 2);
    assert_eq!(model(|m| m.current.clone()), root);
    assert_eq!(model(|m| m.prefs.data.visits[root].count), visits);
    let count: isize = unsafe { msg_send![w.tabs, numberOfTabViewItems] };
    assert_eq!(count, 2);
    set_value(w.path, &other.to_string_lossy());
    navigate(unsafe { &*w.delegate }, sel!(navigate:), NIL);
    settle();
    unsafe {
        let _: () = msg_send![w.tabs,selectTabViewItemAtIndex:0isize];
    }
    settle();
    assert_eq!(model(|m| m.current.clone()), root);
    assert_eq!(model(|m| m.selected.clone()), selection);
    assert_eq!(value(w.path), "unfinished / path draft");
    key("\t", 1 << 18, 48);
    settle();
    assert_eq!(model(|m| m.current.clone()), other);
    key("\t", (1 << 18) | (1 << 17), 48);
    settle();
    assert_eq!(model(|m| m.current.clone()), root);
    key("\t", 1 << 18, 48);
    settle();
    key("w", 1 << 20, 13);
    settle();
    assert_eq!(tabs(|t| t.count()), 1);
    assert_eq!(model(|m| m.current.clone()), root);
    unsafe {
        let _: () = msg_send![w.new_tab,performClick:NIL];
    }
    settle();
    assert_eq!(tabs(|t| t.count()), 2);
    unsafe {
        let _: () = msg_send![w.close_tab,performClick:NIL];
    }
    settle();
    assert_eq!(tabs(|t| t.count()), 1);
    set_value(w.path, &root.to_string_lossy());
    println!("PASS native tabs: Command-T/W key equivalents, click/cycle switching, independent folder/selection/draft, shared visits and add/close controls");
}
pub(super) fn exercise_tiling(menu: Id) {
    for title in [
        "Fill",
        "Center",
        "Left",
        "Right",
        "Top",
        "Bottom",
        "Return to Previous Size",
    ] {
        let item = find_menu_item(menu, title);
        assert!(!item.is_null(), "missing OS tiling item {title}");
        unsafe {
            let flags: usize = msg_send![item, keyEquivalentModifierMask];
            assert_eq!(
                flags & ((1 << 23) | (1 << 18)),
                (1 << 23) | (1 << 18),
                "missing native Fn-Control key modifiers for {title}"
            );
        }
    }
    println!(
        "PASS native tiling menu: OS-provided Fill/Center/halves/restore and Fn-Control modifiers"
    );
}

/// Separate acceptance for saved tabs; the full tiling fixture remains mandatory
/// and unchanged. This fixture never reads real preferences or the clipboard.
pub(super) fn saved_tabs_fixture() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let a = temp.path().join("first");
    let b = temp.path().join("second");
    std::fs::create_dir(&a).unwrap();
    std::fs::create_dir(&b).unwrap();
    let prefs = temp.path().join("preferences.json");
    let timer = setup(Preferences::load(prefs.clone()), Some(a.clone()));
    settle();
    exercise_tabs(&a, &b);
    let w = widgets();
    unsafe {
        let cols: Id = msg_send![w.table, tableColumns];
        let size: Id = msg_send![cols,objectAtIndex:2usize];
        let _: () = msg_send![w.delegate,tableView:w.table,didClickTableColumn:size];
        let _: () = msg_send![w.new_tab,performClick:NIL];
    }
    settle();
    set_value(w.path, &b.to_string_lossy());
    navigate(unsafe { &*w.delegate }, sel!(navigate:), NIL);
    settle();
    unsafe {
        let cols: Id = msg_send![w.table, tableColumns];
        let modified: Id = msg_send![cols,objectAtIndex:1usize];
        let _: () = msg_send![w.delegate,tableView:w.table,didClickTableColumn:modified];
    }
    let visits = model(|m| serde_json::to_value(&m.prefs.data.visits).unwrap());
    cleanup(timer);
    let restarted = setup(Preferences::load(prefs), None);
    settle();
    assert_eq!(tabs(|t| (t.count(), t.active())), (2, 1));
    assert_eq!(model(|m| m.current.clone()), b);
    assert_eq!(model(|m| m.prefs.data.sort), Sort::Modified);
    unsafe {
        let count: isize = msg_send![widgets().tabs, numberOfTabViewItems];
        assert_eq!(count, 2);
        let _: () = msg_send![widgets().tabs,selectTabViewItemAtIndex:0isize];
    }
    settle();
    assert_eq!(model(|m| m.current.clone()), a);
    assert_eq!(model(|m| m.prefs.data.sort), Sort::Size);
    assert!(model(|m| m.selected.is_empty()));
    assert_eq!(
        model(|m| serde_json::to_value(&m.prefs.data.visits).unwrap()),
        visits
    );
    cleanup(restarted);
    println!("PASS native saved tabs: key equivalents, actual tab controls, restored order/active folder, independent sorts, unchanged visits");
    Ok(())
}

/// Real independent NSWindows, without reading any personal preferences/files.
pub(super) fn new_windows_fixture() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let root = temp.path().join("root");
    let other = temp.path().join("other");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&other).unwrap();
    let file = root.join("fixture.txt");
    std::fs::write(&file, "unchanged").unwrap();
    let prefs = temp.path().join("preferences.json");
    let timer = setup(Preferences::load(prefs.clone()), Some(root.clone()));
    settle();
    let first = widgets();
    model(|m| {
        m.selected.insert(file.clone());
    });
    focus(unsafe { &*first.delegate }, sel!(focus:), NIL);
    set_value(first.path, "unfinished first-window draft");
    key("n", 1 << 20, 45);
    let second_state = WINDOWS.with_borrow(|windows| windows.last().unwrap().clone());
    let second = second_state.ui.get().unwrap();
    assert_ne!(first.window, second.window);
    assert_ne!(first.table, second.table);
    assert_ne!(first.delegate, second.delegate);
    assert_eq!(
        WINDOWS.with_borrow(|windows| windows.iter().filter(|w| !w.closed.get()).count()),
        2
    );
    for window in [first.window, second.window] {
        let visible: Bool = unsafe { msg_send![window, isVisible] };
        assert!(
            visible.as_bool(),
            "New Window must create a real visible NSWindow"
        );
    }
    assert_eq!(value(first.path), "unfinished first-window draft");
    {
        let _window = enter_window(unsafe { &*second.delegate }, NIL);
        settle();
        assert_eq!(model(|m| m.current.clone()), root);
        assert!(model(|m| m.selected.is_empty()));
        assert_eq!(tabs(|t| t.count()), 1);
        set_value(second.path, &other.to_string_lossy());
        navigate(unsafe { &*second.delegate }, sel!(navigate:), second.path);
        settle();
        assert_eq!(model(|m| m.current.clone()), other);
        favorite(unsafe { &*second.delegate }, sel!(favorite:), NIL);
        model(|m| {
            m.prefs.data.theme = "midnight".into();
            m.save();
        });
        let token = model(|m| m.begin_connection());
        let (tx, rx) = mpsc::channel();
        context()
            .connection
            .with_borrow_mut(|slot| *slot = Some(rx));
        tx.send((token, Err("second-window connection cancelled".into())))
            .unwrap();
    }
    // Menu targets are shared, but actions must route to the focused window.
    key("t", 1 << 20, 17);
    assert_eq!(tabs(|t| t.count()), 1);
    assert_eq!(model(|m| m.current.clone()), root);
    assert!(model(|m| m.selected.contains(&file)));
    assert_eq!(value(first.path), "unfinished first-window draft");
    assert!(model(|m| m.prefs.data.favorites.contains(&other)));
    assert_eq!(model(|m| m.prefs.data.theme.clone()), "midnight");
    assert!(!model(|m| m.status.contains("second-window connection")));
    {
        let _window = enter_window(unsafe { &*second.delegate }, NIL);
        assert_eq!(tabs(|t| t.count()), 2);
        assert_eq!(model(|m| m.current.clone()), other);
        // Close Tab leaves its window open until its last tab closes.
        key("w", 1 << 20, 13);
        assert!(!second_state.closed.get());
        assert_eq!(tabs(|t| t.count()), 1);
        key("w", 1 << 20, 13);
        assert!(second_state.closed.get());
        assert!(second_state.model.with_borrow(|m| m.is_none()));
    }
    assert_eq!(model(|m| m.current.clone()), root);
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "unchanged");
    // Closing the original window must also leave newer windows functional.
    key("n", 1 << 20, 45);
    let third = WINDOWS.with_borrow(|windows| windows.last().unwrap().ui.get().unwrap());
    {
        let _window = enter_window(unsafe { &*third.delegate }, NIL);
        settle();
        assert_eq!(model(|m| m.current.clone()), root);
    }
    unsafe {
        let _: () = msg_send![first.window, performClose:NIL];
    }
    assert!(!context().closed.get());
    favorite(unsafe { &*third.delegate }, sel!(favorite:), NIL);
    key("n", 1 << 20, 45);
    let fourth = WINDOWS.with_borrow(|windows| windows.last().unwrap().ui.get().unwrap());
    {
        let _window = enter_window(unsafe { &*fourth.delegate }, NIL);
        settle();
        assert!(model(|m| m.prefs.data.favorites.contains(&root)));
        assert!(model(|m| m.prefs.data.favorites.contains(&other)));
    }
    unsafe {
        let _: () = msg_send![third.window, performClose:NIL];
    }
    // New Window also remains usable after closing the last window.
    unsafe {
        let _: () = msg_send![fourth.window, performClose:NIL];
    }
    assert!(WINDOWS.with_borrow(|windows| windows.iter().all(|w| w.closed.get())));
    key("n", 1 << 20, 45);
    assert_eq!(
        WINDOWS.with_borrow(|windows| windows.iter().filter(|w| !w.closed.get()).count()),
        1
    );
    let saved = Preferences::load(prefs);
    assert!(saved.data.favorites.contains(&other));
    assert_eq!(saved.data.theme, "midnight");
    cleanup(timer);
    println!("PASS native new windows: Command-N from path editor, visible independent NSWindows, separate folders/tabs/selection/drafts/connections, focused menu routing, shared favorites/appearance, close isolation and reopen after last close");
    Ok(())
}

/// Uses real windows, pasteboard objects, callback routing and async file_ops.
pub(super) fn clipboard_fixture() -> Result<(), String> {
    let temp = tempfile::tempdir().map_err(|e| e.to_string())?;
    let source = temp.path().join("source");
    let destination = temp.path().join("destination");
    std::fs::create_dir(&source).unwrap();
    std::fs::create_dir(&destination).unwrap();
    let file = source.join("résumé 100% 'notes.txt");
    std::fs::write(&file, b"original").unwrap();
    let mut restore = ClipboardRestore::snapshot();
    FIXTURE_ALERTS.with_borrow_mut(|slot| *slot = Some(Vec::new()));
    let timer = setup(
        Preferences::load(temp.path().join("prefs.json")),
        Some(destination.clone()),
    );
    settle();
    let first = widgets();
    new_window(unsafe { &*first.delegate }, sel!(newWindow:), NIL);
    let second = WINDOWS.with_borrow(|windows| windows.last().unwrap().ui.get().unwrap());
    let _active_window = enter_window(unsafe { &*second.delegate }, NIL);
    settle();
    unsafe {
        let _: () = msg_send![second.window,makeKeyAndOrderFront:NIL];
    }
    pump();
    unsafe {
        let menu: Id = msg_send![first.table, menu];
        let sender = find_menu_item(menu, "Paste Items");
        let before = context();
        {
            let _guard = enter_window(&*first.delegate, sender);
            assert_eq!(
                widgets().window,
                first.window,
                "context menu must own its window, not focused window"
            );
            assert!(context_sender(sender));
        }
        assert!(Rc::ptr_eq(&before, &context()));
        let main: Id = msg_send![app(), mainMenu];
        let sender = find_menu_item(main, "New Tab");
        {
            let _guard = enter_window(&*first.delegate, sender);
            assert_eq!(
                widgets().window,
                second.window,
                "app menu must use focused window"
            );
            assert!(!context_sender(sender));
        }
        let pb: Id = msg_send![class!(NSPasteboard), generalPasteboard];
        let _: isize = msg_send![pb, clearContents];
        let ok: Bool = msg_send![pb,writeObjects:array(&[file_url(&file)])];
        assert!(ok.as_bool());
        restore.capture_change_count();
        let incoming = clipboard_paths(pb).unwrap();
        assert_eq!(incoming.len(), 1);
        assert_eq!(
            std::fs::canonicalize(&incoming[0]).unwrap(),
            std::fs::canonicalize(&file).unwrap()
        );
        let _: Bool = msg_send![second.window,makeFirstResponder:second.table];
        key("v", 1 << 20, 9);
        let deadline = Instant::now() + Duration::from_secs(5);
        while context().transfer.with_borrow(Option::is_some) {
            pump();
            assert!(Instant::now() < deadline, "transfer timeout");
        }
        let copied = destination.join(file.file_name().unwrap());
        assert_eq!(std::fs::read(&copied).unwrap(), b"original");
        assert!(model(|m| m
            .status
            .contains("Transferred 1 item(s); 0 failed")));
        std::fs::write(&copied, b"existing").unwrap();
        paste_files(Action::Copy);
        let deadline = Instant::now() + Duration::from_secs(5);
        while context().transfer.with_borrow(Option::is_some) {
            pump();
            assert!(Instant::now() < deadline, "conflict timeout");
        }
        assert!(model(|m| m.status.contains("0 item(s); 1 failed")));
        assert_eq!(std::fs::read(&copied).unwrap(), b"existing");
        assert_eq!(std::fs::read(&file).unwrap(), b"original");

        // Copy the actual list selection through the native responder chain.
        settle();
        let selected = context().display.with_borrow(|d| {
            d.paths
                .iter()
                .position(|p| std::fs::canonicalize(p).ok() == std::fs::canonicalize(&copied).ok())
                .unwrap()
        });
        let indexes: Id = msg_send![class!(NSIndexSet),indexSetWithIndex:selected];
        let _: () = msg_send![second.table,selectRowIndexes:indexes,byExtendingSelection:Bool::NO];
        key("c", 1 << 20, 8);
        restore.capture_change_count();
        assert_eq!(clipboard_paths(pb).unwrap().len(), 1);
        assert_eq!(
            std::fs::read(&clipboard_paths(pb).unwrap()[0]).unwrap(),
            b"existing"
        );

        // Command-Option-V explicitly moves a folder (and its contents).
        let move_folder = source.join("move folder");
        std::fs::create_dir(&move_folder).unwrap();
        std::fs::write(move_folder.join("nested.txt"), b"move content").unwrap();
        let _: isize = msg_send![pb, clearContents];
        let _: Bool = msg_send![pb,writeObjects:array(&[file_url(&move_folder)])];
        restore.capture_change_count();
        key("v", (1 << 20) | (1 << 19), 9);
        let deadline = Instant::now() + Duration::from_secs(5);
        while context().transfer.with_borrow(Option::is_some) {
            pump();
            assert!(Instant::now() < deadline, "move timeout");
        }
        assert!(!move_folder.exists());
        assert_eq!(
            std::fs::read(destination.join("move folder/nested.txt")).unwrap(),
            b"move content"
        );

        // Text Paste/Copy must remain text editing, not filesystem actions.
        let _: isize = msg_send![pb, clearContents];
        let _: Bool =
            msg_send![pb,setString:ns("text editor fixture"),forType:ns("public.utf8-plain-text")];
        restore.capture_change_count();
        focus(&*second.delegate, sel!(focus:), NIL);
        key("v", 1 << 20, 9);
        assert_eq!(value(second.path), "text editor fixture");
        let _: () = msg_send![second.path,selectText:NIL];
        key("c", 1 << 20, 8);
        restore.capture_change_count();
        let text: Id = msg_send![pb,stringForType:ns("public.utf8-plain-text")];
        assert_eq!(string(text), "text editor fixture");
        assert!(!destination.join("text editor fixture").exists());
        let _: Bool = msg_send![second.window,makeFirstResponder:second.table];

        let bad: Id = msg_send![class!(NSPasteboardItem), new];
        let _: Bool = msg_send![bad,setString:ns("https://example.invalid/file"),forType:ns("public.file-url")];
        let _: isize = msg_send![pb, clearContents];
        let _: Bool = msg_send![pb,writeObjects:array(&[file_url(&file), bad])];
        let _: () = msg_send![bad, release];
        restore.capture_change_count();
        assert!(clipboard_paths(pb).is_err());
        paste_files(Action::Move);
        assert!(context().transfer.with_borrow(Option::is_none));
        assert!(model(|m| m.status.contains("no items were transferred")));
        assert_eq!(std::fs::read(&file).unwrap(), b"original");

        // A held worker outcome must prevent application termination and cancel
        // remaining items, while keeping its window/receiver alive.
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel();
        context().transfer.with_borrow_mut(|slot| {
            *slot = Some(Transfer {
                cancel: cancel.clone(),
                reply: rx,
            })
        });
        assert_eq!(
            should_terminate(&*first.delegate, sel!(applicationShouldTerminate:), app()),
            0
        );
        assert!(cancel.load(Ordering::Acquire));
        assert!(!context().closed.get());
        assert!(context().transfer.with_borrow(Option::is_some));
        tx.send("Transferred 0 item(s); 0 failed; 1 not attempted.".into())
            .unwrap();
        tick(&*second.delegate, sel!(tick:), NIL);
        assert!(context().transfer.with_borrow(Option::is_none));
        assert_eq!(
            should_terminate(&*first.delegate, sel!(applicationShouldTerminate:), app()),
            1
        );
    }
    assert!(FIXTURE_ALERTS.with_borrow(|slot| slot
        .as_ref()
        .unwrap()
        .iter()
        .any(|m| m.starts_with("Invalid clipboard:"))));
    drop(_active_window);
    cleanup(timer);
    FIXTURE_ALERTS.with_borrow_mut(|slot| *slot = None);
    println!("PASS native clipboard: Unicode/escaped URLs, mixed batch rejection, owned context/app menus, native Command-C/V/Option-V, text editing, async copy/move/conflict and quit cancellation");
    Ok(())
}
