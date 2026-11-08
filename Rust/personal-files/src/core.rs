//! Filesystem behavior and durable local preferences, independent of GTK.
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{self, Write},
    os::unix::fs::OpenOptionsExt,
    path::{Component, Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn home() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/"))
}

pub fn location(text: &str, current: &Path) -> io::Result<PathBuf> {
    if text.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "Enter a file or folder path.",
        ));
    }
    let input = if text == "~" {
        home()
    } else if let Some(tail) = text.strip_prefix("~/") {
        home().join(tail)
    } else {
        PathBuf::from(text)
    };
    let absolute = if input.is_absolute() {
        input
    } else {
        current.join(input)
    };
    let mut normalized = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::ParentDir => {
                normalized.pop();
            }
            Component::CurDir => {}
            component => normalized.push(component.as_os_str()),
        }
    }
    // Deliberately avoid canonicalize: preserve familiar workspace symlink paths.
    Ok(normalized)
}

#[derive(Clone, Debug)]
pub struct Entry {
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: Option<u64>,
    pub modified: u64,
    pub symlink: bool,
}
impl Entry {
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    }
}

pub fn entries(path: &Path) -> io::Result<Vec<Entry>> {
    fs::read_dir(path)?
        .map(|item| {
            let item = item?;
            let path = item.path();
            let metadata = fs::metadata(&path);
            let is_dir = metadata.as_ref().is_ok_and(|m| m.is_dir());
            let size = metadata.as_ref().ok().map(|m| m.len());
            let modified = metadata
                .as_ref()
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map_or(0, |t| t.as_secs());
            Ok(Entry {
                path,
                is_dir,
                size,
                modified,
                symlink: item.file_type().is_ok_and(|t| t.is_symlink()),
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Sort {
    #[default]
    Name,
    Modified,
    Size,
}

pub fn sort_entries(items: &mut [Entry], key: Sort, descending: bool) {
    items.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| {
            let comparison = match key {
                Sort::Name => a.name().to_lowercase().cmp(&b.name().to_lowercase()),
                Sort::Modified => a.modified.cmp(&b.modified),
                Sort::Size => a.size.cmp(&b.size),
            }
            .then_with(|| a.name().to_lowercase().cmp(&b.name().to_lowercase()))
            .then_with(|| a.path.cmp(&b.path));
            if descending {
                comparison.reverse()
            } else {
                comparison
            }
        })
    });
}

pub fn size_label(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "—".into();
    };
    if bytes < 1024 {
        return format!("{bytes} B");
    }
    let mut value = bytes as f64;
    for unit in ["KB", "MB", "GB", "TB", "PB", "EB"] {
        value /= 1024.;
        if value < 1024. || unit == "EB" {
            return format!("{value:.1} {unit}");
        }
    }
    unreachable!()
}

pub fn shell_quote(value: &str) -> String {
    if !value.is_empty()
        && value
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"/_-.,:@%+=".contains(&b))
    {
        return value.into();
    }
    format!("'{}'", value.replace('\'', "'\\''"))
}

