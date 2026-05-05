use std::cmp::Ordering;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;

use crate::batch;
use crate::cli::{self, ConvertArgs, FigureMode, FormulaMode, TableMode};
use crate::document::types::{Block, BlockKind, Document, ImageRef, Page, RawPage};
use crate::figure::{
    detect_figure_candidates, render_figure_snapshots, FigureCandidate, FigureDetectionOptions,
};
use crate::formats;
use crate::formula::{detect_formula_candidates, FormulaCandidate};
use crate::hybrid;
use crate::layout::{
    classifier::Classifier,
    table::{detect_coordinate_tables, TableCandidate},
    xycut::{assign_reading_order, build_xycut_order, XyCutConfig},
};
use crate::ocr;
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

    let (raw_pages, metadata) = PdfExtractor::extract(pdf_path)
        .with_context(|| format!("Failed to extract {}", pdf_path.display()))?;
    let ocr_report = ocr::triage::triage_raw_pages(&raw_pages);
    let prepared = ocr::prepare_pdf_with_report(
        pdf_path,
        &args.options.ocr,
        &ocr_report,
        args.options.verbose,
    )?;

    let mut doc = if prepared.effective_path == pdf_path {
        build_document_from_raw(pdf_path, pdf_path, raw_pages, metadata, args, &xycut_config)?
    } else {
        build_document(&prepared.effective_path, pdf_path, args, &xycut_config)?
    };
    let scan_report = hybrid::triage::scan_report(&doc.pages);

    warn_on_scan_like_pages(pdf_path, args, &doc.pages, &scan_report);
    apply_hybrid_if_enabled(&mut doc, pdf_path, args, &scan_report)?;
    write_document(&doc, pdf_path, args)
}

fn build_document(
    pdf_path: &Path,
    output_base_path: &Path,
    args: &ConvertArgs,
    xycut_config: &XyCutConfig,
) -> anyhow::Result<Document> {
    let (raw_pages, metadata) = PdfExtractor::extract(pdf_path)
        .with_context(|| format!("Failed to extract {}", pdf_path.display()))?;
    build_document_from_raw(
        pdf_path,
        output_base_path,
        raw_pages,
        metadata,
        args,
        xycut_config,
    )
}

fn build_document_from_raw(
    pdf_path: &Path,
    output_base_path: &Path,
    raw_pages: Vec<RawPage>,
    metadata: crate::document::types::DocumentMetadata,
    args: &ConvertArgs,
    xycut_config: &XyCutConfig,
) -> anyhow::Result<Document> {
    let classifier = Classifier::new_for_document(&raw_pages);
    let output_dir = batch::output_dir_for(output_base_path, args.options.output.as_deref());
    let images_dir = output_dir.join("images");
    let figure_mode = args.options.effective_figure_mode();
    let extract_embedded_images =
        !args.options.no_images && matches!(figure_mode, FigureMode::Embedded | FigureMode::Both);
    let extract_snapshot_figures =
        !args.options.no_images && matches!(figure_mode, FigureMode::Snapshot | FigureMode::Both);

    let page_build_context = PageBuildContext {
        pdf_path,
        classifier: &classifier,
        xycut_config,
        extract_embedded_images,
        extract_snapshot_figures,
        images_dir: &images_dir,
        output_dir: &output_dir,
        args,
    };

    let mut pages = Vec::new();
    let mut formula_candidate_pages = 0usize;
    let mut formula_candidate_count = 0usize;
    for raw_page in raw_pages {
        let built = build_page(raw_page, &page_build_context)?;
        if built.formula_candidate_count > 0 {
            formula_candidate_pages += 1;
            formula_candidate_count += built.formula_candidate_count;
        }
        pages.push(built.page);
    }
    warn_on_formula_candidate_summary(args, formula_candidate_pages, formula_candidate_count);

    Ok(Document {
        source_path: pdf_path.to_path_buf(),
        pages,
        metadata,
    })
}

