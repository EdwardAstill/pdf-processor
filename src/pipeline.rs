use std::cmp::Ordering;
use std::path::Path;
use std::time::Duration;

use anyhow::Context;

use crate::batch;
use crate::cli::{self, ConvertArgs, FigureMode, FormulaMode, TableMode};
use crate::document::types::{
    Bbox, Block, BlockKind, DetectedTable, Document, ImageRef, Page, RawPage, TableRender,
};
use crate::figure::{
    detect_figure_candidates, render_figure_snapshots, FigureCandidate, FigureDetectionOptions,
};
use crate::formats;
use crate::formula::detect::FormulaStatus;
use crate::formula::ocr::{FormulaSidecar, SubprocessSidecar};
#[cfg(feature = "onnx-ocr")]
use crate::formula::ocr_onnx::OnnxFormulaSidecar;
use crate::formula::{
    detect_formula_candidates, detect_visual_formula_candidates, FormulaCandidate,
};
use crate::hybrid;
use crate::layout::{
    classifier::Classifier,
    drawing_ops::extract_lines,
    furniture::detect_furniture_bboxes,
    table::{detect_coordinate_tables, TableCandidate},
    table_detector::{detect_table_region_candidates, GeometryTableRegion},
    xycut::{assign_reading_order, build_xycut_order, XyCutConfig},
};
use crate::ocr;
use crate::pdf::{self, extractor::PdfExtractor};
use crate::render::markdown::MarkdownRenderer;

pub fn process_pdf(pdf_path: &Path, args: &ConvertArgs) -> anyhow::Result<()> {
    if args.options.verbose {
        eprintln!("  processing PDF: {}", pdf_path.display());
    }

    let doc = process_pdf_to_document(pdf_path, args)?;
    write_document(&doc, pdf_path, args)
}

pub fn process_pdf_to_document(pdf_path: &Path, args: &ConvertArgs) -> anyhow::Result<Document> {
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
    Ok(doc)
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
    let furniture_mask = detect_furniture_bboxes(&raw_pages);
    let formula_sidecar = build_formula_sidecar(args.options.formula_sidecar.as_deref())?;
    let table_geometry_doc = mupdf::Document::open(pdf_path).ok();
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
        furniture_mask: &furniture_mask,
        formula_sidecar: formula_sidecar.as_deref(),
        table_geometry_doc: table_geometry_doc.as_ref(),
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

fn build_formula_sidecar(value: Option<&str>) -> anyhow::Result<Option<Box<dyn FormulaSidecar>>> {
    let Some(value) = value else {
        return Ok(None);
    };

    match cli::parse_formula_sidecar(value)? {
        cli::FormulaSidecarArg::Command(command) => {
            Ok(Some(Box::new(SubprocessSidecar::new(command))))
        }
        #[cfg(feature = "onnx-ocr")]
        cli::FormulaSidecarArg::Onnx(model_dir) => {
            let sidecar = OnnxFormulaSidecar::new(&model_dir).with_context(|| {
                format!(
                    "failed to initialise ONNX formula sidecar from {}",
                    model_dir.display()
                )
            })?;
            Ok(Some(Box::new(sidecar)))
        }
    }
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
    furniture_mask: &'a std::collections::HashMap<usize, Vec<Bbox>>,
    formula_sidecar: Option<&'a dyn FormulaSidecar>,
    table_geometry_doc: Option<&'a mupdf::Document>,
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
    let mut table_candidates =
        detect_coordinate_tables(&raw_page.words, raw_page.width, table_mode);
    table_candidates.extend(detect_geometry_table_candidates(
        ctx.table_geometry_doc,
        &raw_page,
        table_mode,
    ));
    let table_candidates = suppress_overlapping_table_candidates(table_candidates);
    let furniture_bboxes = ctx
        .furniture_mask
        .get(&raw_page.page_num)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let excluded_regions = formula_excluded_regions(&table_candidates, furniture_bboxes);
    if ctx.args.options.debug_tables && !matches!(table_mode, TableMode::Off) {
        write_table_debug(ctx.output_dir, raw_page.page_num, &table_candidates)?;
    }
    let mut formula_candidates = if matches!(formula_mode, FormulaMode::Off) {
        Vec::new()
    } else {
        detect_formula_candidates(&raw_page, &excluded_regions)
    };
    formula_candidates =
        suppress_formula_candidates_overlapping_tables(formula_candidates, &table_candidates);
    if ctx.args.options.debug_formulas && !matches!(formula_mode, FormulaMode::Off) {
        let visual_candidates = detect_visual_formula_candidates(
            ctx.pdf_path,
            &raw_page,
            &formula_candidates,
            &excluded_regions,
        )?;
        formula_candidates.extend(visual_candidates);
        renumber_formula_candidates(&mut formula_candidates);
    }
    if (ctx.args.options.debug_formulas || ctx.formula_sidecar.is_some())
        && !matches!(formula_mode, FormulaMode::Off)
    {
        write_formula_debug(
            ctx.pdf_path,
            ctx.output_dir,
            raw_page.page_num,
            &mut formula_candidates,
            ctx.args.options.figure_dpi,
            ctx.formula_sidecar,
            ctx.args.options.debug_formulas,
        )?;
    }
    let metadata = pdf::metadata::load_page_metadata(ctx.pdf_path, raw_page.page_num);
    let text_classified =
        ctx.classifier
            .classify_page_with_metadata(text_blocks, &page_shell, metadata.as_ref());
    let text_classified = suppress_text_covered_by_furniture(text_classified, furniture_bboxes);
    let text_classified = suppress_text_covered_by_tables(text_classified, &table_candidates);
    let formula_candidate_count = formula_candidates.len();
    let table_blocks = table_candidates_to_blocks(raw_page.page_num, table_candidates);
    let formula_blocks = formula_candidates_to_blocks(
        raw_page.page_num,
        formula_candidates,
        formula_mode,
        !ctx.args.options.conservative,
    );
    let text_classified = suppress_text_covered_by_formulas(text_classified, &formula_blocks);

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
            bold: false,
            italic: false,
        })
        .collect()
}

