use crate::core::Entry;

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