struct PageBuildContext<'a> {
    pdf_path: &'a Path,
    classifier: &'a Classifier,
    xycut_config: &'a XyCutConfig,
    extract_embedded_images: bool,
    extract_snapshot_figures: bool,
    images_dir: &'a Path,
    output_dir: &'a Path,
    args: &'a ConvertArgs,
}

struct BuiltPage {
    page: Page,
    formula_candidate_count: usize,
}

fn build_page(mut raw_page: RawPage, ctx: &PageBuildContext<'_>) -> anyhow::Result<BuiltPage> {
    let mut text_blocks = std::mem::take(&mut raw_page.blocks);
    let order = build_xycut_order(&text_blocks, ctx.xycut_config);
    assign_reading_order(&order, &mut text_blocks);

    let page_shell = RawPage {
        page_num: raw_page.page_num,
        width: raw_page.width,
        height: raw_page.height,
        blocks: Vec::new(),
        words: Vec::new(),
        image_refs: Vec::new(),
    };
    let table_mode = ctx.args.options.effective_table_mode();
    let formula_mode = ctx.args.options.effective_formula_mode();
    let table_candidates = detect_coordinate_tables(&raw_page.words, raw_page.width, table_mode);
    if ctx.args.options.debug_tables && !matches!(table_mode, TableMode::Off) {
        write_table_debug(ctx.output_dir, raw_page.page_num, &table_candidates)?;
    }
    let mut formula_candidates = if matches!(formula_mode, FormulaMode::Off) {
        Vec::new()
    } else {
        detect_formula_candidates(&raw_page)
    };
    if ctx.args.options.debug_formulas && !matches!(formula_mode, FormulaMode::Off) {
        write_formula_debug(
            ctx.pdf_path,
            ctx.output_dir,
            raw_page.page_num,
            &mut formula_candidates,
            ctx.args.options.figure_dpi,
        )?;
    }
    let metadata = pdf::metadata::load_page_metadata(ctx.pdf_path, raw_page.page_num);
    let text_classified =
        ctx.classifier
            .classify_page_with_metadata(text_blocks, &page_shell, metadata.as_ref());
    let text_classified = suppress_text_covered_by_tables(text_classified, &table_candidates);
    let formula_candidate_count = formula_candidates.len();
    let table_blocks = table_candidates_to_blocks(raw_page.page_num, table_candidates);
    let formula_blocks =
        formula_candidates_to_blocks(raw_page.page_num, formula_candidates, formula_mode);

    let figure_candidates = if ctx.extract_snapshot_figures {
        detect_figure_candidates(
            &raw_page,
            &text_classified,
            FigureDetectionOptions {
                padding: ctx.args.options.figure_padding,
                ..Default::default()
            },
        )
    } else {
        Vec::new()
    };

    if ctx.args.options.debug_figures && ctx.extract_snapshot_figures {
        write_figure_debug(ctx.output_dir, raw_page.page_num, &figure_candidates)?;
    }

    let image_blocks = if ctx.extract_embedded_images && !raw_page.image_refs.is_empty() {
        save_page_images(&raw_page.image_refs, ctx.images_dir)?
    } else {
        Vec::new()
    };
    let figure_blocks = if ctx.extract_snapshot_figures && !figure_candidates.is_empty() {
        render_figure_snapshots(
            ctx.pdf_path,
            raw_page.page_num,
            &figure_candidates,
            ctx.images_dir,
            ctx.args.options.figure_dpi,
        )?
        .into_iter()
        .map(|rendered| rendered.block)
        .collect()
    } else {
        Vec::new()
    };

    Ok(BuiltPage {
        page: Page {
            page_num: raw_page.page_num,
            width: raw_page.width,
            height: raw_page.height,
            blocks: merge_text_and_images(
                merge_text_and_formulas(
                    merge_text_and_tables(text_classified, table_blocks),
                    formula_blocks,
                ),
                merge_media_blocks(image_blocks, figure_blocks),
            ),
            override_markdown: None,
        },
        formula_candidate_count,
    })
}