fn detect_geometry_table_candidates(
    mu_doc: Option<&mupdf::Document>,
    raw_page: &RawPage,
    mode: TableMode,
) -> Vec<TableCandidate> {
    if matches!(mode, TableMode::Off) {
        return Vec::new();
    }

    let Some(mu_doc) = mu_doc else {
        return Vec::new();
    };
    let Ok(mu_page) = mu_doc.load_page(raw_page.page_num as i32) else {
        return Vec::new();
    };
    let Ok((hlines, vlines)) = extract_lines(&mu_page, raw_page.width, raw_page.height) else {
        return Vec::new();
    };
    let regions = detect_table_region_candidates(
        &hlines,
        &vlines,
        &raw_page.words,
        raw_page.width,
        raw_page.height,
    );

    regions
        .into_iter()
        .filter_map(|region| geometry_region_to_table_candidate(region, mode))
        .collect()
}

fn geometry_region_to_table_candidate(
    region: GeometryTableRegion,
    mode: TableMode,
) -> Option<TableCandidate> {
    let render = match mode {
        TableMode::Off => return None,
        TableMode::Layout => TableRender::Layout {
            text: region.layout_text,
        },
        TableMode::Native => TableRender::Markdown,
        TableMode::Auto if region.row_consistency >= 0.80 && region.rows.len() >= 3 => {
            TableRender::Markdown
        }
        TableMode::Auto => TableRender::Layout {
            text: region.layout_text,
        },
    };

    Some(TableCandidate {
        table: DetectedTable {
            bbox: region.bbox,
            rows: region.rows,
            confidence: region.confidence,
            render,
        },
        source_block_ids: region.source_block_ids,
    })
}

