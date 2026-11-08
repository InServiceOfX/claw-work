use personal_files::{
    appearance,
    core::{self, Entry, Preferences, Sort},
};
use std::{
    fs,
    os::unix::fs::{symlink, PermissionsExt},
    path::{Path, PathBuf},
    process::Command,
};
use tempfile::TempDir;

fn entry(root: &Path, name: &str, folder: bool, size: u64, modified: u64) -> Entry {
    Entry {
        path: root.join(name),
        is_dir: folder,
        size: Some(size),
        modified,
        symlink: false,
    }
}

#[test]
fn absolute_relative_home_and_parent_paths() {
    let temp = TempDir::new().unwrap();
    let root = temp.path();
    assert_eq!(core::location("a/../b", root).unwrap(), root.join("b"));
    assert_eq!(core::location("/tmp/../", root).unwrap(), Path::new("/"));
    assert_eq!(
        core::location("~/Documents", root).unwrap(),
        core::home().join("Documents")
    );
    assert!(core::location("", root).is_err());
}

#[test]
fn symlink_alias_spaces_and_unicode_preserved() {
    let t = TempDir::new().unwrap();
    let root = t.path();
    fs::create_dir(root.join("real")).unwrap();
    symlink(root.join("real"), root.join("workspace2")).unwrap();
    assert_eq!(
        core::location("workspace2/long name/物理", root).unwrap(),
        root.join("workspace2/long name/物理")
    );
}

#[test]
fn hidden_files_and_broken_links_visible() {
    let t = TempDir::new().unwrap();
    fs::write(t.path().join(".hidden"), "fixture").unwrap();
    fs::create_dir(t.path().join("folder")).unwrap();
    symlink(t.path().join("missing"), t.path().join("broken")).unwrap();
    let items = core::entries(t.path()).unwrap();
    assert_eq!(items.len(), 3);
    assert!(items.iter().any(|e| e.name() == ".hidden"));
    assert!(items
        .iter()
        .any(|e| e.name() == "broken" && e.size.is_none()));
}

#[test]
fn missing_and_permission_failures_leave_sources_unchanged() {
    let t = TempDir::new().unwrap();
    assert!(core::entries(&t.path().join("missing")).is_err());
    let locked = t.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o0)).unwrap();
    let result = core::entries(&locked);
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert!(!t.path().join("missing").exists());
}

#[test]
fn all_three_sorts_reverse_with_folders_first() {
    let root = Path::new("/fixture");
    let values = vec![
        entry(root, "zFolder", true, 1, 2),
        entry(root, "b", false, 4, 10),
        entry(root, "A", false, 12, 1),
    ];
    for (key, names) in [
        (Sort::Name, vec!["A", "b"]),
        (Sort::Size, vec!["b", "A"]),
        (Sort::Modified, vec!["A", "b"]),
    ] {
        for reverse in [false, true] {
            let mut items = values.clone();
            core::sort_entries(&mut items, key, reverse);
            let mut wanted = names.clone();
            if reverse {
                wanted.reverse();
            }
            wanted.insert(0, "zFolder");
            assert_eq!(items.iter().map(Entry::name).collect::<Vec<_>>(), wanted);
        }
    }
}

#[test]
fn clipboard_plain_and_shell_paths_roundtrip_without_execution() {
    let t = TempDir::new().unwrap();
    let paths = vec![
        t.path().join("a b's $notes;$(touch SHOULD_NOT_EXIST)"),
        t.path().join("line\nbreak"),
    ];
    let quoted = core::clipboard_text(&paths, true);
    let result = Command::new("/bin/sh")
        .arg("-c")
        .arg(format!("set -- {quoted}; printf '%s\\0' \"$@\""))
        .current_dir(t.path())
        .output()
        .unwrap();
    assert!(result.status.success());
    let fields: Vec<_> = result
        .stdout
        .split(|b| *b == 0)
        .filter(|s| !s.is_empty())
        .map(|s| String::from_utf8(s.to_vec()).unwrap())
        .collect();
    assert_eq!(
        fields,
        paths
            .iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect::<Vec<_>>()
    );
    assert!(!t.path().join("SHOULD_NOT_EXIST").exists());
    assert_eq!(
        core::clipboard_text(&paths, false),
        format!("{}\n{}", paths[0].display(), paths[1].display())
    );
}

