use std::path::PathBuf;
use thiserror::Error;

/// Error type for pdfp operations. Marked allow(dead_code) because many variants
/// are only used through the PdfpResult<T> type alias, not directly referenced.
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum PdfpError {
    #[error("Failed to open PDF '{path}': {message}")]
    PdfOpen { path: PathBuf, message: String },

    #[error("Failed to extract page {page}: {message}")]
    PdfExtraction { page: usize, message: String },

    #[error("Page {0} has no extractable text (possibly a scanned image PDF)")]
    EmptyPage(usize),

    #[error("Layout analysis failed on page {page}: {message}")]
    LayoutAnalysis { page: usize, message: String },

    #[error("IO error writing to '{path}': {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Invalid input '{0}': {1}")]
    InvalidInput(String, String),

    #[error("PDF is password-protected, cannot process: {0}")]
    PasswordProtected(PathBuf),

    #[error("Hybrid backend ({url}) failed: {message}")]
    HybridBackend { url: String, message: String },
}

/// Convenience result type for pdfp operations.
pub type PdfpResult<T> = Result<T, PdfpError>;