pub fn clipboard_text(paths: &[PathBuf], shell: bool) -> String {
    paths
        .iter()
        .map(|p| {
            let text = p.to_string_lossy();
            if shell {
                shell_quote(&text)
            } else {
                text.into_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(if shell { " " } else { "\n" })
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Visit {
    pub count: u64,
    pub last: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SavedTab {
    pub path: PathBuf,
    #[serde(default)]
    pub sort: Sort,
    #[serde(default)]
    pub descending: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Saved {
    pub favorites: Vec<PathBuf>,
    pub visits: BTreeMap<PathBuf, Visit>,
    pub last_path: PathBuf,
    pub theme: String,
    pub image: PathBuf,
    pub sort: Sort,
    pub descending: bool,
    pub tabs: Vec<SavedTab>,
    pub active_tab: usize,
}
impl Default for Saved {
    fn default() -> Self {
        Self {
            favorites: vec![],
            visits: BTreeMap::new(),
            last_path: home(),
            theme: "silver".into(),
            image: PathBuf::new(),
            sort: Sort::Name,
            descending: false,
            tabs: vec![],
            active_tab: 0,
        }
    }
}

pub struct Preferences {
    pub path: PathBuf,
    pub data: Saved,
    pub warning: Option<String>,
    corrupt: bool,
}
impl Preferences {
    pub fn default_path() -> PathBuf {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home().join(".config"))
            .join("personal-files/preferences.json")
    }
    pub fn load(path: PathBuf) -> Self {
        let mut prefs = Self {
            path,
            data: Saved::default(),
            warning: None,
            corrupt: false,
        };
        match fs::read(&prefs.path) {
            Ok(bytes) => {
                match serde_json::from_slice::<serde_json::Map<String, serde_json::Value>>(&bytes)
                    .and_then(|object| {
                        serde_json::from_value::<Saved>(serde_json::Value::Object(object))
                    }) {
                    Ok(mut data)
                        if data.favorites.iter().all(|p| p.is_absolute())
                            && data.visits.iter().all(|(p, v)| {
                                p.is_absolute() && v.count > 0 && v.last.is_finite()
                            }) =>
                    {
                        let mut unique = vec![];
                        for p in data.favorites {
                            if !unique.contains(&p) {
                                unique.push(p)
                            }
                        }
                        data.favorites = unique;
                        data.tabs.retain(|tab| tab.path.is_absolute());
                        data.active_tab = data.active_tab.min(data.tabs.len().saturating_sub(1));
                        if !["silver", "midnight", "aurora", "auto"].contains(&data.theme.as_str())
                        {
                            data.theme = "silver".into()
                        }
                        prefs.data = data;
                        prefs.trim_visits();
                    }
                    _ => {
                        prefs.warning = Some("Saved preferences were invalid. Defaults are shown; the original will be backed up on save.".into());
                        prefs.corrupt = true;
                    }
                }
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => {
                prefs.warning = Some(format!("Could not load preferences: {e}"));
            }
        }
        prefs
    }
    pub fn save(&mut self) -> io::Result<()> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| io::Error::other("Preferences need a parent folder"))?;
        fs::create_dir_all(dir)?;
        let bytes = serde_json::to_vec_pretty(&self.data).map_err(io::Error::other)?;
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let temporary = dir.join(format!(".preferences-{}-{nonce}", std::process::id()));
        let result = (|| {
            let mut stream = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&temporary)?;
            stream.write_all(&bytes)?;
            stream.sync_all()?;
            if self.corrupt && self.path.exists() {
                fs::rename(
                    &self.path,
                    self.path.with_extension(format!("json.corrupt-{nonce}")),
                )?;
                self.corrupt = false;
            }
            fs::rename(&temporary, &self.path)
        })();
        if temporary.exists() {
            let _ = fs::remove_file(temporary);
        }
        result
    }
    pub fn visit(&mut self, path: &Path) {
        let visit = self
            .data
            .visits
            .entry(path.into())
            .or_insert(Visit { count: 0, last: 0. });
        visit.count = visit.count.saturating_add(1);
        visit.last = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        self.data.last_path = path.into();
        self.trim_visits();
    }
    fn trim_visits(&mut self) {
        let keep = self.recent(200);
        self.data.visits.retain(|path, _| keep.contains(path));
    }
    pub fn favorite(&mut self, path: &Path) {
        if !self.data.favorites.iter().any(|p| p == path) {
            self.data.favorites.push(path.into());
        }
    }
    pub fn unfavorite(&mut self, path: &Path) {
        self.data.favorites.retain(|p| p != path);
    }
    pub fn recent(&self, limit: usize) -> Vec<PathBuf> {
        let mut visits: Vec<_> = self.data.visits.iter().collect();
        visits.sort_by(|a, b| b.1.last.total_cmp(&a.1.last).then_with(|| a.0.cmp(b.0)));
        visits
            .into_iter()
            .take(limit)
            .map(|(p, _)| p.clone())
            .collect()
    }
    pub fn frequent(&self, limit: usize) -> Vec<PathBuf> {
        let mut visits: Vec<_> = self.data.visits.iter().collect();
        visits.sort_by(|a, b| {
            b.1.count
                .cmp(&a.1.count)
                .then_with(|| b.1.last.total_cmp(&a.1.last))
                .then_with(|| a.0.cmp(b.0))
        });
        visits
            .into_iter()
            .take(limit)
            .map(|(p, _)| p.clone())
            .collect()
    }
}
