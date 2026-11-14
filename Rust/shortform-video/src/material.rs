use crate::config::MaterialsConfig;
use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Aspect {
    Portrait,
    Landscape,
    Square,
}

impl Aspect {
    pub fn resolution(self) -> (u32, u32) {
        match self {
            Aspect::Portrait => (1080, 1920),
            Aspect::Landscape => (1920, 1080),
            Aspect::Square => (1080, 1080),
        }
    }

    /// Whether a source of the given dimensions matches this orientation.
    /// Square accepts anything: 1:1 stock is rare, so the compositor crops.
    pub fn matches(self, width: u32, height: u32) -> bool {
        if width == 0 || height == 0 {
            return false;
        }
        match self {
            Aspect::Portrait => height > width,
            Aspect::Landscape => width > height,
            Aspect::Square => true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialItem {
    pub provider: String,
    pub url: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub search_term: String,
    /// Public page for attribution (Pexels/Pixabay license asks for credit
    /// where possible).
    pub source_page: Option<String>,
}

fn http_client() -> Result<reqwest::blocking::Client> {
    Ok(reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(90))
        .user_agent("Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) shortform-video/0.1")
        .build()?)
}

/// Search Pexels for videos matching the term, orientation, and a minimum
/// duration. Picks the smallest rendition that still covers the target
/// resolution to keep downloads reasonable.
pub fn search_pexels(
    config: &MaterialsConfig,
    term: &str,
    minimum_duration: f64,
    aspect: Aspect,
) -> Result<Vec<MaterialItem>> {
    if config.pexels_api_key.trim().is_empty() {
        return Err(Error::Material(
            "Pexels API key missing; set SHORTFORM_PEXELS_API_KEY or materials.pexels_api_key"
                .to_string(),
        ));
    }
    let orientation = match aspect {
        Aspect::Portrait => "portrait",
        Aspect::Landscape => "landscape",
        Aspect::Square => "square",
    };
    let response: Value = http_client()?
        .get("https://api.pexels.com/videos/search")
        .header("Authorization", config.pexels_api_key.trim())
        .query(&[
            ("query", term),
            ("per_page", "20"),
            ("orientation", orientation),
        ])
        .send()?
        .error_for_status()
        .map_err(|e| Error::Material(format!("pexels search failed: {e}")))?
        .json()?;

    let (target_width, _) = aspect.resolution();
    let mut items = Vec::new();
    for video in response["videos"].as_array().unwrap_or(&Vec::new()) {
        let duration = video["duration"].as_f64().unwrap_or(0.0);
        if duration < minimum_duration {
            continue;
        }
        let files = video["video_files"].as_array().cloned().unwrap_or_default();
        let mut candidates: Vec<(u32, u32, String)> = files
            .iter()
            .filter_map(|file| {
                let width = file["width"].as_u64()? as u32;
                let height = file["height"].as_u64()? as u32;
                let link = file["link"].as_str()?.to_string();
                (aspect.matches(width, height) || aspect == Aspect::Square)
                    .then_some((width, height, link))
            })
            .collect();
        candidates.sort_by_key(|(width, _, _)| *width);
        let chosen = candidates
            .iter()
            .find(|(width, _, _)| *width >= target_width)
            .or(candidates.last());
        if let Some((width, height, link)) = chosen {
            items.push(MaterialItem {
                provider: "pexels".to_string(),
                url: link.clone(),
                duration,
                width: *width,
                height: *height,
                search_term: term.to_string(),
                source_page: video["url"].as_str().map(str::to_string),
            });
        }
    }
    Ok(items)
}

/// Search Pixabay. Same contract as `search_pexels`.
pub fn search_pixabay(
    config: &MaterialsConfig,
    term: &str,
    minimum_duration: f64,
    aspect: Aspect,
) -> Result<Vec<MaterialItem>> {
    if config.pixabay_api_key.trim().is_empty() {
        return Err(Error::Material(
            "Pixabay API key missing; set SHORTFORM_PIXABAY_API_KEY or materials.pixabay_api_key"
                .to_string(),
        ));
    }
    let response: Value = http_client()?
        .get("https://pixabay.com/api/videos/")
        .query(&[
            ("q", term),
            ("video_type", "all"),
            ("per_page", "50"),
            ("key", config.pixabay_api_key.trim()),
        ])
        .send()?
        .error_for_status()
        .map_err(|e| Error::Material(format!("pixabay search failed: {e}")))?
        .json()?;

    let (target_width, _) = aspect.resolution();
    let mut items = Vec::new();
    for hit in response["hits"].as_array().unwrap_or(&Vec::new()) {
        let duration = hit["duration"].as_f64().unwrap_or(0.0);
        if duration < minimum_duration {
            continue;
        }
        let Some(renditions) = hit["videos"].as_object() else {
            continue;
        };
        let mut candidates: Vec<(u32, u32, String)> = renditions
            .values()
            .filter_map(|rendition| {
                let width = rendition["width"].as_u64()? as u32;
                let height = rendition["height"].as_u64()? as u32;
                let url = rendition["url"].as_str()?.to_string();
                (aspect.matches(width, height) || aspect == Aspect::Square)
                    .then_some((width, height, url))
            })
            .collect();
        candidates.sort_by_key(|(width, _, _)| *width);
        let chosen = candidates
            .iter()
            .find(|(width, _, _)| *width >= target_width)
            .or(candidates.last());
        if let Some((width, height, url)) = chosen {
            items.push(MaterialItem {
                provider: "pixabay".to_string(),
                url: url.clone(),
                duration,
                width: *width,
                height: *height,
                search_term: term.to_string(),
                source_page: hit["pageURL"].as_str().map(str::to_string),
            });
        }
    }
    Ok(items)
}

/// Deduplicate by URL and keep only as many items as needed to cover the
/// narration: each item contributes at most `max_clip_duration` seconds to the
/// final cut, so budget with min(duration, max_clip_duration) and stop once
/// the audio duration is covered. Ported from MoneyPrinterTurbo's
/// download_videos budgeting.
pub fn plan_downloads(
    items: Vec<MaterialItem>,
    audio_duration: f64,
    max_clip_duration: f64,
    shuffle: bool,
) -> Vec<MaterialItem> {
    let mut seen = std::collections::HashSet::new();
    let mut unique: Vec<MaterialItem> = items
        .into_iter()
        .filter(|item| seen.insert(item.url.clone()))
        .collect();

    if shuffle {
        use rand::seq::SliceRandom;
        unique.shuffle(&mut rand::rng());
    }

    let mut planned = Vec::new();
    let mut covered = 0.0;
    for item in unique {
        if covered > audio_duration {
            break;
        }
        covered += item.duration.min(max_clip_duration);
        planned.push(item);
    }
    planned
}

/// Download one material into `dir`, named by its position and provider.
pub fn download(item: &MaterialItem, dir: &Path, index: usize) -> Result<PathBuf> {
    std::fs::create_dir_all(dir)?;
    let path = dir.join(format!("material-{:02}-{}.mp4", index, item.provider));
    let mut response = http_client()?
        .get(&item.url)
        .send()?
        .error_for_status()
        .map_err(|e| Error::Material(format!("download failed for {}: {e}", item.url)))?;
    let mut file = std::fs::File::create(&path)?;
    response.copy_to(&mut file).map_err(|e| {
        Error::Material(format!("download interrupted for {}: {e}", item.url))
    })?;
    file.flush()?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(url: &str, duration: f64) -> MaterialItem {
        MaterialItem {
            provider: "test".to_string(),
            url: url.to_string(),
            duration,
            width: 1080,
            height: 1920,
            search_term: "term".to_string(),
            source_page: None,
        }
    }

    #[test]
    fn aspect_orientation_matching() {
        assert!(Aspect::Portrait.matches(1080, 1920));
        assert!(!Aspect::Portrait.matches(1920, 1080));
        assert!(Aspect::Landscape.matches(1920, 1080));
        assert!(Aspect::Square.matches(1234, 999));
        assert!(!Aspect::Portrait.matches(0, 1920));
    }

    #[test]
    fn plan_dedups_and_budgets() {
        let items = vec![
            item("a", 30.0),
            item("a", 30.0), // duplicate URL dropped
            item("b", 4.0),
            item("c", 30.0),
            item("d", 30.0),
        ];
        // Each item contributes min(duration, 5s). Audio is 12s, so we need
        // items until covered > 12: a=5, b=4 (9), c=5 (14) -> stop before d.
        let planned = plan_downloads(items, 12.0, 5.0, false);
        let urls: Vec<&str> = planned.iter().map(|i| i.url.as_str()).collect();
        assert_eq!(urls, vec!["a", "b", "c"]);
    }

    #[test]
    fn plan_handles_empty_input() {
        assert!(plan_downloads(Vec::new(), 10.0, 5.0, true).is_empty());
    }
}
