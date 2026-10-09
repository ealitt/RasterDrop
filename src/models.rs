use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;
use uuid::Uuid;

pub type JobId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    Jpeg,
    Png,
    WebP,
    Tiff,
    Bmp,
    Avif,
}

impl OutputFormat {
    pub const ALL: [Self; 6] = [
        Self::Jpeg,
        Self::Png,
        Self::WebP,
        Self::Tiff,
        Self::Bmp,
        Self::Avif,
    ];

    pub const fn extension(self) -> &'static str {
        match self {
            Self::Jpeg => "jpg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Tiff => "tiff",
            Self::Bmp => "bmp",
            Self::Avif => "avif",
        }
    }

    pub const fn selector_value(self) -> &'static str {
        match self {
            Self::Jpeg => "jpeg",
            Self::Png => "png",
            Self::WebP => "webp",
            Self::Tiff => "tiff",
            Self::Bmp => "bmp",
            Self::Avif => "avif",
        }
    }
}

impl std::str::FromStr for OutputFormat {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "jpeg" => Ok(Self::Jpeg),
            "png" => Ok(Self::Png),
            "webp" => Ok(Self::WebP),
            "tiff" => Ok(Self::Tiff),
            "bmp" => Ok(Self::Bmp),
            "avif" => Ok(Self::Avif),
            _ => Err(()),
        }
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::WebP => "WebP",
            Self::Tiff => "TIFF",
            Self::Bmp => "BMP",
            Self::Avif => "AVIF",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    Heif,
    Jpeg,
    Png,
    WebP,
    Tiff,
    Bmp,
    Avif,
    Gif,
}

impl fmt::Display for InputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Heif => "HEIC/HEIF",
            Self::Jpeg => "JPEG",
            Self::Png => "PNG",
            Self::WebP => "WebP",
            Self::Tiff => "TIFF",
            Self::Bmp => "BMP",
            Self::Avif => "AVIF",
            Self::Gif => "GIF",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", content = "directory", rename_all = "snake_case")]
pub enum OutputLocation {
    BesideSource,
    FixedDirectory(PathBuf),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollisionPolicy {
    CreateNumberedFile,
    Overwrite,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JobStatus {
    Queued,
    Inspecting,
    Converting,
    Completed { output: PathBuf },
    Skipped { reason: String },
    Failed { message: String, details: String },
    Cancelled,
}

impl JobStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed { .. } | Self::Skipped { .. } | Self::Failed { .. } | Self::Cancelled
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConversionJob {
    pub id: JobId,
    pub source: PathBuf,
    pub detected_format: Option<InputFormat>,
    pub output_format: OutputFormat,
    pub status: JobStatus,
}

impl ConversionJob {
    pub fn new(source: PathBuf, output_format: OutputFormat) -> Self {
        Self {
            id: Uuid::new_v4(),
            source,
            detected_format: None,
            output_format,
            status: JobStatus::Queued,
        }
    }
}
