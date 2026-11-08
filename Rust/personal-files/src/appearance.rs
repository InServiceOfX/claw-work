use crate::core::Entry;
use gtk::{gio, prelude::*};
use std::path::Path;

pub fn effective_theme(theme: &str, items: &[Entry]) -> String {
    if theme != "auto" {
        return theme.into();
    }
    let suffixes: Vec<_> = items
        .iter()
        .filter(|e| !e.is_dir)
        .map(|e| {
            e.path
                .extension()
                .unwrap_or_default()
                .to_string_lossy()
                .to_lowercase()
        })
        .collect();
    if suffixes.is_empty() {
        return "silver".into();
    }
    let pictures = suffixes
        .iter()
        .filter(|s| ["png", "jpg", "jpeg", "webp", "svg", "gif"].contains(&s.as_str()))
        .count();
    let code = suffixes
        .iter()
        .filter(|s| {
            ["py", "rs", "js", "ts", "cpp", "c", "sh", "json", "toml"].contains(&s.as_str())
        })
        .count();
    if pictures * 2 > suffixes.len() {
        "aurora"
    } else if code * 2 > suffixes.len() {
        "midnight"
    } else {
        "silver"
    }
    .into()
}

pub fn stylesheet(theme: &str, image: &Path) -> String {
    let (base, sidebar, ink, muted, panel, hover) = match theme {
        "midnight" => (
            "#151c2c",
            "#1c263a",
            "#ecf1ff",
            "#a4b3cf",
            "rgba(21,28,44,0.86)",
            "#263651",
        ),
        "aurora" => (
            "#152b37",
            "#1a3945",
            "#eafffa",
            "#a4c9d2",
            "rgba(16,40,49,0.82)",
            "#285363",
        ),
        _ => (
            "#f5f6fa",
            "#e9ecf3",
            "#202b40",
            "#627089",
            "rgba(255,255,255,0.84)",
            "#edf1f8",
        ),
    };
    let background = if image.as_os_str().is_empty() {
        "background-image: none;".into()
    } else {
        // URI encoding plus apostrophe escaping prevents filenames becoming CSS.
        let uri = gio::File::for_path(image).uri().replace('\'', "%27");
        format!(
            "background-image: url('{uri}'); background-size: cover; background-position: center;"
        )
    };
    format!(
        r#"
    window {{ background-color: {base}; color: {ink}; }}
    headerbar {{ background: {sidebar}; color: {ink}; box-shadow: none; border-bottom: 1px solid alpha({muted},0.18); }}
    headerbar label {{ color: {ink}; }}
    headerbar button.titlebutton {{ border: 0; border-radius: 50%; padding: 1px; min-width: 16px; min-height: 16px; margin: 0 2px; color: alpha(#172333,0.65); }}
    headerbar button.titlebutton.close {{ background: #ff625c; }}
    headerbar button.titlebutton.minimize {{ background: #ffbd44; }}
    headerbar button.titlebutton.maximize {{ background: #00ca4e; }}
    #sidebar {{ background: {sidebar}; padding: 12px 6px; }}
    #sidebar list, #sidebar row {{ background: transparent; color: {ink}; }}
    #sidebar row {{ border-radius: 7px; margin: 1px 4px; padding: 5px; }}
    #sidebar row:hover {{ background: {hover}; }}
    .section {{ color: {muted}; font-size: 10px; font-weight: bold; letter-spacing: 1px; padding: 10px 12px 4px; }}
    .hint {{ color: {muted}; font-size: 11px; }}
    #toolbar {{ padding: 10px 14px; background: {base}; }}
    #tabs {{ background: {base}; }}
    #tabs > header {{ background: {sidebar}; border-bottom: 1px solid alpha({muted},0.2); }}
    #tabs tab {{ padding: 4px 12px; color: {muted}; }}
    #tabs tab:checked {{ background: {base}; color: {ink}; box-shadow: inset 0 -2px #3879df; }}
    #tabs tab button {{ padding: 0; min-width: 18px; min-height: 18px; border: 0; background: transparent; }}
    entry {{ background: {panel}; color: {ink}; border: 1px solid alpha({muted},0.28); border-radius: 7px; min-height: 29px; padding: 3px 10px; }}
    entry selection {{ background: #3879df; color: white; }}
    button {{ background: {panel}; color: {ink}; border: 1px solid alpha({muted},0.22); border-radius: 6px; box-shadow: none; text-shadow: none; padding: 5px 9px; }}
    button:hover {{ background: {hover}; }}
    button:disabled {{ opacity: 0.4; }}
    #canvas {{ background-color: {base}; {background} }}
    treeview.view {{ background: {panel}; color: {ink}; }}
    treeview.view:selected {{ background: #3879df; color: white; }}
    treeview.view header button {{ background: {base}; border-radius: 0; border-width: 0 0 1px 0; color: {muted}; padding: 9px; }}
    #status {{ padding: 8px 16px; background: {base}; }}
    menu, menuitem, popover {{ background: {base}; color: {ink}; }}
    menuitem:hover {{ background: #3879df; color: white; }}
    tooltip {{ background: {sidebar}; color: {ink}; }}
    "#
    )
}