#[test]
fn favorites_visits_sort_and_skin_survive_restart() {
    let t = TempDir::new().unwrap();
    let file = t.path().join("config/preferences.json");
    let mut p = Preferences::load(file.clone());
    p.favorite(t.path());
    p.favorite(t.path());
    p.visit(t.path());
    p.visit(t.path());
    p.data.theme = "aurora".into();
    p.data.image = PathBuf::from("/tmp/a background.png");
    p.data.sort = Sort::Size;
    p.data.descending = true;
    p.save().unwrap();
    let saved = Preferences::load(file.clone());
    assert_eq!(saved.data.favorites, vec![t.path()]);
    assert_eq!(saved.data.visits[t.path()].count, 2);
    assert_eq!(saved.data.last_path, t.path());
    assert_eq!(saved.data.theme, "aurora");
    assert_eq!(saved.data.sort, Sort::Size);
    assert!(saved.data.descending);
    assert_eq!(saved.data.image, Path::new("/tmp/a background.png"));
    assert_eq!(
        fs::metadata(file).unwrap().permissions().mode() & 0o777,
        0o600
    );
}

#[test]
fn recent_and_frequent_separate_from_explicit_favorites() {
    let t = TempDir::new().unwrap();
    let mut p = Preferences::load(t.path().join("state.json"));
    let a = t.path().join("a");
    let b = t.path().join("b");
    p.visit(&a);
    p.visit(&a);
    p.visit(&b);
    assert_eq!(p.recent(10), vec![b.clone(), a.clone()]);
    assert_eq!(p.frequent(10), vec![a.clone(), b.clone()]);
    assert!(p.data.favorites.is_empty());
    p.favorite(&a);
    p.data.visits.clear();
    assert_eq!(p.data.favorites, vec![a.clone()]);
    p.unfavorite(&a);
    assert!(p.data.favorites.is_empty());
}

#[test]
fn bounded_history_does_not_evict_manual_favorites() {
    let t = TempDir::new().unwrap();
    let mut p = Preferences::load(t.path().join("state.json"));
    p.favorite(&t.path().join("0"));
    for n in 0..210 {
        p.visit(&t.path().join(n.to_string()));
    }
    assert_eq!(p.data.visits.len(), 200);
    assert!(!p.data.visits.contains_key(&t.path().join("0")));
    assert_eq!(p.data.favorites, vec![t.path().join("0")]);
}

#[test]
fn corrupt_preferences_are_backed_up_before_recovery() {
    let t = TempDir::new().unwrap();
    let file = t.path().join("preferences.json");
    fs::write(&file, "{bad json").unwrap();
    let mut p = Preferences::load(file.clone());
    assert!(p.warning.is_some());
    p.favorite(t.path());
    p.save().unwrap();
    let backup = fs::read_dir(t.path())
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| p.to_string_lossy().contains(".corrupt-"))
        .unwrap();
    assert_eq!(fs::read_to_string(backup).unwrap(), "{bad json");
    assert_eq!(Preferences::load(file).data.favorites, vec![t.path()]);
}

#[test]
fn invalid_schema_recovers_safely() {
    let t = TempDir::new().unwrap();
    let file = t.path().join("state.json");
    for invalid in [
        "[]",
        r#"{"favorites":[1]}"#,
        r#"{"favorites":["relative"]}"#,
        r#"{"visits":{"/tmp":{"count":0,"last":0}}}"#,
    ] {
        fs::write(&file, invalid).unwrap();
        let p = Preferences::load(file.clone());
        assert!(p.warning.is_some(), "{invalid}");
        assert!(p.data.favorites.is_empty());
    }
}

#[test]
fn disconnected_favorite_is_retained() {
    let t = TempDir::new().unwrap();
    let file = t.path().join("state.json");
    let missing = t.path().join("unmounted");
    let mut p = Preferences::load(file.clone());
    p.favorite(&missing);
    p.save().unwrap();
    assert_eq!(Preferences::load(file).data.favorites, vec![missing]);
}

