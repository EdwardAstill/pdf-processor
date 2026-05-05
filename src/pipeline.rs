use std::cmp::Ordering;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;

use crate::batch;
use crate::cli::{self, ConvertArgs};
use crate::document::types::{Block, BlockKind, Document, ImageRef, Page, RawPage};
use crate::formats;
use crate::hybrid;
use crate::layout::{
    classifier::Classifier,
    xycut::{assign_reading_order, build_xycut_order, XyCutConfig},
};
use crate::pdf::{self, extractor::PdfExtractor};
use crate::render::markdown::MarkdownRenderer;

pub fn process_pdf(pdf_path: &Path, args: &ConvertArgs) -> anyhow::Result<()> {
    if args.options.verbose {
        eprintln!("  processing PDF: {}", pdf_path.display());
    }

    let xycut_config = XyCutConfig {
        min_horizontal_gap: args.options.min_h_gap,
        min_vertical_gap: args.options.min_v_gap,
        ..Default::default()
    };

    let mut doc = build_document(pdf_path, args, &xycut_config)?;
    let scan_report = hybrid::triage::scan_report(&doc.pages);

    warn_on_scan_like_pages(pdf_path, args, &doc.pages, &scan_report);
    apply_hybrid_if_enabled(&mut doc, pdf_path, args, &scan_report)?;
    write_document(&doc, pdf_path, args)
}

fn build_document(
    pdf_path: &Path,
    args: &ConvertArgs,
    xycut_config: &XyCutConfig,
) -> anyhow::Result<Document> {
    let (raw_pages, metadata) = PdfExtractor::extract(pdf_path)
        .with_context(|| format!("Failed to extract {}", pdf_path.display()))?;

    let classifier = Classifier::new_for_document(&raw_pages);
    let output_dir = batch::output_dir_for(pdf_path, args.options.output.as_deref());
    let images_dir = output_dir.join("images");
    let extract_images = !args.options.no_images;

    let pages = raw_pages
        .into_iter()
        .map(|raw_page| {
            build_page(
                raw_page,
                pdf_path,
                &classifier,
                xycut_config,
                extract_images,
                &images_dir,
            )
        })
        .collect::<anyhow::Result<Vec<_>>>()?;

    Ok(Document {
        source_path: pdf_path.to_path_buf(),
        pages,
        metadata,
    })
}

fn build_page(
    raw_page: RawPage,
    pdf_path: &Path,
    classifier: &Classifier,
    xycut_config: &XyCutConfig,
    extract_images: bool,
    images_dir: &Path,
) -> anyhow::Result<Page> {
    let mut text_blocks = raw_page.blocks;
    let order = build_xycut_order(&text_blocks, xycut_config);
    assign_reading_order(&order, &mut text_blocks);

    let page_shell = RawPage {
        page_num: raw_page.page_num,
        width: raw_page.width,
        height: raw_page.height,
        blocks: Vec::new(),
        image_refs: Vec::new(),
    };
    let metadata = pdf::metadata::load_page_metadata(pdf_path, raw_page.page_num);
    let text_classified =
        classifier.classify_page_with_metadata(text_blocks, &page_shell, metadata.as_ref());

    let image_blocks = if extract_images && !raw_page.image_refs.is_empty() {
        save_page_images(&raw_page.image_refs, images_dir)?
    } else {
        Vec::new()
    };

    Ok(Page {
        page_num: raw_page.page_num,
        width: raw_page.width,
        height: raw_page.height,
        blocks: merge_text_and_images(text_classified, image_blocks),
        override_markdown: None,
    })
}

fn warn_on_scan_like_pages(
    pdf_path: &Path,
    args: &ConvertArgs,
    pages: &[Page],
    scan_report: &hybrid::triage::ScanReport,
) {
    let empty_page_count = pages.iter().filter(|p| p.blocks.is_empty()).count();
    if empty_page_count > 0 {
        eprintln!(
            "  warning: {} of {} pages have no extractable text (possibly scanned)",
            empty_page_count,
            pages.len()
        );
    }

    if !args.options.hybrid.is_on() && scan_report.likely_scan_like() {
        eprintln!(
            "  warning: {} looks scan-heavy ({} image-only / {} low-density page(s), {} page(s) with readable text); local output may be poor. Try `--hybrid docling`.",
            pdf_path.display(),
            scan_report.image_only_pages,
            scan_report.low_density_pages,
            scan_report.pages_with_readable_text
        );
    }
}