fn suppress_overlapping_table_candidates(candidates: Vec<TableCandidate>) -> Vec<TableCandidate> {
    let mut kept: Vec<TableCandidate> = Vec::new();
    'candidate: for candidate in candidates {
        for existing in &kept {
            if bbox_overlap_smaller(candidate.table.bbox, existing.table.bbox) > 0.65 {
                continue 'candidate;
            }
        }
        kept.push(candidate);
    }
    kept
}

fn formula_excluded_regions(tables: &[TableCandidate], furniture_bboxes: &[Bbox]) -> Vec<Bbox> {
    let mut excluded = Vec::with_capacity(tables.len() + furniture_bboxes.len());
    excluded.extend(tables.iter().map(|candidate| candidate.table.bbox));
    excluded.extend_from_slice(furniture_bboxes);
    excluded
}

fn formula_candidates_to_blocks(
    page_num: usize,
    candidates: Vec<FormulaCandidate>,
    mode: cli::FormulaMode,
    render_math: bool,
) -> Vec<Block> {
    if matches!(mode, cli::FormulaMode::Off) {
        return Vec::new();
    }

    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(idx, candidate)| {
            if is_unresolved_formula_review(&candidate) {
                return Some(Block {
                    id: 3_100_000 + idx,
                    bbox: candidate.bbox,
                    text: String::new(),
                    kind: BlockKind::FormulaReview {
                        reason: candidate.reason,
                        crop_path: candidate.crop_path,
                    },
                    font_size: 0.0,
                    font_name: "formula-review".to_string(),
                    page_num,
                    reading_order: 0,
                    bold: false,
                    italic: false,
                });
            }

            if !render_math || !should_emit_formula_candidate(&candidate, mode) {
                return None;
            }

            let latex = build_formula_latex(&candidate);
            Some(Block {
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
                bold: false,
                italic: false,
            })
        })
        .collect()
}

fn is_unresolved_formula_review(candidate: &FormulaCandidate) -> bool {
    candidate.latex.is_none()
        && candidate.source_text.trim().is_empty()
        && candidate.backend.as_deref() == Some("visual-page-render")
}

fn should_emit_formula_candidate(candidate: &FormulaCandidate, mode: cli::FormulaMode) -> bool {
    if candidate.latex.is_none() && candidate.source_text.trim().is_empty() {
        return false;
    }
    match mode {
        cli::FormulaMode::Auto => candidate.confidence >= 70,
        cli::FormulaMode::Local | cli::FormulaMode::Hybrid => true,
        cli::FormulaMode::Off => false,
    }
}

fn renumber_formula_candidates(candidates: &mut [FormulaCandidate]) {
    for (idx, candidate) in candidates.iter_mut().enumerate() {
        candidate.formula_index = idx;
    }
}

fn build_formula_latex(candidate: &FormulaCandidate) -> String {
    candidate.latex.clone().unwrap_or_else(|| {
        let mut text = candidate.source_text.clone();
        if let Some(eq_num) = &candidate.equation_number {
            if let Some(stripped) = text.trim_end().strip_suffix(eq_num.as_str()) {
                let tag_inner = eq_num.trim_matches(|c| c == '(' || c == ')');
                text = format!("{} \\tag{{{}}}", stripped.trim_end(), tag_inner);
            }
        }
        unicode_to_latex(&text)
    })
}

