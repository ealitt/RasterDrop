use crate::conversion::errors::ConversionError;
use crate::models::{CollisionPolicy, OutputFormat, OutputLocation};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{LazyLock, Mutex};
use uuid::Uuid;

static RESERVED_PATHS: LazyLock<Mutex<HashSet<PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashSet::new()));

#[derive(Debug)]
pub struct ReservedDestination {
    pub final_path: PathBuf,
    pub temp_path: PathBuf,
}

impl Drop for ReservedDestination {
    fn drop(&mut self) {
        if let Ok(mut reservations) = RESERVED_PATHS.lock() {
            reservations.remove(&self.final_path);
        }
        let _ = std::fs::remove_file(&self.temp_path);
    }
}

pub fn resolve_output_directory(
    source: &Path,
    location: &OutputLocation,
) -> Result<PathBuf, ConversionError> {
    let directory = match location {
        OutputLocation::BesideSource => source
            .parent()
            .ok_or_else(|| ConversionError::InvalidOutputDirectory("source has no parent".into()))?
            .to_path_buf(),
        OutputLocation::FixedDirectory(directory) => directory.clone(),
    };
    if !directory.is_dir() {
        return Err(ConversionError::InvalidOutputDirectory(
            directory.display().to_string(),
        ));
    }
    Ok(directory)
}

pub fn reserve_destination(
    source: &Path,
    output_directory: &Path,
    format: OutputFormat,
    policy: CollisionPolicy,
) -> Result<ReservedDestination, ConversionError> {
    let stem = source
        .file_stem()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .ok_or(ConversionError::UnsupportedFormat)?;
    let source_normalized = source
        .canonicalize()
        .unwrap_or_else(|_| source.to_path_buf());
    let mut reservations = RESERVED_PATHS
        .lock()
        .map_err(|_| ConversionError::Io(std::io::Error::other("destination lock poisoned")))?;

    let mut index = 0_u32;
    let final_path = loop {
        let suffix = if index == 0 {
            String::new()
        } else {
            format!(" ({index})")
        };
        let candidate = output_directory.join(format!("{stem}{suffix}.{}", format.extension()));
        let candidate_normalized = candidate
            .canonicalize()
            .unwrap_or_else(|_| candidate.clone());
        if candidate_normalized == source_normalized {
            if policy == CollisionPolicy::Overwrite {
                return Err(ConversionError::WouldOverwriteSource);
            }
            index += 1;
            continue;
        }
        let available = match policy {
            CollisionPolicy::Overwrite => !reservations.contains(&candidate),
            CollisionPolicy::CreateNumberedFile => {
                !candidate.exists() && !reservations.contains(&candidate)
            }
        };
        if available {
            reservations.insert(candidate.clone());
            break candidate;
        }
        index = index.checked_add(1).ok_or_else(|| {
            ConversionError::Io(std::io::Error::other("too many filename collisions"))
        })?;
    };

    let temp_path = output_directory.join(format!(".rasterdrop-{}.tmp", Uuid::new_v4().simple()));
    Ok(ReservedDestination {
        final_path,
        temp_path,
    })
}
