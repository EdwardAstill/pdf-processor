//! Formula OCR sidecar contract and subprocess implementation.

use std::path::Path;
use std::process::Command;

/// Converts a formula crop image to a LaTeX string.
pub trait FormulaSidecar: Send + Sync {
    /// Returns recognized LaTeX, or `None` when recognition fails.
    fn recognize(&self, crop_path: &Path) -> Option<String>;
}

/// Formula OCR sidecar backed by an external command.
///
/// The command receives the crop PNG path as its first positional argument and
/// should print LaTeX to stdout with exit status 0.
pub struct SubprocessSidecar {
    command: String,
}

impl SubprocessSidecar {
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

impl FormulaSidecar for SubprocessSidecar {
    fn recognize(&self, crop_path: &Path) -> Option<String> {
        let output = Command::new(&self.command).arg(crop_path).output().ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8(output.stdout).ok()?;
        let latex = normalise_latex(&stdout);
        if latex.is_empty() {
            None
        } else {
            Some(latex.to_string())
        }
    }
}

pub fn normalise_latex(raw: &str) -> &str {
    raw.trim()
}
