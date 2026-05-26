//! Hybrid backend: delegate PDF pages that the local pipeline cannot handle
//! well (formulas, complex tables, scans) to an external service.
//!
//! Architecture (Phase 2b):
//!
//! 1. The local mupdf pipeline runs first and produces a `Document` with
//!    per-page `Block`s, images already saved to disk.
//! 2. [`apply_to_document`] iterates the pages, asks [`triage::should_route`]
//!    (or the policy override) whether each page should go to the backend,
//!    extracts the qualifying pages as single-page PDFs via
//!    [`page_extract::extract_page_as_pdf_bytes`], uploads each via
//!    [`client::DoclingClient::convert_bytes_to_markdown`], and stashes the
//!    returned markdown on `page.override_markdown`.
//! 3. The renderer honours `override_markdown`: when set, it emits that
//!    markdown verbatim for the page instead of serialising the local
//!    `blocks`. Images saved by the local pipeline remain on disk; nothing
//!    references them from routed pages, which is the current Phase 2b
//!    trade-off (documented in the plan).
//!
//! Backend failures are logged and skipped page-by-page: a single timeout
//! never kills the whole document.

pub mod client;
pub mod page_extract;
pub mod triage;

use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::document::types::Document;
use crate::error::PdfpResult;

/// Routing policy for `--hybrid docling`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingPolicy {
    /// Per-page triage decides (default).
    Auto,
    /// Route every page, regardless of triage. Useful for tests and for
    /// users who want uniform Docling-quality output across a document.
    All,
}

/// Stats returned after a hybrid run, surfaced to `--verbose`.
#[derive(Debug, Default)]
pub struct HybridStats {
    pub pages_total: usize,
    pub pages_routed: usize,
    pub pages_failed: usize,
    pub pages_cached: usize,
}

/// Augment `doc` in place by routing triage-qualifying pages through the
/// external Docling backend.
///
/// Per-page failures (network error, HTTP error, empty response) are logged
/// to stderr but do not abort — the page simply keeps its local rendering.
pub fn apply_to_document(
    doc: &mut Document,
    source_pdf: &Path,
    policy: RoutingPolicy,
    base_url: &str,
    timeout: Duration,
    cache_dir: Option<&Path>,
    verbose: bool,
) -> PdfpResult<HybridStats> {
    let mut stats = HybridStats {
        pages_total: doc.pages.len(),
        ..Default::default()
    };
    let client = client::DoclingClient::new(base_url, timeout);

    for page in doc.pages.iter_mut() {
        let should = matches!(policy, RoutingPolicy::All) || triage::should_route(page);
        if !should {
            continue;
        }

        if let Some(cache_dir) = cache_dir {
            let key = cache_key(source_pdf, page.page_num);
            let cached_path = cache_path(cache_dir, &key);
            if let Ok(md) = std::fs::read_to_string(&cached_path) {
                if !md.trim().is_empty() {
                    if verbose {
                        eprintln!(
                            "  hybrid: page {} loaded from cache {}",
                            page.page_num + 1,
                            cached_path.display()
                        );
                    }
                    page.override_markdown = Some(md);
                    stats.pages_routed += 1;
                    stats.pages_cached += 1;
                    continue;
                }
            }
        }

        let bytes = match page_extract::extract_page_as_pdf_bytes(source_pdf, page.page_num) {
            Ok(b) => b,
            Err(e) => {
                stats.pages_failed += 1;
                eprintln!(
                    "  hybrid: page {}: extract failed, keeping local output ({e})",
                    page.page_num + 1
                );
                continue;
            }
        };

        let filename = format!("page{}.pdf", page.page_num + 1);
        match client.convert_bytes_to_markdown(bytes, &filename) {
            Ok(md) => {
                if let Some(cache_dir) = cache_dir {
                    let key = cache_key(source_pdf, page.page_num);
                    let cached_path = cache_path(cache_dir, &key);
                    if let Err(e) = write_cache_entry(&cached_path, &md) {
                        eprintln!(
                            "  hybrid: page {}: cache write failed at {} ({e})",
                            page.page_num + 1,
                            cached_path.display()
                        );
                    }
                }
                if verbose {
                    eprintln!(
                        "  hybrid: page {} routed to {} (got {} bytes of md)",
                        page.page_num + 1,
                        base_url,
                        md.len()
                    );
                }
                page.override_markdown = Some(md);
                stats.pages_routed += 1;
            }
            Err(e) => {
                stats.pages_failed += 1;
                eprintln!(
                    "  hybrid: page {}: backend call failed, keeping local output ({e})",
                    page.page_num + 1
                );
            }
        }
    }

    Ok(stats)
}

pub(crate) fn cache_key(source_pdf: &Path, page_num: usize) -> String {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source_pdf.to_string_lossy().hash(&mut hasher);
    page_num.hash(&mut hasher);
    if let Ok(metadata) = std::fs::metadata(source_pdf) {
        metadata.len().hash(&mut hasher);
        if let Ok(modified) = metadata.modified() {
            if let Ok(duration) = modified.duration_since(std::time::UNIX_EPOCH) {
                duration.as_secs().hash(&mut hasher);
            }
        }
    }
    format!("{:016x}-p{}", hasher.finish(), page_num + 1)
}

fn cache_path(cache_dir: &Path, key: &str) -> PathBuf {
    cache_dir.join(format!("{key}.md"))
}

fn write_cache_entry(path: &Path, markdown: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, markdown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::{Bbox, Block, BlockKind, DocumentMetadata, Page};

    fn routed_page() -> Page {
        Page {
            page_num: 0,
            width: 612.0,
            height: 792.0,
            blocks: vec![Block {
                id: 0,
                bbox: Bbox::new(0.0, 0.0, 10.0, 10.0),
                text: String::new(),
                kind: BlockKind::Image { path: None },
                font_size: 0.0,
                font_name: "image".to_string(),
                page_num: 0,
                reading_order: 0,
                bold: false,
                italic: false,
                override_markdown: None,
            }],
            override_markdown: None,
        }
    }

    #[test]
    fn apply_uses_cached_markdown_without_backend_or_pdf_extraction() {
        let source_pdf = Path::new("missing-source.pdf");
        let cache_dir =
            std::env::temp_dir().join(format!("pdfp-hybrid-cache-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&cache_dir);
        std::fs::create_dir_all(&cache_dir).unwrap();

        let key = cache_key(source_pdf, 0);
        std::fs::write(cache_dir.join(format!("{key}.md")), "# Cached OCR\n").unwrap();

        let mut doc = Document {
            source_path: source_pdf.to_path_buf(),
            pages: vec![routed_page()],
            metadata: DocumentMetadata::default(),
        };

        let stats = apply_to_document(
            &mut doc,
            source_pdf,
            RoutingPolicy::All,
            "http://127.0.0.1:1",
            Duration::from_millis(1),
            Some(&cache_dir),
            false,
        )
        .unwrap();

        assert_eq!(stats.pages_routed, 1);
        assert_eq!(stats.pages_cached, 1);
        assert_eq!(
            doc.pages[0].override_markdown.as_deref(),
            Some("# Cached OCR\n")
        );

        let _ = std::fs::remove_dir_all(&cache_dir);
    }
}