fn unicode_to_latex(s: &str) -> String {
    s.chars()
        .fold(String::with_capacity(s.len() + 16), |mut out, c| {
            match c {
                'α' => out.push_str("\\alpha "),
                'β' => out.push_str("\\beta "),
                'γ' => out.push_str("\\gamma "),
                'δ' => out.push_str("\\delta "),
                'ε' => out.push_str("\\varepsilon "),
                'ζ' => out.push_str("\\zeta "),
                'η' => out.push_str("\\eta "),
                'θ' => out.push_str("\\theta "),
                'λ' => out.push_str("\\lambda "),
                'μ' => out.push_str("\\mu "),
                'ν' => out.push_str("\\nu "),
                'ξ' => out.push_str("\\xi "),
                'π' => out.push_str("\\pi "),
                'ρ' => out.push_str("\\rho "),
                'σ' => out.push_str("\\sigma "),
                'τ' => out.push_str("\\tau "),
                'φ' => out.push_str("\\phi "),
                'χ' => out.push_str("\\chi "),
                'ψ' => out.push_str("\\psi "),
                'ω' => out.push_str("\\omega "),
                'Γ' => out.push_str("\\Gamma "),
                'Δ' | '∆' => out.push_str("\\Delta "),
                'Θ' => out.push_str("\\Theta "),
                'Λ' => out.push_str("\\Lambda "),
                'Π' => out.push_str("\\Pi "),
                'Σ' => out.push_str("\\Sigma "),
                'Φ' => out.push_str("\\Phi "),
                'Ψ' => out.push_str("\\Psi "),
                'Ω' => out.push_str("\\Omega "),
                '∑' => out.push_str("\\sum "),
                '∏' => out.push_str("\\prod "),
                '∫' => out.push_str("\\int "),
                '∂' => out.push_str("\\partial "),
                '∞' => out.push_str("\\infty "),
                '√' => out.push_str("\\sqrt{} "),
                '±' => out.push_str("\\pm "),
                '∓' => out.push_str("\\mp "),
                '×' => out.push_str("\\times "),
                '÷' => out.push_str("\\div "),
                '≤' => out.push_str("\\leq "),
                '≥' => out.push_str("\\geq "),
                '≠' => out.push_str("\\neq "),
                '≈' => out.push_str("\\approx "),
                '∝' => out.push_str("\\propto "),
                '∈' => out.push_str("\\in "),
                '∉' => out.push_str("\\notin "),
                '⊂' => out.push_str("\\subset "),
                '∪' => out.push_str("\\cup "),
                '∩' => out.push_str("\\cap "),
                '−' => out.push('-'),
                _ => out.push(c),
            }
            out
        })
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

fn suppress_text_covered_by_furniture(blocks: Vec<Block>, furniture_bboxes: &[Bbox]) -> Vec<Block> {
    if furniture_bboxes.is_empty() {
        return blocks;
    }

    blocks
        .into_iter()
        .filter(|block| {
            !furniture_bboxes
                .iter()
                .any(|bbox| bbox_overlap_ratio(block.bbox, *bbox) > 0.55)
        })
        .collect()
}

fn suppress_text_covered_by_formulas(blocks: Vec<Block>, candidates: &[Block]) -> Vec<Block> {
    if candidates.is_empty() {
        return blocks;
    }

    blocks
        .into_iter()
        .filter(|block| {
            !candidates.iter().any(|candidate| {
                matches!(candidate.kind, BlockKind::Formula { .. })
                    && bbox_overlap_ratio(block.bbox, candidate.bbox) > 0.55
            })
        })
        .collect()
}

fn suppress_formula_candidates_overlapping_tables(
    candidates: Vec<FormulaCandidate>,
    tables: &[TableCandidate],
) -> Vec<FormulaCandidate> {
    if candidates.is_empty() || tables.is_empty() {
        return candidates;
    }

    candidates
        .into_iter()
        .filter(|candidate| {
            !tables.iter().any(|table| {
                table.table.confidence >= 0.70
                    && bbox_overlap_ratio(candidate.bbox, table.table.bbox) > 0.55
            })
        })
        .enumerate()
        .map(|(idx, mut candidate)| {
            candidate.formula_index = idx;
            candidate
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

fn bbox_overlap_smaller(a: crate::document::types::Bbox, b: crate::document::types::Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    let intersection = ((x1 - x0).max(0.0)) * ((y1 - y0).max(0.0));
    intersection / a.area().min(b.area()).max(1.0)
}

fn write_table_debug(
    output_dir: &Path,
    page_num: usize,
    candidates: &[TableCandidate],
) -> anyhow::Result<()> {
    #[derive(serde::Serialize)]
    struct DebugTable<'a> {
        table_region: Bbox,
        confidence: f32,
        render: &'a TableRender,
        rows: &'a [Vec<String>],
    }

    let debug_dir = output_dir.join("debug").join("tables");
    std::fs::create_dir_all(&debug_dir)
        .with_context(|| format!("Failed to create table debug dir {}", debug_dir.display()))?;
    let path = debug_dir.join(format!("page{}.json", page_num + 1));
    let tables: Vec<_> = candidates
        .iter()
        .map(|candidate| DebugTable {
            table_region: candidate.table.bbox,
            confidence: candidate.table.confidence,
            render: &candidate.table.render,
            rows: &candidate.table.rows,
        })
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
    sidecar: Option<&dyn FormulaSidecar>,
    write_json: bool,
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
                if let Some(sidecar) = sidecar {
                    if should_send_to_formula_sidecar(candidate) {
                        candidate.latex = sidecar.recognize(&abs_path);
                        if candidate.latex.is_some() {
                            candidate.status = FormulaStatus::BackendRecovered;
                            candidate.backend = Some("formula-sidecar".to_string());
                        }
                    }
                }
            }
        }
    }

    if write_json && !candidates.is_empty() {
        let path = debug_dir.join(format!("page{}.json", page_num + 1));
        let json = serde_json::to_string_pretty(candidates)?;
        std::fs::write(&path, json)
            .with_context(|| format!("Failed to write formula debug {}", path.display()))?;
    }

    Ok(())
}

