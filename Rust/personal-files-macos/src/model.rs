//! Platform-independent state machine. One worker reads metadata, never contents.
use crate::core::{self, Entry, Preferences, SavedTab, Sort};
use std::{
    collections::BTreeSet,
    io,
    path::{Path, PathBuf},
    sync::mpsc::{self, Receiver, Sender},
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
enum Reason {
    Navigate,
    History(usize),
    Refresh,
    Startup,
    Restore,
}
struct Request {
    id: u64,
    path: PathBuf,
    reason: Reason,
}
struct Reply {
    request: Request,
    result: io::Result<(PathBuf, Vec<Entry>, Option<PathBuf>)>,
}

pub struct Browser {
    pub prefs: Preferences,
    pub current: PathBuf,
    pub items: Vec<Entry>,
    pub selected: BTreeSet<PathBuf>,
    pub status: String,
    pub history: Vec<PathBuf>,
    pub cursor: usize,
    sender: Sender<Request>,
    receiver: Receiver<Reply>,
    generation: u64,
    pending: bool,
    pending_request: Option<(PathBuf, Reason)>,
    last_poll: Instant,
    restore_session: bool,
}
impl Browser {
    pub fn new(mut prefs: Preferences, start: Option<PathBuf>) -> Self {
        let restore_session = start.is_none() && !prefs.data.tabs.is_empty();
        let initial = if restore_session {
            let tab = &prefs.data.tabs[prefs.data.active_tab];
            prefs.data.sort = tab.sort;
            prefs.data.descending = tab.descending;
            tab.path.clone()
        } else {
            if start.is_some() {
                prefs.data.tabs.clear();
                prefs.data.active_tab = 0;
            }
            start.unwrap_or_else(|| prefs.data.last_path.clone())
        };
        let (sender, requests) = mpsc::channel::<Request>();
        let (results, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("directory-scanner".into())
            .spawn(move || {
                while let Ok(mut request) = requests.recv() {
                    while let Ok(newer) = requests.try_recv() {
                        request = newer;
                    }
                    let result = (|| {
                        let metadata = std::fs::metadata(&request.path)?;
                        if matches!(request.reason, Reason::Refresh) && !metadata.is_dir() {
                            return Err(io::Error::new(
                                io::ErrorKind::NotADirectory,
                                "The displayed folder was replaced by a file.",
                            ));
                        }
                        let (directory, selection) = if metadata.is_dir() {
                            (request.path.clone(), None)
                        } else {
                            (
                                request
                                    .path
                                    .parent()
                                    .unwrap_or(Path::new("/"))
                                    .to_path_buf(),
                                Some(request.path.clone()),
                            )
                        };
                        let items = core::entries(&directory)?;
                        Ok((directory, items, selection))
                    })();
                    if results.send(Reply { request, result }).is_err() {
                        break;
                    }
                }
            })
            .expect("could not create directory worker");
        let mut browser = Self {
            status: prefs.warning.clone().unwrap_or_default(),
            prefs,
            current: core::home(),
            items: vec![],
            selected: BTreeSet::new(),
            history: vec![],
            cursor: 0,
            sender,
            receiver,
            generation: 0,
            pending: false,
            pending_request: None,
            last_poll: Instant::now(),
            restore_session,
        };
        browser.request(
            initial,
            if restore_session {
                Reason::Restore
            } else {
                Reason::Startup
            },
        );
        browser
    }
    fn request(&mut self, path: PathBuf, reason: Reason) {
        self.generation += 1;
        self.pending = true;
        self.pending_request = Some((path.clone(), reason.clone()));
        if self
            .sender
            .send(Request {
                id: self.generation,
                path,
                reason,
            })
            .is_err()
        {
            self.pending = false;
            self.status = "Directory worker stopped. Close and reopen this window.".into();
        }
    }
    /// Suspend polling while a platform connection resolves. Any later navigation
    /// or tab transition invalidates this token, including switching away/back.
    pub fn begin_connection(&mut self) -> u64 {
        self.generation += 1;
        self.pending = true;
        self.pending_request = None;
        self.status = "Connecting to server… Complete any macOS connection dialog.".into();
        self.generation
    }
    pub fn finish_connection(&mut self, token: u64, result: Result<PathBuf, String>) -> bool {
        if token != self.generation {
            return false;
        }
        self.pending = false;
        match result {
            Ok(path) => self.request(path, Reason::Navigate),
            Err(message) => self.status = message,
        }
        true
    }
    pub fn navigate(&mut self, text: &str) {
        match core::location(text, &self.current) {
            Ok(path) => self.request(path, Reason::Navigate),
            Err(e) => self.status = e.to_string(),
        }
    }
    pub fn back(&mut self) {
        if self.cursor > 0 {
            self.request(
                self.history[self.cursor - 1].clone(),
                Reason::History(self.cursor - 1),
            );
        }
    }
    pub fn forward(&mut self) {
        if self.cursor + 1 < self.history.len() {
            self.request(
                self.history[self.cursor + 1].clone(),
                Reason::History(self.cursor + 1),
            );
        }
    }
    pub fn up(&mut self) {
        if let Some(parent) = self.current.parent() {
            self.request(parent.to_path_buf(), Reason::Navigate);
        }
    }
    pub fn refresh(&mut self) {
        if !self.pending {
            self.request(self.current.clone(), Reason::Refresh);
        }
    }
    pub fn save(&mut self) {
        if let Some(tab) = self.prefs.data.tabs.get_mut(self.prefs.data.active_tab) {
            if !self.history.is_empty() {
                tab.path = self.current.clone();
            }
            tab.sort = self.prefs.data.sort;
            tab.descending = self.prefs.data.descending;
        }
        if let Err(e) = self.prefs.save() {
            self.status = format!("Could not save preferences: {e}");
        }
    }
    pub fn sort(&mut self, key: Sort) {
        if self.prefs.data.sort == key {
            self.prefs.data.descending = !self.prefs.data.descending;
        } else {
            self.prefs.data.sort = key;
            self.prefs.data.descending = false;
        }
        core::sort_entries(&mut self.items, key, self.prefs.data.descending);
        self.save();
    }
    pub fn toggle_favorite(&mut self) {
        if self.prefs.data.favorites.contains(&self.current) {
            self.prefs.unfavorite(&self.current);
        } else {
            self.prefs.favorite(&self.current);
        }
        self.save();
    }
    pub fn clear_visits(&mut self) {
        self.prefs.data.visits.clear();
        self.save();
    }
    pub fn copy_paths(&self, shell: bool) -> String {
        let paths = if self.selected.is_empty() {
            vec![self.current.clone()]
        } else {
            self.items
                .iter()
                .filter(|e| self.selected.contains(&e.path))
                .map(|e| e.path.clone())
                .collect()
        };
        core::clipboard_text(&paths, shell)
    }
    /// Poll worker results on the UI thread. Returns whether visible state changed.
    pub fn tick(&mut self) -> bool {
        let mut changed = false;
        while let Ok(reply) = self.receiver.try_recv() {
            if reply.request.id != self.generation {
                continue;
            }
            self.pending = false;
            self.pending_request = None;
            self.last_poll = Instant::now();
            let refresh = matches!(reply.request.reason, Reason::Refresh);
            match reply.result {
                Err(e) => {
                    self.status = format!("Cannot open {}: {e}", reply.request.path.display());
                    if matches!(reply.request.reason, Reason::Startup | Reason::Restore)
                        && reply.request.path != core::home()
                    {
                        self.request(core::home(), reply.request.reason);
                    }
                    changed = true;
                }
                Ok((path, mut items, selection)) => {
                    core::sort_entries(
                        &mut items,
                        self.prefs.data.sort,
                        self.prefs.data.descending,
                    );
                    if refresh && same_entries(&items, &self.items) {
                        continue;
                    }
                    self.items = items;
                    if !refresh {
                        self.status = self.prefs.warning.clone().unwrap_or_default();
                        self.current = path.clone();
                        self.selected.clear();
                        if let Some(selection) = selection {
                            self.selected.insert(selection);
                        }
                        match reply.request.reason {
                            Reason::History(index) => self.cursor = index,
                            _ => {
                                if self.history.get(self.cursor) != Some(&path) {
                                    self.history.truncate(if self.history.is_empty() {
                                        0
                                    } else {
                                        self.cursor + 1
                                    });
                                    self.history.push(path.clone());
                                    self.cursor = self.history.len() - 1;
                                }
                            }
                        }
                        if !matches!(reply.request.reason, Reason::Restore) {
                            self.prefs.visit(&path);
                        }
                        self.prefs.data.last_path = path;
                        self.save();
                    } else {
                        self.selected
                            .retain(|p| self.items.iter().any(|e| &e.path == p));
                    }
                    changed = true;
                }
            }
        }
        if !self.pending && self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.refresh();
        }
        changed
    }
    fn snapshot(&self) -> ViewState {
        ViewState {
            current: self.current.clone(),
            items: self.items.clone(),
            selected: self.selected.clone(),
            status: self.status.clone(),
            history: self.history.clone(),
            cursor: self.cursor,
            pending: self.pending_request.clone(),
            sort: self.prefs.data.sort,
            descending: self.prefs.data.descending,
        }
    }
    fn restore(&mut self, view: ViewState) {
        self.current = view.current;
        self.items = view.items;
        self.selected = view.selected;
        self.status = view.status;
        self.history = view.history;
        self.cursor = view.cursor;
        self.prefs.data.sort = view.sort;
        self.prefs.data.descending = view.descending;
        core::sort_entries(
            &mut self.items,
            self.prefs.data.sort,
            self.prefs.data.descending,
        );
        if !self.history.is_empty() {
            self.prefs.data.last_path = self.current.clone();
        }
        let (path, reason) = view
            .pending
            .unwrap_or_else(|| (self.current.clone(), Reason::Refresh));
        // A new generation invalidates any result still in flight for the old tab.
        self.request(path, reason);
    }
    pub fn is_loading(&self) -> bool {
        self.pending
    }
}
#[derive(Clone)]
struct ViewState {
    current: PathBuf,
    items: Vec<Entry>,
    selected: BTreeSet<PathBuf>,
    status: String,
    history: Vec<PathBuf>,
    cursor: usize,
    pending: Option<(PathBuf, Reason)>,
    sort: Sort,
    descending: bool,
}
struct Tab {
    view: ViewState,
    draft: String,
}

/// All tabs share preferences and exactly one directory worker; only the active
/// tab scans. Pending navigation is resumed when its saved tab is reactivated.
pub struct TabbedBrowser {
    pub browser: Browser,
    tabs: Vec<Tab>,
    active: usize,
}
impl TabbedBrowser {
    pub fn new(browser: Browser) -> Self {
        let view = browser.snapshot();
        let draft = view.current.to_string_lossy().into_owned();
        let mut result = Self {
            browser,
            tabs: vec![Tab { view, draft }],
            active: 0,
        };
        if result.browser.restore_session {
            result.active = result.browser.prefs.data.active_tab;
            result.tabs = result
                .browser
                .prefs
                .data
                .tabs
                .iter()
                .map(|saved| {
                    let view = ViewState {
                        current: saved.path.clone(),
                        items: vec![],
                        selected: BTreeSet::new(),
                        status: result.browser.status.clone(),
                        history: vec![],
                        cursor: 0,
                        pending: Some((saved.path.clone(), Reason::Restore)),
                        sort: saved.sort,
                        descending: saved.descending,
                    };
                    Tab {
                        draft: saved.path.to_string_lossy().into_owned(),
                        view,
                    }
                })
                .collect();
        }
        result.save_session();
        result
    }
    fn save_session(&mut self) {
        let active = self.browser.snapshot();
        self.browser.prefs.data.tabs = self
            .tabs
            .iter()
            .enumerate()
            .map(|(i, tab)| {
                let view = if i == self.active { &active } else { &tab.view };
                let path = if view.history.is_empty() {
                    view.pending
                        .as_ref()
                        .map_or(&view.current, |(path, _)| path)
                } else {
                    &view.current
                };
                SavedTab {
                    path: path.clone(),
                    sort: view.sort,
                    descending: view.descending,
                }
            })
            .collect();
        self.browser.prefs.data.active_tab = self.active;
        self.browser.save();
    }
    pub fn count(&self) -> usize {
        self.tabs.len()
    }
    pub fn active(&self) -> usize {
        self.active
    }
    pub fn labels(&self) -> Vec<String> {
        self.tabs
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let path = if i == self.active {
                    &self.browser.current
                } else {
                    &t.view.current
                };
                path.file_name()
                    .unwrap_or(path.as_os_str())
                    .to_string_lossy()
                    .into_owned()
            })
            .collect()
    }
    fn park(&mut self, draft: String) {
        self.tabs[self.active] = Tab {
            view: self.browser.snapshot(),
            draft,
        };
    }
    pub fn open(&mut self, draft: String) -> String {
        self.park(draft);
        let mut view = self.browser.snapshot();
        view.selected.clear();
        view.status.clear();
        if !view.history.is_empty() {
            view.history = vec![view.current.clone()];
            view.cursor = 0;
            view.pending = None;
        }
        let draft = view.current.to_string_lossy().into_owned();
        self.tabs.push(Tab {
            view: view.clone(),
            draft: draft.clone(),
        });
        self.active = self.tabs.len() - 1;
        self.browser.restore(view);
        self.save_session();
        draft
    }
    pub fn select(&mut self, index: usize, draft: String) -> Option<String> {
        if index >= self.tabs.len() || index == self.active {
            return None;
        }
        self.park(draft);
        self.active = index;
        self.browser.restore(self.tabs[index].view.clone());
        self.save_session();
        Some(self.tabs[index].draft.clone())
    }
    pub fn close(&mut self) -> Option<String> {
        if self.tabs.len() == 1 {
            return None;
        }
        self.tabs.remove(self.active);
        self.active = self.active.min(self.tabs.len() - 1);
        self.browser.restore(self.tabs[self.active].view.clone());
        self.save_session();
        Some(self.tabs[self.active].draft.clone())
    }
}
fn same_entries(a: &[Entry], b: &[Entry]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(a, b)| {
            a.path == b.path
                && a.is_dir == b.is_dir
                && a.size == b.size
                && a.modified == b.modified
                && a.symlink == b.symlink
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn settle(b: &mut Browser) {
        let start = Instant::now();
        while b.is_loading() {
            b.tick();
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(5));
        }
    }
    #[test]
    fn tabs_keep_history_selection_and_drafts_but_share_preferences() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        let b = t.path().join("b");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        let f = a.join("keep");
        std::fs::write(&f, "fixture").unwrap();
        let mut browser = Browser::new(
            Preferences::load(t.path().join("prefs.json")),
            Some(a.clone()),
        );
        settle(&mut browser);
        browser.selected.insert(f.clone());
        let visits = browser.prefs.data.visits[&a].count;
        let mut tabs = TabbedBrowser::new(browser);
        tabs.open("unfinished/path".into());
        settle(&mut tabs.browser);
        assert_eq!(tabs.count(), 2);
        assert_eq!(tabs.browser.current, a);
        assert_eq!(tabs.browser.prefs.data.visits[&a].count, visits);
        tabs.browser.navigate(b.to_str().unwrap());
        settle(&mut tabs.browser);
        tabs.browser.toggle_favorite();
        assert_eq!(
            tabs.select(0, "other draft".into()).unwrap(),
            "unfinished/path"
        );
        settle(&mut tabs.browser);
        assert_eq!(tabs.browser.current, a);
        assert!(tabs.browser.selected.contains(&f));
        assert_eq!(tabs.browser.history, vec![a.clone()]);
        assert!(tabs.browser.prefs.data.favorites.contains(&b));
        assert_eq!(tabs.select(1, "first draft".into()).unwrap(), "other draft");
        settle(&mut tabs.browser);
        assert_eq!(tabs.browser.current, b);
        assert_eq!(tabs.browser.history, vec![a.clone(), b.clone()]);
        assert_eq!(tabs.close().unwrap(), "first draft");
        settle(&mut tabs.browser);
        assert_eq!(tabs.count(), 1);
        assert_eq!(tabs.browser.current, a);
        assert_eq!(tabs.close(), None);
        assert_eq!(std::fs::read_to_string(f).unwrap(), "fixture");
    }
    #[test]
    fn inactive_tab_resumes_pending_navigation_and_old_replies_cannot_leak() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        let b = t.path().join("b");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        let mut browser = Browser::new(
            Preferences::load(t.path().join("prefs.json")),
            Some(a.clone()),
        );
        settle(&mut browser);
        let mut tabs = TabbedBrowser::new(browser);
        tabs.browser.navigate(b.to_str().unwrap());
        tabs.open(a.to_string_lossy().into_owned());
        settle(&mut tabs.browser);
        assert_eq!(tabs.browser.current, a);
        tabs.select(0, a.to_string_lossy().into_owned());
        settle(&mut tabs.browser);
        assert_eq!(tabs.browser.current, b);
        tabs.select(1, b.to_string_lossy().into_owned());
        settle(&mut tabs.browser);
        assert_eq!(tabs.browser.current, a);
        assert_eq!(tabs.browser.prefs.data.visits[&a].count, 1);
    }
    #[test]
    fn refresh_rejects_replaced_directory_without_relabeling_parent_listing() {
        let t = tempfile::tempdir().unwrap();
        let folder = t.path().join("folder");
        std::fs::create_dir(&folder).unwrap();
        std::fs::write(folder.join("original"), "fixture").unwrap();
        let mut m = Browser::new(
            Preferences::load(t.path().join("state.json")),
            Some(folder.clone()),
        );
        settle(&mut m);
        std::fs::rename(&folder, t.path().join("moved")).unwrap();
        std::fs::write(&folder, "now a file").unwrap();
        m.refresh();
        settle(&mut m);
        assert_eq!(m.current, folder);
        assert_eq!(m.items.len(), 1);
        assert_eq!(m.items[0].name(), "original");
        assert!(m.status.contains("Cannot open"));
        assert_eq!(m.prefs.data.visits[&folder].count, 1);
    }
    #[test]
    fn navigation_errors_history_and_file_selection() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        let b = t.path().join("b");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        std::fs::write(b.join(".hidden"), "fixture").unwrap();
        let mut m = Browser::new(
            Preferences::load(t.path().join("prefs.json")),
            Some(a.clone()),
        );
        settle(&mut m);
        m.navigate(b.join(".hidden").to_str().unwrap());
        settle(&mut m);
        assert_eq!(m.current, b);
        assert!(m.selected.contains(&b.join(".hidden")));
        m.navigate("missing");
        settle(&mut m);
        assert_eq!(m.current, b);
        assert!(m.status.contains("Cannot open"));
        m.back();
        settle(&mut m);
        assert_eq!(m.current, a);
        m.forward();
        settle(&mut m);
        assert_eq!(m.current, b);
        assert_eq!(m.history.len(), 2);
    }
    #[test]
    fn refresh_does_not_count_visits_or_erase_selection_and_last_request_wins() {
        let t = tempfile::tempdir().unwrap();
        let a = t.path().join("a");
        let b = t.path().join("b");
        std::fs::create_dir(&a).unwrap();
        std::fs::create_dir(&b).unwrap();
        let file = b.join("keep");
        std::fs::write(&file, "fixture").unwrap();
        let mut m = Browser::new(
            Preferences::load(t.path().join("prefs.json")),
            Some(a.clone()),
        );
        m.navigate(a.to_str().unwrap());
        m.navigate(b.to_str().unwrap());
        settle(&mut m);
        assert_eq!(m.current, b);
        assert_eq!(m.prefs.data.visits.len(), 1);
        m.selected.insert(file.clone());
        let count = m.prefs.data.visits[&b].count;
        std::fs::write(b.join("new"), "fixture").unwrap();
        m.refresh();
        settle(&mut m);
        assert_eq!(m.items.len(), 2);
        assert_eq!(m.prefs.data.visits[&b].count, count);
        assert!(m.selected.contains(&file));
        m.toggle_favorite();
        m.clear_visits();
        assert_eq!(m.prefs.data.favorites, vec![b]);
        assert_eq!(std::fs::read_to_string(file).unwrap(), "fixture");
    }
}

