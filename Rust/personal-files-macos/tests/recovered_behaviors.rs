use personal_files::{
    core::{Preferences, Sort},
    model::{Browser, TabbedBrowser},
};
use std::{fs, os::unix::fs::PermissionsExt, path::Path, time::Instant};

fn settle(browser: &mut Browser) {
    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    while browser.is_loading() {
        browser.tick();
        assert!(Instant::now() < deadline, "directory worker timed out");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}

fn start(prefs: &Path, path: Option<&Path>) -> TabbedBrowser {
    let mut tabs = TabbedBrowser::new(Browser::new(
        Preferences::load(prefs.into()),
        path.map(Path::to_path_buf),
    ));
    settle(&mut tabs.browser);
    tabs
}

#[test]
fn unreadable_preferences_are_never_replaced_by_defaults() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("preferences.json");
    let original = br#"{"theme":"aurora"}"#;
    fs::write(&path, original).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o0)).unwrap();
    let mut prefs = Preferences::load(path.clone());
    assert!(prefs.warning.as_deref().unwrap().contains("Could not load"));
    let save = prefs.save();
    let mut tabs = TabbedBrowser::new(Browser::new(prefs, Some(temp.path().into())));
    settle(&mut tabs.browser);
    tabs.browser.toggle_favorite();
    tabs.browser.save();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(
        save.is_err(),
        "storage must reject saving an unreadable original"
    );
    assert_eq!(fs::read(&path).unwrap(), original);
    assert!(tabs.browser.status.contains("Could not save preferences"));
    let mut reloaded = Preferences::load(path.clone());
    assert!(reloaded.warning.is_none());
    assert_eq!(reloaded.data.theme, "aurora");
    reloaded.favorite(temp.path());
    reloaded.save().unwrap();
    assert_eq!(Preferences::load(path).data.theme, "aurora");
}

#[test]
fn tabs_restore_order_active_folder_and_independent_sorting() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    fs::write(a.join("fixture"), "unchanged").unwrap();
    let prefs = temp.path().join("preferences.json");
    let mut tabs = start(&prefs, Some(&a));
    tabs.browser.sort(Sort::Size);
    tabs.browser.sort(Sort::Size);
    tabs.open("unfinished draft".into());
    tabs.browser.navigate(b.to_str().unwrap());
    settle(&mut tabs.browser);
    tabs.browser.sort(Sort::Modified);
    tabs.select(0, "second draft".into());
    settle(&mut tabs.browser);
    assert_eq!(tabs.browser.prefs.data.sort, Sort::Size);
    assert!(tabs.browser.prefs.data.descending);
    drop(tabs);

    let mut restored = start(&prefs, None);
    assert_eq!(restored.count(), 2);
    assert_eq!(restored.active(), 0);
    assert_eq!(restored.labels(), vec!["a", "b"]);
    assert_eq!(restored.browser.current, a);
    assert_eq!(restored.browser.prefs.data.sort, Sort::Size);
    assert!(restored.browser.prefs.data.descending);
    assert!(restored.browser.selected.is_empty());
    assert_eq!(restored.browser.history, vec![a.clone()]);
    restored.select(1, "".into());
    settle(&mut restored.browser);
    assert_eq!(restored.browser.current, b);
    assert_eq!(restored.browser.prefs.data.sort, Sort::Modified);
    assert!(!restored.browser.prefs.data.descending);
    drop(restored);
    assert_eq!(start(&prefs, None).active(), 1);
    assert_eq!(fs::read_to_string(a.join("fixture")).unwrap(), "unchanged");
}

#[test]
fn restored_tab_switching_does_not_add_visits_and_closed_tabs_stay_closed() {
    let temp = tempfile::tempdir().unwrap();
    let a = temp.path().join("a");
    let b = temp.path().join("b");
    fs::create_dir(&a).unwrap();
    fs::create_dir(&b).unwrap();
    let prefs = temp.path().join("preferences.json");
    let mut tabs = start(&prefs, Some(&a));
    tabs.open("".into());
    tabs.browser.navigate(b.to_str().unwrap());
    settle(&mut tabs.browser);
    drop(tabs);
    let mut tabs = start(&prefs, None);
    assert_eq!(tabs.count(), 2);
    let visits = serde_json::to_value(&tabs.browser.prefs.data.visits).unwrap();
    for index in [0, 1, 0, 1] {
        tabs.select(index, "".into());
        settle(&mut tabs.browser);
    }
    assert_eq!(
        serde_json::to_value(&tabs.browser.prefs.data.visits).unwrap(),
        visits
    );
    tabs.close().unwrap();
    settle(&mut tabs.browser);
    drop(tabs);
    let tabs = start(&prefs, None);
    assert_eq!(tabs.count(), 1);
    assert_eq!(tabs.browser.current, a);
}

