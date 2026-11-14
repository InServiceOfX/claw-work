use crate::caption::{CaptionStyle, render_overlays};
use crate::config::Config;
use crate::error::{Error, Result};
use crate::llm::LlmClient;
use crate::material::{self, Aspect, MaterialItem};
use crate::subtitle;
use crate::tts;
use crate::video::{
    self, CaptionPosition, ConcatMode, DURATION_SAFETY_MARGIN, FitMode, SegmentPlan, SourceClip,
    Transition,
};
use std::path::{Path, PathBuf};

/// Everything a full run needs beyond the config file.
pub struct RunOptions {
    pub subject: String,
    pub language: String,
    pub paragraphs: u32,
    pub aspect: Aspect,
    pub fit: FitMode,
    pub concat_mode: ConcatMode,
    pub transition: Transition,
    pub max_clip_duration: f64,
    pub terms_amount: usize,
    pub source: MaterialSource,
    pub script_file: Option<PathBuf>,
    pub bgm_file: Option<PathBuf>,
    pub subtitles_enabled: bool,
}

#[derive(Debug, Clone)]
pub enum MaterialSource {
    Pexels,
    Pixabay,
    /// Use video files already present in a local directory; skips the LLM
    /// search-terms stage and all network access.
    LocalDir(PathBuf),
}

pub struct Pipeline {
    pub config: Config,
    pub task_dir: PathBuf,
}