#[test]
fn auto_theme_uses_file_types_and_silver_fallback() {
    let root = Path::new("/fixture");
    for (name, theme) in [
        ("book.pdf", "silver"),
        ("source.rs", "midnight"),
        ("photo.jpg", "aurora"),
    ] {
        assert_eq!(
            appearance::effective_theme("auto", &[entry(root, name, false, 1, 1)]),
            theme
        );
    }
    assert_eq!(appearance::effective_theme("auto", &[]), "silver");
    assert_eq!(
        appearance::effective_theme("silver", &[entry(root, "photo.jpg", false, 1, 1)]),
        "silver"
    );
    let css = appearance::stylesheet("silver", Path::new("/tmp/a b's.png"));
    assert!(css.contains("a%20b%27s.png"));
}

#[test]
fn size_labels_do_not_invent_folder_totals() {
    assert_eq!(core::size_label(None), "—");
    assert_eq!(core::size_label(Some(0)), "0 B");
    assert_eq!(core::size_label(Some(1024)), "1.0 KB");
}

#[test]
fn failed_save_does_not_clobber_existing_preferences() {
    let t = TempDir::new().unwrap();
    let file = t.path().join("prefs.json");
    let mut p = Preferences::load(file.clone());
    p.favorite(t.path());
    p.save().unwrap();
    let before = fs::read(&file).unwrap();
    fs::set_permissions(t.path(), fs::Permissions::from_mode(0o500)).unwrap();
    let result = p.save();
    fs::set_permissions(t.path(), fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert_eq!(fs::read(file).unwrap(), before);
}

#[test]
fn legacy_preferences_load_without_tabs() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("preferences.json");
    fs::write(
        &path,
        r#"{"favorites":["/tmp"],"last_path":"/tmp","sort":"size","descending":true}"#,
    )
    .unwrap();
    let prefs = Preferences::load(path);
    assert!(prefs.warning.is_none());
    assert!(prefs.data.tabs.is_empty());
    assert_eq!(prefs.data.favorites, vec![PathBuf::from("/tmp")]);
    assert_eq!(prefs.data.sort, Sort::Size);
    assert!(prefs.data.descending);
}

#[test]
fn tab_order_active_index_and_per_tab_sort_survive_restart() {
    use personal_files::core::SavedTab;
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("preferences.json");
    let mut prefs = Preferences::load(path.clone());
    prefs.data.tabs = vec![
        SavedTab {
            path: temp.path().join("first"),
            sort: Sort::Name,
            descending: false,
        },
        SavedTab {
            path: temp.path().join("second"),
            sort: Sort::Size,
            descending: true,
        },
    ];
    prefs.data.active_tab = 1;
    prefs.save().unwrap();
    let restored = Preferences::load(path);
    assert_eq!(restored.data.tabs.len(), 2);
    assert_eq!(restored.data.tabs[0].path, temp.path().join("first"));
    assert_eq!(restored.data.tabs[1].path, temp.path().join("second"));
    assert_eq!(restored.data.tabs[1].sort, Sort::Size);
    assert!(restored.data.tabs[1].descending);
    assert_eq!(restored.data.active_tab, 1);
}

#[test]
fn invalid_tab_paths_and_active_index_do_not_discard_favorites() {
    let temp = TempDir::new().unwrap();
    let path = temp.path().join("preferences.json");
    fs::write(&path,r#"{"favorites":["/tmp"],"tabs":[{"path":"relative"},{"path":"/unmounted/folder"}],"active_tab":999}"#).unwrap();
    let prefs = Preferences::load(path);
    assert!(prefs.warning.is_none());
    assert_eq!(prefs.data.tabs.len(), 1);
    assert_eq!(prefs.data.tabs[0].path, Path::new("/unmounted/folder"));
    assert_eq!(prefs.data.active_tab, 0);
    assert_eq!(prefs.data.favorites, vec![PathBuf::from("/tmp")]);
}