#[cfg(test)]
mod connection_tests {
    use super::*;
    fn settle(browser: &mut Browser) {
        for _ in 0..1000 {
            browser.tick();
            if !browser.is_loading() {
                return;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        panic!("scan timed out");
    }
    #[test]
    fn connection_success_errors_and_stale_results_preserve_documents_and_history() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("root");
        let mounted = temp.path().join("mounted share");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&mounted).unwrap();
        std::fs::write(mounted.join("fixture"), "unchanged").unwrap();
        let mut b = Browser::new(
            Preferences::load(temp.path().join("prefs.json")),
            Some(root.clone()),
        );
        settle(&mut b);
        let history = b.history.clone();
        let token = b.begin_connection();
        assert!(b.finish_connection(token, Err("Server connection cancelled.".into())));
        assert_eq!(b.current, root);
        assert_eq!(b.history, history);
        assert!(b.status.contains("cancelled"));
        let token = b.begin_connection();
        b.navigate(root.to_str().unwrap());
        assert!(!b.finish_connection(token, Ok(mounted.clone())));
        assert!(!b.finish_connection(token, Err("stale error".into())));
        assert!(!b.status.contains("stale error"));
        settle(&mut b);
        assert_eq!(b.current, root);
        let token = b.begin_connection();
        assert!(b.finish_connection(token, Ok(mounted.clone())));
        settle(&mut b);
        assert_eq!(b.current, mounted);
        assert_eq!(b.items.len(), 1);
        b.back();
        settle(&mut b);
        assert_eq!(b.current, root);
        assert_eq!(
            std::fs::read_to_string(mounted.join("fixture")).unwrap(),
            "unchanged"
        );
        let mut tabs = TabbedBrowser::new(b);
        let token = tabs.browser.begin_connection();
        tabs.open("smb://server.invalid/share".into());
        assert!(!tabs.browser.finish_connection(token, Ok(mounted.clone())));
        tabs.select(0, String::new());
        assert!(!tabs.browser.finish_connection(token, Ok(mounted)));
        settle(&mut tabs.browser);
        assert_eq!(tabs.browser.current, root);
    }
}
