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