fn apply_hybrid_if_enabled(
    doc: &mut Document,
    pdf_path: &Path,
    args: &ConvertArgs,
    scan_report: &hybrid::triage::ScanReport,
) -> anyhow::Result<()> {
    if !args.options.hybrid.is_on() {
        return Ok(());
    }

    let policy = match args.options.hybrid_policy {
        cli::HybridPolicy::Auto if scan_report.likely_scan_like() => {
            if args.options.verbose {
                eprintln!(
                    "  hybrid: document looks scan-heavy; upgrading auto policy to route all pages"
                );
            }
            hybrid::RoutingPolicy::All
        }
        cli::HybridPolicy::Auto => hybrid::RoutingPolicy::Auto,
        cli::HybridPolicy::All => hybrid::RoutingPolicy::All,
    };

    let stats = hybrid::apply_to_document(
        doc,
        pdf_path,
        policy,
        &args.options.hybrid_url,
        Duration::from_secs(args.options.hybrid_timeout_secs),
        args.options.hybrid_cache_dir.as_deref(),
        args.options.verbose,
    )
    .with_context(|| {
        format!(
            "hybrid backend ({}) failed for {}",
            args.options.hybrid_url,
            pdf_path.display()
        )
    })?;

    if args.options.verbose {
        eprintln!(
            "  hybrid: routed {}/{} pages ({} failed)",
            stats.pages_routed, stats.pages_total, stats.pages_failed
        );
        if stats.pages_cached > 0 {
            eprintln!("  hybrid: cache hits {}", stats.pages_cached);
        }
    }

    Ok(())
}

fn save_page_images(image_refs: &[ImageRef], images_dir: &Path) -> anyhow::Result<Vec<Block>> {
    std::fs::create_dir_all(images_dir)
        .with_context(|| format!("Failed to create images dir {}", images_dir.display()))?;

    let mut blocks: Vec<Block> = Vec::with_capacity(image_refs.len());
    for img_ref in image_refs {
        let filename = format!(
            "page{}_img{}.{}",
            img_ref.page_num + 1,
            img_ref.image_index + 1,
            img_ref.format,
        );
        let abs_path = images_dir.join(&filename);
        std::fs::write(&abs_path, &img_ref.bytes)
            .with_context(|| format!("Failed to write image {}", abs_path.display()))?;
        let rel_path = format!("images/{filename}");
        blocks.push(Block {
            id: 1_000_000 + img_ref.image_index,
            bbox: img_ref.bbox,
            text: String::new(),
            kind: BlockKind::Image {
                path: Some(rel_path),
            },
            font_size: 0.0,
            font_name: "image".to_string(),
            page_num: img_ref.page_num,
            reading_order: 0,
        });
    }
    Ok(blocks)
}

fn merge_text_and_images(mut text: Vec<Block>, mut images: Vec<Block>) -> Vec<Block> {
    if images.is_empty() {
        return text;
    }
    text.sort_by_key(|b| b.reading_order);
    images.sort_by(|a, b| a.bbox.y0.partial_cmp(&b.bbox.y0).unwrap_or(Ordering::Equal));

    let mut result: Vec<Block> = Vec::with_capacity(text.len() + images.len());
    let mut img_iter = images.into_iter().peekable();
    let mut order: usize = 0;
    for mut tb in text {
        while let Some(peek) = img_iter.peek() {
            if peek.bbox.y0 < tb.bbox.y0 {
                let mut img = img_iter.next().expect("peek succeeded");
                img.reading_order = order;
                order += 1;
                result.push(img);
            } else {
                break;
            }
        }
        tb.reading_order = order;
        order += 1;
        result.push(tb);
    }
    for mut img in img_iter {
        img.reading_order = order;
        order += 1;
        result.push(img);
    }
    result
}

fn write_document(doc: &Document, input_path: &Path, args: &ConvertArgs) -> anyhow::Result<()> {
    let output_dir = batch::output_dir_for(input_path, args.options.output.as_deref());
    let renderer = MarkdownRenderer::new(!args.options.no_images, Some(output_dir.join("images")));
    let rendered = renderer
        .render_document(doc)
        .with_context(|| "Failed to render markdown")?;

    let stem = input_path
        .file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    formats::raw::RawFormat::write(&rendered, doc, &output_dir, &stem)
        .with_context(|| format!("Failed to write output to {}", output_dir.display()))
}