fn merge_text_and_formulas(mut text: Vec<Block>, mut formulas: Vec<Block>) -> Vec<Block> {
    if formulas.is_empty() {
        return text;
    }
    text.sort_by_key(|block| block.reading_order);
    formulas.sort_by(|a, b| {
        a.bbox
            .y0
            .partial_cmp(&b.bbox.y0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap_or(Ordering::Equal))
    });

    let mut result = Vec::with_capacity(text.len() + formulas.len());
    let mut formula_iter = formulas.into_iter().peekable();
    let mut order = 0usize;
    for mut block in text {
        while let Some(formula) = formula_iter.peek() {
            if formula.bbox.y0 < block.bbox.y0 {
                let mut formula = formula_iter.next().expect("peek succeeded");
                formula.reading_order = order;
                order += 1;
                result.push(formula);
            } else {
                break;
            }
        }
        block.reading_order = order;
        order += 1;
        result.push(block);
    }
    for mut formula in formula_iter {
        formula.reading_order = order;
        order += 1;
        result.push(formula);
    }
    result
}

fn merge_text_and_tables(mut text: Vec<Block>, mut tables: Vec<Block>) -> Vec<Block> {
    if tables.is_empty() {
        return text;
    }
    text.sort_by_key(|block| block.reading_order);
    tables.sort_by(|a, b| {
        a.bbox
            .y0
            .partial_cmp(&b.bbox.y0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap_or(Ordering::Equal))
    });

    let mut result = Vec::with_capacity(text.len() + tables.len());
    let mut table_iter = tables.into_iter().peekable();
    let mut order = 0usize;
    for mut block in text {
        while let Some(table) = table_iter.peek() {
            if table.bbox.y0 < block.bbox.y0 {
                let mut table = table_iter.next().expect("peek succeeded");
                table.reading_order = order;
                order += 1;
                result.push(table);
            } else {
                break;
            }
        }
        block.reading_order = order;
        order += 1;
        result.push(block);
    }
    for mut table in table_iter {
        table.reading_order = order;
        order += 1;
        result.push(table);
    }
    result
}

fn table_candidates_to_blocks(page_num: usize, candidates: Vec<TableCandidate>) -> Vec<Block> {
    candidates
        .into_iter()
        .enumerate()
        .map(|(idx, candidate)| Block {
            id: 2_000_000 + idx,
            bbox: candidate.table.bbox,
            text: String::new(),
            kind: BlockKind::CoordinateTable {
                table: candidate.table,
            },
            font_size: 0.0,
            font_name: "table".to_string(),
            page_num,
            reading_order: 0,
        })
        .collect()
}

fn formula_candidates_to_blocks(
    page_num: usize,
    candidates: Vec<FormulaCandidate>,
    mode: cli::FormulaMode,
) -> Vec<Block> {
    if !matches!(mode, cli::FormulaMode::Local | cli::FormulaMode::Hybrid) {
        return Vec::new();
    }

    candidates
        .into_iter()
        .enumerate()
        .map(|(idx, candidate)| {
            let latex = candidate
                .latex
                .clone()
                .unwrap_or_else(|| candidate.source_text.clone());
            Block {
                id: 3_000_000 + idx,
                bbox: candidate.bbox,
                text: candidate.source_text,
                kind: BlockKind::Formula {
                    latex,
                    display: true,
                },
                font_size: 0.0,
                font_name: "formula-candidate".to_string(),
                page_num,
                reading_order: 0,
            }
        })
        .collect()
}

fn suppress_text_covered_by_tables(
    blocks: Vec<Block>,
    candidates: &[TableCandidate],
) -> Vec<Block> {
    if candidates.is_empty() {
        return blocks;
    }

    blocks
        .into_iter()
        .filter(|block| {
            !candidates.iter().any(|candidate| {
                candidate.source_block_ids.contains(&block.id)
                    || bbox_overlap_ratio(block.bbox, candidate.table.bbox) > 0.55
            })
        })
        .collect()
}

fn bbox_overlap_ratio(a: crate::document::types::Bbox, b: crate::document::types::Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    let intersection = ((x1 - x0).max(0.0)) * ((y1 - y0).max(0.0));
    intersection / a.area().max(1.0)
}

