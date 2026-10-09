use crate::models::{CollisionPolicy, OutputFormat, OutputLocation};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use tracing::warn;

pub const SETTINGS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub schema_version: u32,
    pub output_format: OutputFormat,
    pub output_location: OutputLocation,
    pub collision_policy: CollisionPolicy,
    pub jpeg_quality: u8,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            schema_version: SETTINGS_SCHEMA_VERSION,
            output_format: OutputFormat::Jpeg,
            output_location: OutputLocation::BesideSource,
            collision_policy: CollisionPolicy::CreateNumberedFile,
            jpeg_quality: 90,
        }
    }
}

impl AppSettings {
    pub fn sanitize(&mut self) {
        self.schema_version = SETTINGS_SCHEMA_VERSION;
        self.jpeg_quality = self.jpeg_quality.clamp(1, 100);
    }
}

pub struct SettingsStore;

impl SettingsStore {
    pub fn path() -> Option<PathBuf> {
        ProjectDirs::from("com", "REPLACE_ME", "RasterDrop")
            .map(|dirs| dirs.config_dir().join("settings.json"))
    }

    pub fn load() -> AppSettings {
        Self::path()
            .map(|path| Self::load_from(&path))
            .unwrap_or_default()
    }

    pub fn load_from(path: &Path) -> AppSettings {
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return AppSettings::default(),
            Err(error) => {
                warn!(path = %path.display(), %error, "could not read settings; using defaults");
                return AppSettings::default();
            }
        };
        match serde_json::from_slice::<AppSettings>(&bytes) {
            Ok(mut settings) => {
                settings.sanitize();
                settings
            }
            Err(error) => {
                warn!(path = %path.display(), %error, "could not parse settings; using defaults");
                AppSettings::default()
            }
        }
    }

    pub fn save(settings: &AppSettings) -> io::Result<()> {
        let path = Self::path().ok_or_else(|| io::Error::other("config directory unavailable"))?;
        Self::save_to(&path, settings)
    }

    pub fn save_to(path: &Path, settings: &AppSettings) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temporary = path.with_extension("json.tmp");
        let bytes = serde_json::to_vec_pretty(settings).map_err(io::Error::other)?;
        let mut file = File::create(&temporary)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        if cfg!(windows) && path.exists() {
            fs::remove_file(path)?;
        }
        fs::rename(&temporary, path).inspect_err(|_| {
            let _ = fs::remove_file(&temporary);
        })
    }
}
