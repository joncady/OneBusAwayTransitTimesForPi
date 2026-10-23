//! Bounded local media experiment. Configuration and personal assets stay private.
use serde::Deserialize;
use std::{
    collections::BTreeMap,
    fs,
    path::PathBuf,
    process::{Child, Command, Stdio},
    time::{Duration, Instant},
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Config {
    #[serde(default)]
    trigger_card: String,
    #[serde(default)]
    photo_directory: Option<PathBuf>,
    #[serde(default)]
    albums: BTreeMap<String, PathBuf>,
    sound_file: PathBuf,
    #[serde(default)]
    sound_enabled: bool,
    slide_seconds: u64,
    session_seconds: u64,
    sound_timeout_seconds: u64,
    #[serde(default)]
    enable_amplifier: bool,
}

struct Amplifier {
    changed: bool,
}

fn amplifier_state() -> std::io::Result<bool> {
    let output = Command::new("/usr/bin/NxExe")
        .args(["prox2", "portinfo"])
        .output()?;
    if output.status.success() {
        for line in String::from_utf8_lossy(&output.stdout).lines() {
            if let Some(value) = line.trim().strip_prefix("amp:") {
                match value.trim() {
                    "0" => return Ok(false),
                    "1" => return Ok(true),
                    _ => break,
                }
            }
        }
    }
    Err(std::io::Error::other("could not read amplifier state"))
}

fn toggle_amplifier() -> std::io::Result<()> {
    let status = Command::new("/usr/bin/NxExe")
        .args(["prox2", "amp"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("amplifier toggle failed"))
    }
}

impl Amplifier {
    fn enable() -> std::io::Result<Self> {
        let changed = !amplifier_state()?;
        let guard = Self { changed };
        if changed {
            toggle_amplifier()?;
        }
        if !amplifier_state()? {
            return Err(std::io::Error::other("amplifier did not enable"));
        }
        println!("Audio amplifier ready");
        Ok(guard)
    }
}

impl Drop for Amplifier {
    fn drop(&mut self) {
        if self.changed && matches!(amplifier_state(), Ok(true)) {
            if let Err(error) = toggle_amplifier() {
                eprintln!("Amplifier restore failed: {error}");
            }
        }
    }
}

pub struct Media {
    config: Config,
    albums: BTreeMap<String, Vec<PathBuf>>,
    active_album: Option<String>,
    sound: Option<Child>,
    triggered: Option<Instant>,
    slideshow: Option<Instant>,
    last_slide: Option<usize>,
    _amplifier: Option<Amplifier>,
}

impl Media {
    pub fn load(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let (config, directories) = load_config(path)?;
        let mut albums = BTreeMap::new();
        for (label, directory) in directories {
            match photo_paths(&directory) {
                Ok(paths) => {
                    albums.insert(label, paths);
                }
                Err(_) => eprintln!("Skipping an unavailable or empty photo album"),
            }
        }
        if albums.is_empty() {
            return Err("no usable photo albums found".into());
        }
        println!("Media ready: {} albums", albums.len());
        let amplifier = if config.sound_enabled && config.enable_amplifier {
            match Amplifier::enable() {
                Ok(guard) => Some(guard),
                Err(error) => {
                    eprintln!("Amplifier unavailable: {error}");
                    None
                }
            }
        } else {
            None
        };
        Ok(Self {
            config,
            albums,
            active_album: None,
            sound: None,
            triggered: None,
            slideshow: None,
            last_slide: None,
            _amplifier: amplifier,
        })
    }

    pub fn has_album(&self, label: &str) -> bool {
        self.albums.contains_key(label)
    }

    /// Repeated scanner output must not keep restarting the current slideshow.
    pub fn scan(&mut self, label: &str) -> bool {
        if self.triggered.is_some() && self.active_album.as_deref() == Some(label) {
            return false;
        }
        self.tap(label)
    }

    /// NFC and QR share the same action and two-second cross-input cooldown.
    pub fn tap(&mut self, label: &str) -> bool {
        if !self.albums.contains_key(label) {
            return false;
        }
        if self
            .triggered
            .is_some_and(|start| start.elapsed() < Duration::from_secs(2))
        {
            return false;
        }
        self.active_album = Some(label.to_string());
        self.stop_sound();
        self.triggered = Some(Instant::now());
        self.slideshow = None;
        self.last_slide = None;
        self.sound = if !self.config.sound_enabled {
            None
        } else {
            match Command::new("/usr/bin/aplay")
                .arg("-q")
                .arg(&self.config.sound_file)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .spawn()
            {
                Ok(child) => Some(child),
                Err(error) => {
                    eprintln!("Audio unavailable: {error}");
                    None
                }
            }
        };
        println!("Media action triggered");
        true
    }

    pub fn tick(&mut self) -> Option<(bool, Option<slint::Image>)> {
        let triggered = self.triggered?;
        if self.slideshow.is_none() {
            let finished = match self.sound.as_mut().map(Child::try_wait) {
                None => true,
                Some(Ok(Some(status))) => {
                    if !status.success() {
                        eprintln!("Audio playback failed");
                    }
                    true
                }
                Some(Ok(None)) => false,
                Some(Err(error)) => {
                    eprintln!("Audio status unavailable: {error}");
                    true
                }
            };
            if !finished
                && triggered.elapsed() < Duration::from_secs(self.config.sound_timeout_seconds)
            {
                return None;
            }
            self.stop_sound();
            self.slideshow = Some(Instant::now());
        }
        let elapsed = self.slideshow.unwrap().elapsed();
        if elapsed >= Duration::from_secs(self.config.session_seconds) {
            self.triggered = None;
            self.slideshow = None;
            self.last_slide = None;
            println!("Media action finished; returning to departures");
            return Some((false, None));
        }
        let paths = self.albums.get(self.active_album.as_ref()?)?;
        let index = (elapsed.as_secs() / self.config.slide_seconds) as usize % paths.len();
        if self.last_slide == Some(index) {
            return None;
        }
        match decode_photo(&paths[index]) {
            Ok(photo) => {
                self.last_slide = Some(index);
                println!("Showing photo {}", index + 1);
                Some((true, Some(photo)))
            }
            Err(_) => {
                eprintln!("Photo unavailable; returning to departures");
                self.triggered = None;
                self.slideshow = None;
                self.last_slide = None;
                Some((false, None))
            }
        }
    }

    fn stop_sound(&mut self) {
        if let Some(mut child) = self.sound.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Validate references without opening media or enabling audio/hardware.
pub fn configured_labels(path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let (_, directories) = load_config(path)?;
    Ok(directories.into_keys().collect())
}

fn load_config(
    path: &str,
) -> Result<(Config, BTreeMap<String, PathBuf>), Box<dyn std::error::Error>> {
    if fs::metadata(path)?.len() > 16 * 1024 {
        return Err("media configuration too large".into());
    }
    let config: Config = serde_json::from_slice(&fs::read(path)?)?;
    if !(1..=60).contains(&config.slide_seconds)
        || !(5..=600).contains(&config.session_seconds)
        || !(1..=10).contains(&config.sound_timeout_seconds)
    {
        return Err("invalid media action configuration".into());
    }
    let mut directories = config.albums.clone();
    if let Some(path) = &config.photo_directory {
        if directories
            .insert(config.trigger_card.clone(), path.clone())
            .is_some()
        {
            return Err("duplicate legacy/card album setting".into());
        }
    } else if !config.trigger_card.is_empty() {
        return Err("triggerCard requires photoDirectory".into());
    }
    if directories.is_empty()
        || directories.len() > 32
        || directories.keys().any(|label| {
            label.trim().is_empty()
                || label.chars().count() > 32
                || label.chars().any(char::is_control)
        })
    {
        return Err("configure 1–32 valid card album mappings".into());
    }
    Ok((config, directories))
}

fn photo_paths(directory: &std::path::Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut paths = Vec::new();
    let mut entries = 0;
    for entry in fs::read_dir(directory)? {
        entries += 1;
        if entries > 2048 {
            return Err("too many directory entries".into());
        }
        let path = entry?.path();
        // macOS USB copies may include AppleDouble ._photo.jpg metadata.
        if path
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.starts_with('.'))
        {
            continue;
        }
        if matches!(
            path.extension().and_then(|s| s.to_str()),
            Some("jpg" | "jpeg" | "png")
        ) {
            paths.push(path);
        }
        if paths.len() > 512 {
            return Err("use at most 512 prepared photos per album".into());
        }
    }
    paths.sort();
    if paths.is_empty() {
        return Err("empty album".into());
    }
    Ok(paths)
}

fn decode_photo(path: &std::path::Path) -> Result<slint::Image, Box<dyn std::error::Error>> {
    // Check encoded size and dimensions before allocating a decoded frame.
    if fs::metadata(&path)?.len() > 2 * 1024 * 1024 {
        return Err("photo exceeds 2 MiB".into());
    }
    let (width, height) = image::ImageReader::open(&path)?.into_dimensions()?;
    if width == 0 || height == 0 || width > 800 || height > 480 {
        return Err("prepare photos to fit within 800 x 480 first".into());
    }
    let mut reader = image::ImageReader::open(&path)?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(800);
    limits.max_image_height = Some(480);
    limits.max_alloc = Some(16 * 1024 * 1024);
    reader.limits(limits);
    let rgb = reader.decode()?.to_rgb8();
    let buffer =
        slint::SharedPixelBuffer::<slint::Rgb8Pixel>::clone_from_slice(rgb.as_raw(), width, height);
    Ok(slint::Image::from_rgb8(buffer))
}

impl Drop for Media {
    fn drop(&mut self) {
        self.stop_sound();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn muted_media() -> Media {
        Media {
            config: Config {
                trigger_card: String::new(),
                photo_directory: None,
                albums: BTreeMap::new(),
                sound_file: PathBuf::from("unused.wav"),
                sound_enabled: false,
                enable_amplifier: false,
                slide_seconds: 5,
                session_seconds: 60,
                sound_timeout_seconds: 3,
            },
            albums: BTreeMap::from([("Album A".into(), vec![]), ("Album B".into(), vec![])]),
            active_album: None,
            sound: None,
            triggered: None,
            slideshow: None,
            last_slide: None,
            _amplifier: None,
        }
    }

    #[test]
    fn only_accepted_album_actions_request_feedback() {
        let mut media = muted_media();
        assert!(!media.tap("Unknown"));
        assert!(media.triggered.is_none());
        assert!(media.tap("Album A"));
        assert!(media.sound.is_none());
        assert!(media._amplifier.is_none());
        assert!(!media.tap("Album B"));
        assert!(!media.scan("Album B"));
        assert_eq!(media.active_album.as_deref(), Some("Album A"));
        media.triggered = Some(Instant::now() - Duration::from_secs(3));
        assert!(!media.scan("Album A")); // Held QR: no repeated beep/restart.
        assert!(media.scan("Album B"));
        assert_eq!(media.active_album.as_deref(), Some("Album B"));
        media.triggered = Some(Instant::now() - Duration::from_secs(3));
        assert!(media.tap("Album B")); // A deliberate NFC re-tap may restart.
    }
}