#[test]
fn legacy_preferences_and_saved_sessions_remain_compatible() {
    let temp = tempfile::tempdir().unwrap();
    let prefs = temp.path().join("preferences.json");
    fs::write(
        &prefs,
        serde_json::to_vec(&serde_json::json!({
            "favorites": [temp.path()], "last_path": temp.path(), "sort": "size",
            "descending": true,
            "tabs": [{"path": temp.path(), "sort": "modified", "descending": false}],
            "active_tab": 0
        }))
        .unwrap(),
    )
    .unwrap();
    let tabs = start(&prefs, None);
    assert_eq!(tabs.browser.prefs.data.sort, Sort::Modified);
    assert_eq!(tabs.browser.prefs.data.favorites, vec![temp.path()]);
    drop(tabs);
    let saved: serde_json::Value = serde_json::from_slice(&fs::read(&prefs).unwrap()).unwrap();
    assert_eq!(saved["tabs"][0]["path"], temp.path().to_str().unwrap());
    assert_eq!(saved["active_tab"], 0);

    fs::write(
        &prefs,
        serde_json::to_vec(&serde_json::json!({
            "favorites": [temp.path()], "last_path": temp.path(), "sort": "size",
            "descending": true
        }))
        .unwrap(),
    )
    .unwrap();
    let tabs = start(&prefs, None);
    assert_eq!(tabs.count(), 1);
    assert_eq!(tabs.browser.current, temp.path());
    assert_eq!(tabs.browser.prefs.data.sort, Sort::Size);
    assert!(tabs.browser.prefs.data.descending);
}

#[test]
fn missing_saved_folder_falls_back_without_discarding_other_tabs_or_favorites() {
    let temp = tempfile::tempdir().unwrap();
    let prefs = temp.path().join("preferences.json");
    fs::write(
        &prefs,
        serde_json::to_vec(&serde_json::json!({
            "favorites": [temp.path()], "last_path": temp.path(),
            "tabs": [{"path": "relative"}, {"path": temp.path()},
                     {"path": temp.path().join("missing"), "sort": "size"}],
            "active_tab": 2
        }))
        .unwrap(),
    )
    .unwrap();
    let mut tabs = start(&prefs, None);
    assert_eq!(tabs.count(), 2);
    assert_eq!(tabs.active(), 1);
    assert_eq!(tabs.browser.current, personal_files::core::home());
    assert_eq!(tabs.browser.prefs.data.favorites, vec![temp.path()]);
    tabs.select(0, "".into());
    settle(&mut tabs.browser);
    assert_eq!(tabs.browser.current, temp.path());
}

#[test]
fn explicit_path_starts_one_tab_and_failed_navigation_keeps_saved_location() {
    let temp = tempfile::tempdir().unwrap();
    let prefs = temp.path().join("preferences.json");
    fs::write(
        &prefs,
        serde_json::to_vec(&serde_json::json!({
            "last_path": temp.path(), "tabs": [{"path": "/tmp"}, {"path": "/"}],
            "active_tab": 1
        }))
        .unwrap(),
    )
    .unwrap();
    let mut tabs = start(&prefs, Some(temp.path()));
    assert_eq!(tabs.count(), 1);
    let before = fs::read(&prefs).unwrap();
    tabs.browser
        .navigate(temp.path().join("missing").to_str().unwrap());
    settle(&mut tabs.browser);
    assert_eq!(tabs.browser.current, temp.path());
    assert_eq!(fs::read(&prefs).unwrap(), before);
    drop(tabs);
    assert_eq!(start(&prefs, None).count(), 1);
}