impl Pipeline {
    pub fn new(config: Config, task_dir: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&task_dir)?;
        Ok(Self { config, task_dir })
    }

    fn artifact(&self, name: &str) -> PathBuf {
        self.task_dir.join(name)
    }

    pub fn run(&self, options: &RunOptions) -> Result<PathBuf> {
        // 1. Script.
        let script = match &options.script_file {
            Some(path) => std::fs::read_to_string(path)?.trim().to_string(),
            None => {
                eprintln!("## generating script");
                let llm = LlmClient::new(self.config.llm.clone())?;
                llm.generate_script(&options.subject, &options.language, options.paragraphs)?
            }
        };
        if script.is_empty() {
            return Err(Error::Llm("empty script".to_string()));
        }
        std::fs::write(self.artifact("script.txt"), &script)?;
        eprintln!("script:\n{script}\n");

        // 2. Narration.
        eprintln!(
            "## synthesizing narration ({} backend)",
            self.config.tts.backend
        );
        let audio_path = self.artifact("narration.mp3");
        let tts_output = tts::synthesize(&self.config, &script, &audio_path)?;
        eprintln!("narration: {:.2}s", tts_output.duration);

        // 3. Subtitles.
        let cues = if options.subtitles_enabled {
            let lines = subtitle::split_script_lines(&script);
            let cues = subtitle::cues_from_word_boundaries(&tts_output.word_boundaries, &lines)
                .unwrap_or_else(|| {
                    if !tts_output.word_boundaries.is_empty() {
                        eprintln!(
                            "word-boundary aggregation did not line up with the script; \
                             falling back to proportional subtitle timing"
                        );
                    }
                    subtitle::cues_proportional(&lines, tts_output.duration)
                });
            subtitle::write_srt(&cues, &self.artifact("subtitle.srt"))?;
            cues
        } else {
            Vec::new()
        };

        // 4. Materials.
        let materials_dir = self.artifact("materials");
        let sources = self.gather_materials(options, &script, tts_output.duration, &materials_dir)?;
        if sources.is_empty() {
            return Err(Error::Material(
                "no usable video materials were found".to_string(),
            ));
        }

        // 5. Normalize + concat.
        let combined = self.combine(options, &sources, tts_output.duration)?;

        // 6. Captions + final render.
        let final_path = self.artifact("final.mp4");
        let overlays = if cues.is_empty() {
            Vec::new()
        } else {
            let font = self.config.resolve_font_path()?;
            let (canvas_width, _) = options.aspect.resolution();
            let style = CaptionStyle::for_canvas(canvas_width, self.config.render.font_size);
            render_overlays(&cues, &font, &style, &self.artifact("captions"))?
        };
        eprintln!("## rendering final video");
        let loudness = video::render_final(
            &self.config.render,
            &video::RenderJob {
                combined_video: &combined,
                voice_audio: &tts_output.audio_file,
                bgm_audio: options.bgm_file.as_deref(),
                overlays: &overlays,
                output: &final_path,
                duration: tts_output.duration,
                aspect: options.aspect,
                position: CaptionPosition::Bottom,
            },
        )?;
        std::fs::write(self.artifact("loudness.json"), serde_json::to_string_pretty(&loudness)?)?;
        eprintln!(
            "loudness: {:.1} LUFS integrated, {:.1} dBTP true peak",
            loudness.integrated_lufs, loudness.true_peak_dbtp
        );
        eprintln!("final video: {}", final_path.display());
        Ok(final_path)
    }

    fn gather_materials(
        &self,
        options: &RunOptions,
        script: &str,
        audio_duration: f64,
        materials_dir: &Path,
    ) -> Result<Vec<SourceClip>> {
        let paths: Vec<PathBuf> = match &options.source {
            MaterialSource::LocalDir(dir) => {
                let mut paths: Vec<PathBuf> = std::fs::read_dir(dir)?
                    .filter_map(|entry| entry.ok())
                    .map(|entry| entry.path())
                    .filter(|path| video::is_video(path) || video::is_still(path))
                    .collect();
                paths.sort();
                paths
            }
            MaterialSource::Pexels | MaterialSource::Pixabay => {
                eprintln!("## generating search terms");
                let llm = LlmClient::new(self.config.llm.clone())?;
                let ordered = options.concat_mode == ConcatMode::Sequential;
                let terms =
                    llm.generate_terms(&options.subject, script, options.terms_amount, ordered)?;
                eprintln!("search terms: {terms:?}");
                std::fs::write(
                    self.artifact("terms.json"),
                    serde_json::to_string_pretty(&terms)?,
                )?;

                eprintln!("## searching and downloading materials");
                let mut found: Vec<MaterialItem> = Vec::new();
                for term in &terms {
                    let result = match options.source {
                        MaterialSource::Pexels => material::search_pexels(
                            &self.config.materials,
                            term,
                            options.max_clip_duration,
                            options.aspect,
                        ),
                        _ => material::search_pixabay(
                            &self.config.materials,
                            term,
                            options.max_clip_duration,
                            options.aspect,
                        ),
                    };
                    match result {
                        Ok(items) => {
                            eprintln!("  '{term}': {} candidates", items.len());
                            found.extend(items);
                        }
                        Err(e) => eprintln!("  '{term}': search failed: {e}"),
                    }
                }
                let shuffle = options.concat_mode == ConcatMode::Random;
                let planned = material::plan_downloads(
                    found,
                    audio_duration,
                    options.max_clip_duration,
                    shuffle,
                );
                let mut paths = Vec::new();
                for (index, item) in planned.iter().enumerate() {
                    match material::download(item, materials_dir, index) {
                        Ok(path) => paths.push(path),
                        Err(e) => eprintln!("  download failed ({}): {e}", item.url),
                    }
                }
                std::fs::write(
                    self.artifact("materials.json"),
                    serde_json::to_string_pretty(&planned)?,
                )?;
                paths
            }
        };

        let mut clips = Vec::new();
        for path in paths {
            // A still has no duration of its own: it holds for one clip's worth
            // of Ken Burns motion.
            if video::is_still(&path) {
                clips.push(SourceClip { path, duration: options.max_clip_duration });
                continue;
            }
            match video::probe_duration(&self.config.render.ffprobe_path, &path) {
                Ok(duration) if duration > 0.0 => clips.push(SourceClip { path, duration }),
                Ok(_) => eprintln!("skipping zero-length material {}", path.display()),
                Err(e) => eprintln!("skipping unreadable material {}: {e}", path.display()),
            }
        }
        Ok(clips)
    }

    fn combine(
        &self,
        options: &RunOptions,
        sources: &[SourceClip],
        audio_duration: f64,
    ) -> Result<PathBuf> {
        let required = audio_duration + DURATION_SAFETY_MARGIN;
        let plan = video::plan_segments(
            sources,
            options.max_clip_duration,
            required,
            options.concat_mode,
        );
        if plan.is_empty() {
            return Err(Error::Ffmpeg("segment planning produced no clips".to_string()));
        }
        eprintln!("## normalizing {} segments", plan.len());
        let segments_dir = self.artifact("segments");
        std::fs::create_dir_all(&segments_dir)?;
        let mut segment_files = Vec::new();
        for (index, segment) in plan.iter().enumerate() {
            let out = segments_dir.join(format!("segment-{index:03}.mp4"));
            self.normalize_cached(options, segment, index, &out)?;
            segment_files.push(out);
        }
        let combined = self.artifact("combined.mp4");
        video::concat_segments(&self.config.render, &segment_files, &combined)?;
        Ok(combined)
    }

    /// Looped plans reuse the same (source, start) cut; encode each cut once.
    fn normalize_cached(
        &self,
        options: &RunOptions,
        segment: &SegmentPlan,
        index: usize,
        out: &Path,
    ) -> Result<()> {
        // A still's move alternates by position, so the move is part of its key.
        let variant = if video::is_still(&segment.source) { index % 4 } else { 0 };
        let cache_key = format!(
            "{}-{:.3}-{:.3}-{variant}",
            segment.source.display(),
            segment.start,
            segment.duration
        );
        let cache_name = format!(
            "cut-{:016x}.mp4",
            fnv1a(cache_key.as_bytes())
        );
        let cache_path = out.parent().unwrap_or(Path::new(".")).join(cache_name);
        if !cache_path.is_file() {
            video::normalize_segment(
                &self.config.render,
                segment,
                options.aspect,
                options.fit,
                options.transition,
                variant,
                &cache_path,
            )?;
        }
        std::fs::copy(&cache_path, out)?;
        Ok(())
    }
}

fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Default task directory: ./shortform-tasks/<UTC timestamp>.
pub fn default_task_dir() -> PathBuf {
    let stamp = chrono::Utc::now().format("%Y%m%d-%H%M%S");
    PathBuf::from("shortform-tasks").join(stamp.to_string())
}