fn should_send_to_formula_sidecar(candidate: &FormulaCandidate) -> bool {
    candidate.latex.is_none()
        && candidate.confidence >= 70
        && matches!(candidate.status, FormulaStatus::LocalCandidate)
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
            bold: false,
            italic: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::{Bbox, DetectedTable, TableRender};
    use crate::formula::detect::{FormulaCandidate, FormulaStatus};
    use std::collections::BTreeSet;

    #[test]
    fn suppresses_formula_candidates_inside_strong_tables() {
        let table = TableCandidate {
            table: DetectedTable {
                bbox: Bbox::new(80.0, 100.0, 520.0, 240.0),
                rows: vec![
                    vec!["Item".into(), "2025".into(), "2024".into()],
                    vec!["Revenue".into(), "100".into(), "90".into()],
                ],
                confidence: 0.86,
                render: TableRender::Markdown,
            },
            source_block_ids: BTreeSet::new(),
        };
        let inside = FormulaCandidate {
            page_num: 0,
            formula_index: 0,
            bbox: Bbox::new(120.0, 140.0, 500.0, 162.0),
            source_text: "Revenue + assets = total".into(),
            equation_number: None,
            confidence: 80,
            status: FormulaStatus::LocalCandidate,
            backend: None,
            latex: None,
            reason: "relation+math-symbols".into(),
            crop_path: None,
        };
        let outside = FormulaCandidate {
            page_num: 0,
            formula_index: 1,
            bbox: Bbox::new(160.0, 320.0, 460.0, 342.0),
            source_text: "F = m a".into(),
            equation_number: Some("(1)".into()),
            confidence: 88,
            status: FormulaStatus::LocalCandidate,
            backend: None,
            latex: None,
            reason: "centered+equation-number+relation".into(),
            crop_path: None,
        };

        let filtered =
            suppress_formula_candidates_overlapping_tables(vec![inside, outside], &[table]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source_text, "F = m a");
        assert_eq!(filtered[0].formula_index, 0);
    }

    #[test]
    fn formula_exclusions_include_tables_and_furniture() {
        let table = TableCandidate {
            table: DetectedTable {
                bbox: Bbox::new(80.0, 100.0, 520.0, 240.0),
                rows: Vec::new(),
                confidence: 0.86,
                render: TableRender::Markdown,
            },
            source_block_ids: BTreeSet::new(),
        };
        let footer = Bbox::new(0.0, 780.0, 595.0, 842.0);

        let excluded = formula_excluded_regions(&[table], &[footer]);

        assert_eq!(excluded, vec![Bbox::new(80.0, 100.0, 520.0, 240.0), footer]);
    }

    #[test]
    fn furniture_bboxes_suppress_text_blocks() {
        let footer_text = Block {
            id: 42,
            bbox: Bbox::new(40.0, 790.0, 300.0, 805.0),
            text: "Downloaded by ACME".into(),
            kind: BlockKind::Paragraph,
            font_size: 8.0,
            font_name: String::new(),
            page_num: 0,
            reading_order: 0,
            bold: false,
            italic: false,
        };
        let body_text = Block {
            id: 43,
            bbox: Bbox::new(40.0, 120.0, 300.0, 140.0),
            text: "Body text".into(),
            kind: BlockKind::Paragraph,
            font_size: 10.0,
            font_name: String::new(),
            page_num: 0,
            reading_order: 1,
            bold: false,
            italic: false,
        };
        let furniture = Bbox::new(0.0, 780.0, 595.0, 842.0);

        let filtered =
            suppress_text_covered_by_furniture(vec![footer_text, body_text], &[furniture]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Body text");
    }

    fn formula_candidate(source_text: &str, confidence: u8) -> FormulaCandidate {
        FormulaCandidate {
            page_num: 0,
            formula_index: 0,
            bbox: Bbox::new(120.0, 140.0, 500.0, 162.0),
            source_text: source_text.into(),
            equation_number: None,
            confidence,
            status: FormulaStatus::LocalCandidate,
            backend: None,
            latex: None,
            reason: "test".into(),
            crop_path: None,
        }
    }

    #[test]
    fn auto_mode_promotes_only_high_confidence_formula_candidates() {
        let high = formula_candidate("E = mc^2", 70);
        let low = formula_candidate("a + b", 69);

        let blocks = formula_candidates_to_blocks(0, vec![high, low], FormulaMode::Auto, true);

        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0].kind, BlockKind::Formula { .. }));
        assert_eq!(blocks[0].text, "E = mc^2");
    }

    #[test]
    fn formula_latex_strips_equation_number_and_adds_tag() {
        let mut candidate = formula_candidate("E = mc^2 (12)", 88);
        candidate.equation_number = Some("(12)".into());

        assert_eq!(build_formula_latex(&candidate), "E = mc^2 \\tag{12}");
    }

    #[test]
    fn formula_latex_normalizes_unicode_including_theta() {
        let candidate = formula_candidate("σ = √x + θ − Δ", 88);

        assert_eq!(
            build_formula_latex(&candidate),
            "\\sigma  = \\sqrt{} x + \\theta  - \\Delta "
        );
    }

    #[test]
    fn formula_blocks_suppress_overlapping_text_blocks() {
        let formula = formula_candidates_to_blocks(
            0,
            vec![formula_candidate("F = ma", 88)],
            FormulaMode::Auto,
            true,
        );
        let text = Block {
            id: 42,
            bbox: Bbox::new(121.0, 141.0, 499.0, 161.0),
            text: "F = ma".into(),
            kind: BlockKind::Paragraph,
            font_size: 10.0,
            font_name: String::new(),
            page_num: 0,
            reading_order: 0,
            bold: false,
            italic: false,
        };

        let filtered = suppress_text_covered_by_formulas(vec![text], &formula);

        assert!(filtered.is_empty());
    }

    #[test]
    fn visual_only_candidate_becomes_review_block_without_math_rendering() {
        let mut candidate = formula_candidate("", 68);
        candidate.backend = Some("visual-page-render".into());
        candidate.reason = "visual-isolated-equation-band+cue:Hence:".into();
        candidate.crop_path = Some("debug/formulas/page1_formula1.png".into());

        let blocks = formula_candidates_to_blocks(0, vec![candidate], FormulaMode::Auto, false);

        assert_eq!(blocks.len(), 1);
        match &blocks[0].kind {
            BlockKind::FormulaReview { reason, crop_path } => {
                assert!(reason.contains("visual-isolated-equation-band"));
                assert_eq!(
                    crop_path.as_deref(),
                    Some("debug/formulas/page1_formula1.png")
                );
            }
            other => panic!("expected formula review block, got {other:?}"),
        }
    }
}