fn write_table_debug(
    output_dir: &Path,
    page_num: usize,
    candidates: &[TableCandidate],
) -> anyhow::Result<()> {
    let debug_dir = output_dir.join("debug").join("tables");
    std::fs::create_dir_all(&debug_dir)
        .with_context(|| format!("Failed to create table debug dir {}", debug_dir.display()))?;
    let path = debug_dir.join(format!("page{}.json", page_num + 1));
    let tables: Vec<_> = candidates
        .iter()
        .map(|candidate| &candidate.table)
        .collect();
    let json = serde_json::to_string_pretty(&tables)?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write table debug {}", path.display()))
}

fn write_figure_debug(
    output_dir: &Path,
    page_num: usize,
    candidates: &[FigureCandidate],
) -> anyhow::Result<()> {
    let debug_dir = output_dir.join("debug").join("figures");
    std::fs::create_dir_all(&debug_dir)
        .with_context(|| format!("Failed to create figure debug dir {}", debug_dir.display()))?;
    let path = debug_dir.join(format!("page{}.json", page_num + 1));
    let json = serde_json::to_string_pretty(candidates)?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write figure debug {}", path.display()))
}

fn write_formula_debug(
    pdf_path: &Path,
    output_dir: &Path,
    page_num: usize,
    candidates: &mut [FormulaCandidate],
    dpi: u32,
) -> anyhow::Result<()> {
    let debug_dir = output_dir.join("debug").join("formulas");
    std::fs::create_dir_all(&debug_dir)
        .with_context(|| format!("Failed to create formula debug dir {}", debug_dir.display()))?;

    if !candidates.is_empty() {
        let document = mupdf::Document::open(pdf_path).with_context(|| {
            format!(
                "Failed to open {} for formula rendering",
                pdf_path.display()
            )
        })?;
        let page = document.load_page(page_num as i32).with_context(|| {
            format!("Failed to load page {} for formula rendering", page_num + 1)
        })?;

        for candidate in candidates.iter_mut() {
            let filename = format!(
                "page{}_formula{}.png",
                page_num + 1,
                candidate.formula_index + 1
            );
            let abs_path = debug_dir.join(&filename);
            if let Some(bytes) = crate::figure::render::render_bbox_png(&page, candidate.bbox, dpi)
                .with_context(|| format!("Failed to render formula crop {filename}"))?
            {
                std::fs::write(&abs_path, bytes).with_context(|| {
                    format!("Failed to write formula crop {}", abs_path.display())
                })?;
                candidate.crop_path = Some(format!("debug/formulas/{filename}"));
            }
        }
    }

    let path = debug_dir.join(format!("page{}.json", page_num + 1));
    let json = serde_json::to_string_pretty(candidates)?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write formula debug {}", path.display()))
}

fn warn_on_formula_candidate_summary(
    args: &ConvertArgs,
    page_count: usize,
    candidate_count: usize,
) {
    if candidate_count == 0 || args.options.hybrid.is_on() {
        return;
    }
    if matches!(
        args.options.effective_formula_mode(),
        FormulaMode::Auto | FormulaMode::Hybrid
    ) {
        eprintln!(
            "  warning: detected {candidate_count} formula candidate(s) across {page_count} page(s); use `--debug-formulas` to inspect crops or `--hybrid docling --formulas hybrid` for formula enrichment.",
        );
    }
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
        let next_step = if matches!(args.options.ocr.ocr, cli::OcrMode::Off) {
            "Try `--ocr auto` for local OCR, or `--hybrid docling` for external assist."
        } else {
            "Check `--ocr-lang`, or try `--hybrid docling` for external assist."
        };
        eprintln!(
            "  warning: {} looks scan-heavy ({} image-only / {} low-density page(s), {} page(s) with readable text); local output may be poor. {}",
            pdf_path.display(),
            scan_report.image_only_pages,
            scan_report.low_density_pages,
            scan_report.pages_with_readable_text,
            next_step
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

fn merge_media_blocks(mut left: Vec<Block>, right: Vec<Block>) -> Vec<Block> {
    left.extend(right);
    left
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
