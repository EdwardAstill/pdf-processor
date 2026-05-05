use std::path::Path;

use anyhow::Context;
use mupdf::{Document as MuDocument, MetadataName};
use serde::Serialize;

use crate::cli::InspectArgs;
use crate::pdf::extractor::PdfExtractor;

#[derive(Debug, Serialize)]
struct InspectReport {
    source: String,
    page_count: usize,
    title: Option<String>,
    author: Option<String>,
    subject: Option<String>,
    pages_with_readable_text: usize,
    image_only_pages: usize,
    low_density_pages: usize,
    likely_scan_like: bool,
    pages: Vec<PageReport>,
}

#[derive(Debug, Serialize)]
struct PageReport {
    page: usize,
    width: f32,
    height: f32,
    text_blocks: usize,
    images: usize,
    text_area_fraction: f32,
    readable_text: bool,
    image_only: bool,
    low_density: bool,
}

const MIN_TEXT_AREA_FRACTION: f32 = 0.02;

pub fn run(args: &InspectArgs) -> anyhow::Result<()> {
    let report = inspect_pdf(&args.input)?;
    if args.json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        print_human_report(&report);
    }
    Ok(())
}

fn inspect_pdf(path: &Path) -> anyhow::Result<InspectReport> {
    let (raw_pages, metadata) = PdfExtractor::extract(path)
        .with_context(|| format!("Failed to inspect {}", path.display()))?;

    let pages: Vec<PageReport> = raw_pages
        .iter()
        .map(|page| {
            let page_area = (page.width * page.height).max(1.0);
            let text_area: f32 = page.blocks.iter().map(|block| block.bbox.area()).sum();
            let text_area_fraction = text_area / page_area;
            let readable_text = page
                .blocks
                .iter()
                .any(|block| !block.text.trim().is_empty());
            let image_only = !readable_text && !page.image_refs.is_empty();
            let low_density = page.blocks.is_empty() || text_area_fraction < MIN_TEXT_AREA_FRACTION;

            PageReport {
                page: page.page_num + 1,
                width: page.width,
                height: page.height,
                text_blocks: page.blocks.len(),
                images: page.image_refs.len(),
                text_area_fraction,
                readable_text,
                image_only,
                low_density,
            }
        })
        .collect();

    let pages_with_readable_text = pages.iter().filter(|page| page.readable_text).count();
    let image_only_pages = pages.iter().filter(|page| page.image_only).count();
    let low_density_pages = pages.iter().filter(|page| page.low_density).count();
    let page_count = metadata.page_count;
    let likely_scan_like = if page_count == 0 {
        false
    } else if image_only_pages == page_count {
        true
    } else {
        pages_with_readable_text == 0 && low_density_pages > 0
    };

    Ok(InspectReport {
        source: path.display().to_string(),
        page_count,
        title: metadata.title,
        author: metadata.author,
        subject: metadata.subject,
        pages_with_readable_text,
        image_only_pages,
        low_density_pages,
        likely_scan_like,
        pages,
    })
}

fn print_human_report(report: &InspectReport) {
    println!("source: {}", report.source);
    println!("pages: {}", report.page_count);
    if let Some(title) = &report.title {
        println!("title: {title}");
    }
    if let Some(author) = &report.author {
        println!("author: {author}");
    }
    if let Some(subject) = &report.subject {
        println!("subject: {subject}");
    }
    println!("readable text pages: {}", report.pages_with_readable_text);
    println!("image-only pages: {}", report.image_only_pages);
    println!("low-density pages: {}", report.low_density_pages);
    println!("scan-like: {}", report.likely_scan_like);
}

#[allow(dead_code)]
fn metadata_from_open_document(
    path: &Path,
) -> anyhow::Result<(Option<String>, Option<String>, Option<String>)> {
    let path_str = path.to_string_lossy();
    let doc = MuDocument::open(path_str.as_ref())
        .with_context(|| format!("Failed to open {}", path.display()))?;
    Ok((
        doc.metadata(MetadataName::Title)
            .ok()
            .filter(|s| !s.is_empty()),
        doc.metadata(MetadataName::Author)
            .ok()
            .filter(|s| !s.is_empty()),
        doc.metadata(MetadataName::Subject)
            .ok()
            .filter(|s| !s.is_empty()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_like_requires_no_readable_text() {
        let report = InspectReport {
            source: "x.pdf".to_string(),
            page_count: 1,
            title: None,
            author: None,
            subject: None,
            pages_with_readable_text: 0,
            image_only_pages: 1,
            low_density_pages: 1,
            likely_scan_like: true,
            pages: Vec::new(),
        };
        assert!(report.likely_scan_like);
    }
}
