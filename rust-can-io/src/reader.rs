//! Reader-side format detection.

use std::fmt;
use std::path::Path;

/// Supported log container formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    /// Vector ASC text format.
    Asc,
    /// Vector BLF binary format.
    Blf,
}

/// Error returned when a log format cannot be detected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatError {
    path: String,
}

impl fmt::Display for FormatError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unsupported log format for path {}", self.path)
    }
}

impl std::error::Error for FormatError {}

impl LogFormat {
    /// Detects the log format from a file path extension.
    pub fn from_path(path: &Path) -> Result<Self, FormatError> {
        let extension = effective_extension(path).ok_or_else(|| FormatError {
            path: path.display().to_string(),
        })?;
        match extension {
            "asc" => Ok(Self::Asc),
            "blf" => Ok(Self::Blf),
            _ => Err(FormatError {
                path: path.display().to_string(),
            }),
        }
    }

    /// Detects a log format from leading bytes.
    pub fn from_magic(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(b"LOGG") {
            Some(Self::Blf)
        } else {
            None
        }
    }
}

fn effective_extension(path: &Path) -> Option<&str> {
    let extension = path.extension()?.to_str()?;
    if matches!(extension, "gz" | "bz2" | "xz" | "zst") {
        path.file_stem()
            .and_then(|stem| Path::new(stem).extension())
            .and_then(|inner| inner.to_str())
    } else {
        Some(extension)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn detects_format_from_plain_and_compressed_extension() {
        assert_eq!(LogFormat::from_path(Path::new("capture.asc")).unwrap(), LogFormat::Asc);
        assert_eq!(LogFormat::from_path(Path::new("capture.blf")).unwrap(), LogFormat::Blf);
        assert_eq!(LogFormat::from_path(Path::new("capture.asc.gz")).unwrap(), LogFormat::Asc);
    }

    #[test]
    fn detects_blf_from_magic() {
        assert_eq!(LogFormat::from_magic(b"LOGG\x90\0\0\0"), Some(LogFormat::Blf));
    }

    #[test]
    fn rejects_unknown_extension() {
        let error = LogFormat::from_path(Path::new("capture.unknown")).unwrap_err();
        assert_eq!(error.to_string(), "unsupported log format for path capture.unknown");
    }
}
