//! Window-manager verification must return to NSApplication's outer event loop
//! between actions. A nested synchronous test cannot flush native tiling work.
use super::*;

struct Scenario {
    _temp: tempfile::TempDir,
    browser_timer: Id,
    timer: Id,
    menu: Id,
    stage: usize,
    action: usize,
    editor_focus: bool,
    original: Rect,
    before_action: Rect,
    visible: Rect,
    before: Vec<u8>,
    draft: String,
    began: Instant,
    dispatched: Instant,
}
thread_local! { static SCENARIO: RefCell<Option<Scenario>> = const { RefCell::new(None) }; }
const ACTIONS: [&str; 7] = [
    "Center",
    "Fill",
    "Left",
    "Right",
    "Top",
    "Bottom",
    "Return to Previous Size",
];

pub(super) fn start() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("tiling-fixture");
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join(".fixture"), "unchanged").unwrap();
    let browser_timer = setup(
        Preferences::load(temp.path().join("preferences.json")),
        Some(root),
    );
    let w = widgets();
    unsafe {
        let original: Rect = msg_send![w.window, frame];
        let screen: Id = msg_send![w.window, screen];
        let visible: Rect = msg_send![screen, visibleFrame];
        let menu: Id = msg_send![app(), windowsMenu];
        assert!(!menu.is_null(), "native Window menu missing");
        let timer: Id = msg_send![class!(NSTimer),scheduledTimerWithTimeInterval:0.15f64,target:w.delegate,selector:sel!(asyncTilingStep:),userInfo:NIL,repeats:Bool::YES];
        SCENARIO.with_borrow_mut(|s| {
            *s = Some(Scenario {
                _temp: temp,
                browser_timer,
                timer,
                menu,
                stage: 0,
                action: 0,
                editor_focus: false,
                original,
                before_action: original,
                visible,
                before: vec![],
                draft: "unfinished path draft".into(),
                began: Instant::now(),
                dispatched: Instant::now(),
            })
        });
    }
}
fn find(menu: Id, title: &str) -> Id {
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
                let found = find(submenu, title);
                if !found.is_null() {
                    return found;
                }
            }
        }
        NIL
    }
}
fn synth_key(key: &str, keycode: u16) {
    unsafe {
        let w = widgets();
        let number: isize = msg_send![w.window, windowNumber];
        let event: Id = msg_send![class!(NSEvent),keyEventWithType:10usize,location:Point{x:0.,y:0.},modifierFlags:(1usize<<23)|(1usize<<18),timestamp:0f64,windowNumber:number,context:NIL,characters:ns(key),charactersIgnoringModifiers:ns(key),isARepeat:Bool::NO,keyCode:keycode];
        let main: Id = msg_send![app(), mainMenu];
        let handled: Bool = msg_send![main,performKeyEquivalent:event];
        assert!(
            handled.as_bool(),
            "native Fn-Control-{key} key equivalent was not handled"
        );
    }
}
fn browser_state() -> Vec<u8> {
    model(|m| serde_json::to_vec(&(m.current.clone(), m.selected.clone(), &m.prefs.data)).unwrap())
}
fn advance(s: &mut Scenario) -> bool {
    assert!(
        s.began.elapsed() < Duration::from_secs(30),
        "asynchronous tiling fixture timed out"
    );
    let w = widgets();
    unsafe {
        match s.stage {
            0 => {
                if model(|m| m.is_loading()) {
                    return false;
                }
                s.before = browser_state();
                set_value(w.path, &s.draft);
                let _: () = msg_send![w.window, makeKeyWindow];
                let _: () = msg_send![w.window, makeMainWindow];
                let timer: Id = msg_send![class!(NSTimer),scheduledTimerWithTimeInterval:0.1f64,target:w.delegate,selector:sel!(cancelTestMenu:),userInfo:s.menu,repeats:Bool::NO];
                let loop_: Id = msg_send![class!(NSRunLoop), mainRunLoop];
                let _: () = msg_send![loop_,addTimer:timer,forMode:NSRunLoopCommonModes];
                let content: Id = msg_send![w.window, contentView];
                let _: Bool = msg_send![s.menu,popUpMenuPositioningItem:NIL,atLocation:Point{x:12.,y:12.},inView:content];
                s.stage = 1;
            }
            1 => {
                let _: () = msg_send![app(),activateIgnoringOtherApps:Bool::YES];
                let _: () = msg_send![w.window, makeKeyWindow];
                let _: () = msg_send![w.window, makeMainWindow];
                let focused: Bool = msg_send![w.window,makeFirstResponder:if s.editor_focus {w.path} else {w.table}];
                assert!(
                    focused.as_bool(),
                    "could not focus requested native control"
                );
                let first: Id = msg_send![w.window, firstResponder];
                if s.editor_focus {
                    let editor: Id = msg_send![w.path, currentEditor];
                    assert!(!editor.is_null(), "path field has no active field editor");
                    assert_eq!(first, editor, "wrong responder for path-focus tiling");
                } else {
                    assert_eq!(first, w.table, "wrong responder for file-list-focus tiling");
                }
                let key: Id = msg_send![app(), keyWindow];
                if key != w.window {
                    assert!(
                        s.dispatched.elapsed() < Duration::from_secs(3),
                        "native tiling blocked: cannot acquire foreground key window"
                    );
                    return false;
                }
                let title = ACTIONS[s.action];
                let item = find(s.menu, title);
                assert!(!item.is_null(), "OS did not provide tiling action {title}");
                s.before_action = msg_send![w.window, frame];
                if title == "Fill" {
                    synth_key("f", 3);
                } else if title == "Center" {
                    synth_key("c", 8);
                } else if s.action == 6 {
                    synth_key("r", 15);
                } else {
                    let menu: Id = msg_send![item, menu];
                    let index: isize = msg_send![menu,indexOfItem:item];
                    let _: () = msg_send![menu,performActionForItemAtIndex:index];
                }
                s.dispatched = Instant::now();
                s.stage = 2;
            }
            2 => {
                // Wait beyond the native zoom animation, not merely until an
                // intermediate frame crosses our size predicate.
                if s.dispatched.elapsed() < Duration::from_millis(600) {
                    return false;
                }
                let f: Rect = msg_send![w.window, frame];
                let v = s.visible;
                let close = |a: f64, b: f64| (a - b).abs() < 40.;
                let frame_matches = |a: Rect, b: Rect| {
                    close(a.origin.x, b.origin.x)
                        && close(a.origin.y, b.origin.y)
                        && close(a.size.width, b.size.width)
                        && close(a.size.height, b.size.height)
                };
                let matches = match ACTIONS[s.action] {
                    "Fill" => frame_matches(f, v),
                    "Center" => {
                        close(
                            f.origin.x + f.size.width / 2.,
                            v.origin.x + v.size.width / 2.,
                        ) && close(
                            f.origin.y + f.size.height / 2.,
                            v.origin.y + v.size.height / 2.,
                        ) && (f.size.width - s.before_action.size.width).abs() < 8.
                            && (f.size.height - s.before_action.size.height).abs() < 8.
                    }
                    "Left" => frame_matches(
                        f,
                        rect(v.origin.x, v.origin.y, v.size.width / 2., v.size.height),
                    ),
                    "Right" => frame_matches(
                        f,
                        rect(
                            v.origin.x + v.size.width / 2.,
                            v.origin.y,
                            v.size.width / 2.,
                            v.size.height,
                        ),
                    ),
                    "Top" => frame_matches(
                        f,
                        rect(
                            v.origin.x,
                            v.origin.y + v.size.height / 2.,
                            v.size.width,
                            v.size.height / 2.,
                        ),
                    ),
                    "Bottom" => frame_matches(
                        f,
                        rect(v.origin.x, v.origin.y, v.size.width, v.size.height / 2.),
                    ),
                    _ => frame_matches(f, s.original),
                };
                if !matches {
                    assert!(
                        s.dispatched.elapsed() < Duration::from_secs(3),
                        "native {} failed: frame=({}, {}, {}, {}), desktop=({}, {}, {}, {})",
                        ACTIONS[s.action],
                        f.origin.x,
                        f.origin.y,
                        f.size.width,
                        f.size.height,
                        v.origin.x,
                        v.origin.y,
                        v.size.width,
                        v.size.height
                    );
                    return false;
                }
                let style: usize = msg_send![w.window, styleMask];
                assert_eq!(style & (1 << 14), 0, "desktop tiling entered full screen");
                assert_eq!(browser_state(), s.before, "tiling mutated browser state");
                assert_eq!(value(w.path), s.draft, "tiling changed path draft");
                println!(
                    "PASS native {}: editor_focus={}, frame={}x{}",
                    ACTIONS[s.action], s.editor_focus, f.size.width, f.size.height
                );
                // Center is checked from a normal window before filling, so
                // size-preservation is meaningful and native restore has a
                // centered reference frame (including position) to return to.
                if ACTIONS[s.action] == "Center" {
                    s.original = f;
                }
                s.action += 1;
                if s.action == ACTIONS.len() {
                    if !s.editor_focus {
                        s.editor_focus = true;
                        s.action = 0;
                        s.stage = 1;
                    } else {
                        let _: () = msg_send![w.window,setContentSize:Size{width:480.,height:280.}];
                        layout();
                        let path: Rect = msg_send![w.path, frame];
                        assert!(path.size.width >= 180.);
                        let files: Id = msg_send![w.table, enclosingScrollView];
                        let frame: Rect = msg_send![files, frame];
                        assert!(frame.size.width >= 250. && frame.size.height >= 160.);
                        return true;
                    }
                } else {
                    s.stage = 1;
                }
            }
            _ => unreachable!(),
        }
    }
    false
}
pub(super) extern "C" fn step(_: &AnyObject, _: Sel, _: Id) {
    let result =
        std::panic::catch_unwind(|| SCENARIO.with_borrow_mut(|s| advance(s.as_mut().unwrap())));
    let code = match result {
        Ok(false) => return,
        Ok(true) => {
            SCENARIO.with_borrow_mut(|s| {
                let scenario = s.take().unwrap();
                unsafe {
                    let _: () = msg_send![scenario.timer, invalidate];
                }
                cleanup(scenario.browser_timer);
            });
            println!("PASS asynchronous native tiling: real geometry, Fn-Control-F/C/R key equivalents, OS half-screen actions, both focus modes, restore and compact layout");
            0
        }
        Err(_) => 1,
    };
    use std::io::Write;
    let _ = std::io::stdout().flush();
    let _ = std::io::stderr().flush();
    std::process::exit(code);
}
