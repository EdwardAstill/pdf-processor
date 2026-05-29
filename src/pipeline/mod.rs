mod merge;

use std::path::Path;
use std::time::Duration;

use anyhow::Context;
use serde::Serialize;

use crate::batch;
use crate::cli::{self, ConvertArgs, FigureMode, FormulaEmitMode, FormulaMode, TableMode};
use crate::document::types::{
    Bbox, Block, BlockKind, DetectedTable, Document, ImageRef, Page, RawPage, TableRender,
};
use crate::figure::{
    detect_figure_candidates, render_figure_snapshots, FigureCandidate, FigureDetectionOptions,
};
use crate::formats;
use crate::formula::detect::FormulaStatus;
use crate::formula::geometric::geometric_latex;
use crate::formula::ocr::{
    FormulaSidecar, FormulaSidecarAttempt, FormulaSidecarStatus, SubprocessSidecar,
};
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

use merge::{
    formula_excluded_regions, merge_media_blocks, merge_text_and_formulas, merge_text_and_images,
    merge_text_and_tables, suppress_formula_candidates_overlapping_tables,
    suppress_overlapping_table_candidates, suppress_text_covered_by_formulas,
    suppress_text_covered_by_furniture, suppress_text_covered_by_tables,
};

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
    let formula_sidecar = build_formula_sidecar(
        args.options.formula_sidecar.as_deref(),
        Duration::from_secs(args.options.formula_sidecar_timeout_secs),
    )?;
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
    let mut formula_records = Vec::new();
    let mut formula_candidate_pages = 0usize;
    let mut formula_candidate_count = 0usize;
    for raw_page in raw_pages {
        let built = build_page(raw_page, &page_build_context)?;
        if built.formula_candidate_count > 0 {
            formula_candidate_pages += 1;
            formula_candidate_count += built.formula_candidate_count;
        }
        formula_records.extend(built.formula_records);
        pages.push(built.page);
    }
    warn_on_formula_candidate_summary(args, formula_candidate_pages, formula_candidate_count);
    if args.options.debug_formulas || args.options.formula_sidecar.is_some() {
        write_formula_index(&output_dir, output_base_path, pages.len(), &formula_records)?;
    }

    Ok(Document {
        source_path: pdf_path.to_path_buf(),
        pages,
        metadata,
    })
}

fn build_formula_sidecar(
    value: Option<&str>,
    timeout: Duration,
) -> anyhow::Result<Option<Box<dyn FormulaSidecar>>> {
    let Some(value) = value else {
        return Ok(None);
    };

    match cli::parse_formula_sidecar(value)? {
        cli::FormulaSidecarArg::Command(command) => Ok(Some(Box::new(
            SubprocessSidecar::with_timeout(command, timeout),
        ))),
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
    formula_records: Vec<FormulaReportRecord>,
}

#[derive(Debug, Serialize)]
struct FormulaDebugIndex<'a> {
    schema_version: u8,
    source_pdf: String,
    page_count: usize,
    candidate_count: usize,
    pages_with_candidates: usize,
    local_candidate_count: usize,
    needs_review_count: usize,
    backend_recovered_count: usize,
    emitted_count: usize,
    review_block_count: usize,
    candidates: &'a [FormulaReportRecord],
}

#[derive(Clone, Debug, Serialize)]
struct FormulaReportRecord {
    page: usize,
    formula_index: usize,
    confidence: u8,
    status: FormulaStatus,
    backend: Option<String>,
    emitted: bool,
    review_block: bool,
    equation_number: Option<String>,
    crop_path: Option<String>,
    source_text: String,
    latex: Option<String>,
    sidecar: FormulaSidecarAttempt,
    sanity: Option<String>,
    emission_reason: String,
    reason: String,
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
    let table_candidates: Vec<TableCandidate> = table_candidates
        .into_iter()
        .filter(|candidate| candidate.should_emit(raw_page.width, raw_page.height))
        .collect();
    let furniture_bboxes = ctx
        .furniture_mask
        .get(&raw_page.page_num)
        .map(Vec::as_slice)
        .unwrap_or(&[]);
    let formula_blocking_tables: Vec<TableCandidate> = table_candidates
        .iter()
        .filter(|candidate| {
            candidate.table.confidence >= 0.70
                && !is_broad_layout_table_candidate(candidate, raw_page.height)
        })
        .cloned()
        .collect();
    let excluded_regions = formula_excluded_regions(&formula_blocking_tables, furniture_bboxes);
    if ctx.args.options.debug_tables && !matches!(table_mode, TableMode::Off) {
        write_table_debug(ctx.output_dir, raw_page.page_num, &table_candidates)?;
    }
    let mut formula_candidates = if matches!(formula_mode, FormulaMode::Off) {
        Vec::new()
    } else {
        detect_formula_candidates(&raw_page, &excluded_regions)
    };
    formula_candidates = suppress_formula_candidates_overlapping_tables(
        formula_candidates,
        &formula_blocking_tables,
    );
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
    let formula_records = formula_report_records(
        &formula_candidates,
        formula_mode,
        ctx.args.options.effective_render_math(),
        ctx.args.options.formula_emit,
    );
    let table_blocks = table_candidates_to_blocks(raw_page.page_num, table_candidates);
    let formula_blocks = formula_candidates_to_blocks(
        raw_page.page_num,
        formula_candidates,
        formula_mode,
        ctx.args.options.effective_render_math(),
        ctx.args.options.formula_emit,
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
        formula_records,
    })
}

fn table_candidates_to_blocks(page_num: usize, candidates: Vec<TableCandidate>) -> Vec<Block> {
    candidates
        .into_iter()
        .enumerate()
        .map(|(idx, candidate)| {
            Block::special(
                2_000_000 + idx,
                candidate.table.bbox,
                BlockKind::CoordinateTable {
                    table: candidate.table,
                },
                page_num,
                0.0,
                "table".to_string(),
            )
        })
        .collect()
}

fn is_broad_layout_table_candidate(candidate: &TableCandidate, page_height: f32) -> bool {
    candidate.is_broad_layout_candidate(page_height)
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
        evidence: region.evidence,
    })
}

fn formula_report_records(
    candidates: &[FormulaCandidate],
    mode: cli::FormulaMode,
    render_math: bool,
    emit_mode: FormulaEmitMode,
) -> Vec<FormulaReportRecord> {
    candidates
        .iter()
        .map(|candidate| {
            let review_block = is_unresolved_formula_review(candidate);
            let decision = formula_emission_decision(candidate, mode, render_math, emit_mode);
            let emitted = !review_block && decision.emit;
            let latex = if emitted {
                Some(build_formula_latex(candidate))
            } else {
                candidate.latex.clone()
            };

            let sanity = if candidate.latex.is_some()
                && matches!(candidate.status, FormulaStatus::BackendRecovered)
            {
                Some("passed".to_string())
            } else {
                candidate.sidecar.sanity.clone()
            };

            FormulaReportRecord {
                page: candidate.page_num + 1,
                formula_index: candidate.formula_index + 1,
                confidence: candidate.confidence,
                status: candidate.status.clone(),
                backend: candidate.backend.clone(),
                emitted,
                review_block,
                equation_number: candidate.equation_number.clone(),
                crop_path: candidate.crop_path.clone(),
                source_text: candidate.source_text.clone(),
                latex,
                sidecar: candidate.sidecar.clone(),
                sanity,
                emission_reason: if review_block {
                    "review-block".to_string()
                } else {
                    decision.reason
                },
                reason: candidate.reason.clone(),
            }
        })
        .collect()
}

fn formula_candidates_to_blocks(
    page_num: usize,
    candidates: Vec<FormulaCandidate>,
    mode: cli::FormulaMode,
    render_math: bool,
    emit_mode: FormulaEmitMode,
) -> Vec<Block> {
    if matches!(mode, cli::FormulaMode::Off) {
        return Vec::new();
    }

    candidates
        .into_iter()
        .enumerate()
        .filter_map(|(idx, candidate)| {
            if is_unresolved_formula_review(&candidate) {
                return Some(Block::special(
                    3_100_000 + idx,
                    candidate.bbox,
                    BlockKind::FormulaReview {
                        reason: candidate.reason,
                        crop_path: candidate.crop_path,
                    },
                    page_num,
                    0.0,
                    "formula-review".to_string(),
                ));
            }

            if !formula_emission_decision(&candidate, mode, render_math, emit_mode).emit {
                return None;
            }

            let latex = build_formula_latex(&candidate);
            Some(Block {
                override_markdown: None,
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

#[derive(Debug, Clone)]
struct FormulaEmissionDecision {
    emit: bool,
    reason: String,
}

fn formula_emission_decision(
    candidate: &FormulaCandidate,
    mode: cli::FormulaMode,
    render_math: bool,
    emit_mode: FormulaEmitMode,
) -> FormulaEmissionDecision {
    if !render_math {
        return reject_formula("render-math-disabled");
    }
    if matches!(mode, cli::FormulaMode::Off) || matches!(emit_mode, FormulaEmitMode::None) {
        return reject_formula("formula-emission-disabled");
    }
    if candidate.latex.is_none() && candidate.source_text.trim().is_empty() {
        return reject_formula("empty-formula-text");
    }
    if has_replacement_or_gibberish_markers(candidate) {
        return reject_formula("unsafe-source-text");
    }
    if candidate.latex.is_some() {
        return emit_formula("sidecar-recovered");
    }

    match emit_mode {
        FormulaEmitMode::None => reject_formula("formula-emission-disabled"),
        FormulaEmitMode::Conservative => {
            reject_formula("local-heuristic-rejected-by-conservative-policy")
        }
        FormulaEmitMode::Auto => match mode {
            cli::FormulaMode::Auto if candidate.confidence >= 70 => {
                emit_formula("high-confidence-local-auto")
            }
            cli::FormulaMode::Local | cli::FormulaMode::Hybrid => emit_formula("local-mode"),
            cli::FormulaMode::Auto => reject_formula("low-confidence-local-auto"),
            cli::FormulaMode::Off => reject_formula("formula-mode-off"),
        },
        FormulaEmitMode::All => emit_formula("formula-emit-all"),
    }
}

fn emit_formula(reason: &str) -> FormulaEmissionDecision {
    FormulaEmissionDecision {
        emit: true,
        reason: reason.to_string(),
    }
}

fn reject_formula(reason: &str) -> FormulaEmissionDecision {
    FormulaEmissionDecision {
        emit: false,
        reason: reason.to_string(),
    }
}

fn has_replacement_or_gibberish_markers(candidate: &FormulaCandidate) -> bool {
    let text = candidate.source_text.trim();
    text.contains('�')
        || text.matches('?').count() >= 3
        || looks_like_malformed_matrix_fragment(text)
}

fn looks_like_malformed_matrix_fragment(text: &str) -> bool {
    let open = text.matches('(').count();
    let close = text.matches(')').count();
    let whitespace_runs = text.split_whitespace().count();
    open >= 2 && close >= 2 && whitespace_runs >= 5 && !text.contains('[') && !text.contains('\\')
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
        if !candidate.words.is_empty() {
            geometric_latex(&candidate.words, &text, unicode_to_latex)
        } else {
            unicode_to_latex(&text)
        }
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
        evidence: &'a crate::layout::table::TableEvidence,
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
            evidence: &candidate.evidence,
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

fn write_formula_index(
    output_dir: &Path,
    source_pdf: &Path,
    page_count: usize,
    candidates: &[FormulaReportRecord],
) -> anyhow::Result<()> {
    let debug_dir = output_dir.join("debug").join("formulas");
    std::fs::create_dir_all(&debug_dir)
        .with_context(|| format!("Failed to create formula debug dir {}", debug_dir.display()))?;

    let pages_with_candidates = candidates
        .iter()
        .map(|candidate| candidate.page)
        .collect::<std::collections::BTreeSet<_>>()
        .len();
    let local_candidate_count = candidates
        .iter()
        .filter(|candidate| matches!(candidate.status, FormulaStatus::LocalCandidate))
        .count();
    let needs_review_count = candidates
        .iter()
        .filter(|candidate| matches!(candidate.status, FormulaStatus::NeedsReview))
        .count();
    let backend_recovered_count = candidates
        .iter()
        .filter(|candidate| matches!(candidate.status, FormulaStatus::BackendRecovered))
        .count();
    let emitted_count = candidates
        .iter()
        .filter(|candidate| candidate.emitted)
        .count();
    let review_block_count = candidates
        .iter()
        .filter(|candidate| candidate.review_block)
        .count();

    let index = FormulaDebugIndex {
        schema_version: 1,
        source_pdf: source_pdf.display().to_string(),
        page_count,
        candidate_count: candidates.len(),
        pages_with_candidates,
        local_candidate_count,
        needs_review_count,
        backend_recovered_count,
        emitted_count,
        review_block_count,
        candidates,
    };

    let path = debug_dir.join("index.json");
    let json = serde_json::to_string_pretty(&index)?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write formula index {}", path.display()))
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
                        let attempt = sidecar.recognize(&abs_path);
                        if matches!(attempt.status, FormulaSidecarStatus::Recovered) {
                            let latex = attempt.latex.as_deref().unwrap_or("");
                            if recovered_latex_is_sane(latex, candidate) {
                                candidate.latex = attempt.latex.clone();
                                candidate.status = FormulaStatus::BackendRecovered;
                                candidate.backend = attempt.backend.clone();
                                candidate.sidecar.sanity = Some("passed".into());
                            } else {
                                candidate.sidecar.sanity = Some("rejected:bad-output".into());
                            }
                        }
                        candidate.sidecar = attempt;
                    } else {
                        let reason = formula_sidecar_rejection_reason(candidate)
                            .unwrap_or("candidate rejected by sidecar policy");
                        candidate.sidecar = FormulaSidecarAttempt::rejected_by_policy(reason);
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
    formula_sidecar_rejection_reason(candidate).is_none()
}

fn formula_sidecar_rejection_reason(candidate: &FormulaCandidate) -> Option<&'static str> {
    if candidate.confidence < 65 {
        return Some("candidate below sidecar confidence threshold");
    }

    if is_visual_only_formula_candidate(candidate) {
        if visual_candidate_is_ocr_friendly(candidate) {
            return None;
        }
        return Some("visual-only crop too wide or ambiguous for sidecar OCR");
    }

    let text = candidate.source_text.trim();
    if text.is_empty() {
        return None;
    }
    if text.contains("<EOS>") || text.contains("<pad>") {
        return Some("candidate contains model-special prose tokens");
    }
    if candidate.equation_number.is_some() {
        return None;
    }

    let word_count = text.split_whitespace().count();
    let relation_count = text
        .chars()
        .filter(|c| matches!(c, '=' | '<' | '>' | '≤' | '≥'))
        .count();
    if relation_count == 0 {
        return Some("candidate has no formula relation operator");
    }
    if word_count > 14 {
        return Some("candidate has too many words for sidecar OCR");
    }
    if looks_like_definition_line(text) {
        return Some("candidate looks like a variable definition line");
    }
    if looks_like_table_range_comparison(text, relation_count) {
        return Some("candidate looks like a table/range comparison");
    }
    if looks_like_standards_table_or_prose_line(text) {
        return Some("candidate looks like standards table/prose content");
    }

    let stopword_count = text
        .split(|c: char| !c.is_ascii_alphabetic())
        .filter(|word| {
            matches!(
                word.to_ascii_lowercase().as_str(),
                "the"
                    | "and"
                    | "or"
                    | "of"
                    | "to"
                    | "in"
                    | "we"
                    | "with"
                    | "for"
                    | "by"
                    | "is"
                    | "are"
                    | "this"
                    | "that"
                    | "as"
                    | "on"
                    | "from"
                    | "at"
                    | "be"
                    | "all"
                    | "used"
                    | "using"
                    | "have"
                    | "has"
            )
        })
        .count();
    if word_count >= 10 && stopword_count >= 3 && relation_count <= 1 {
        return Some("candidate looks like prose with an inline relation");
    }

    let math_score = text
        .chars()
        .filter(|c| {
            matches!(
                c,
                '=' | '+'
                    | '−'
                    | '-'
                    | '×'
                    | '*'
                    | '/'
                    | '÷'
                    | '<'
                    | '>'
                    | '≤'
                    | '≥'
                    | '√'
                    | '∑'
                    | '∫'
                    | '∂'
                    | '∆'
                    | 'Δ'
                    | 'π'
                    | 'μ'
                    | 'σ'
                    | 'τ'
                    | 'γ'
                    | 'α'
                    | 'β'
                    | 'θ'
                    | 'λ'
                    | 'φ'
                    | 'Ω'
                    | '^'
                    | '_'
            )
        })
        .count();

    if relation_count >= 2 || math_score >= 3 || (candidate.confidence >= 85 && word_count <= 10) {
        None
    } else {
        Some("candidate below sidecar math-content threshold")
    }
}

fn is_visual_only_formula_candidate(candidate: &FormulaCandidate) -> bool {
    candidate.source_text.trim().is_empty()
        || candidate.backend.as_deref() == Some("visual-page-render")
        || candidate.reason.contains("visual-isolated-equation-band")
}

fn visual_candidate_is_ocr_friendly(candidate: &FormulaCandidate) -> bool {
    let width = candidate.bbox.x1 - candidate.bbox.x0;
    let height = candidate.bbox.y1 - candidate.bbox.y0;
    if width <= 0.0 || height <= 0.0 {
        return false;
    }

    let aspect_ratio = width / height;
    if aspect_ratio > 24.0 {
        return false;
    }

    let broad_horizontal_band = width > 420.0
        && (candidate.source_text.trim().is_empty()
            || candidate.reason.contains("horizontal-rule"));
    !broad_horizontal_band
}

fn looks_like_definition_line(text: &str) -> bool {
    let Some((_, rhs)) = text.split_once('=') else {
        return false;
    };
    let rhs_word_count = rhs.split_whitespace().count();
    if rhs_word_count < 4 {
        return false;
    }
    let rhs_lower = rhs.to_ascii_lowercase();
    let definition_terms = [
        "angle",
        "factor",
        "including",
        "margin",
        "object",
        "plates",
        "sling",
        "table",
        "thickness",
        "weight",
    ];
    definition_terms.iter().any(|term| rhs_lower.contains(term))
}

fn looks_like_table_range_comparison(text: &str, relation_count: usize) -> bool {
    !text.contains('=') && relation_count >= 2 && text.split_whitespace().count() >= 4
}

fn recovered_latex_is_sane(latex: &str, candidate: &FormulaCandidate) -> bool {
    if latex.is_empty() {
        return false;
    }

    let height = (candidate.bbox.y1 - candidate.bbox.y0).max(1.0);
    let width = (candidate.bbox.x1 - candidate.bbox.x0).max(1.0);
    let _expected_chars = (height * width / 100.0) as usize;

    // 1. Excessive backslash density: more than ~8 per height-point
    let backslash_count = latex.matches('\\').count();
    if backslash_count > (height as usize) * 8 && latex.len() > 50 {
        return false;
    }

    // 2. Repeated delimiter noise patterns
    let repeated_left = latex.matches("\\left|").count() + latex.matches("\\left\\|").count();
    let _repeated_array = latex.matches("\\begin{array}").count();
    if repeated_left > 3 && latex.len() > 100 {
        return false;
    }

    // 3. Overlong LaTeX for a small crop (more than 50 chars per point of height)
    if latex.len() > (height as usize) * 50 && width < 500.0 {
        return false;
    }

    // 4. Text-heavy recovered LaTeX with long English words not in source
    if !candidate.source_text.trim().is_empty() {
        let source_lower = candidate.source_text.to_ascii_lowercase();
        let latex_lower = latex.to_ascii_lowercase();
        let long_words_in_latex: Vec<&str> = latex_lower
            .split_whitespace()
            .filter(|w| w.len() > 8 && w.chars().all(|c| c.is_ascii_alphabetic()))
            .collect();
        let unexpected_long_words = long_words_in_latex
            .iter()
            .filter(|w| !source_lower.contains(*w))
            .count();
        if unexpected_long_words >= 3 {
            return false;
        }
    }

    // 5. Excessive `\\stackrel`/`\\overset` stacking
    let stack_count = latex.matches("\\stackrel").count()
        + latex.matches("\\overset").count()
        + latex.matches("\\widetilde").count()
        + latex.matches("\\widehat").count();
    if stack_count > 5 && source_text_length(candidate) < 30 {
        return false;
    }

    true
}

fn source_text_length(candidate: &FormulaCandidate) -> usize {
    candidate.source_text.trim().len()
}

fn looks_like_standards_table_or_prose_line(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    let phrase_terms = [
        "defined as",
        "environmental criteria",
        "heading in degrees",
        "linear wave theory",
        "minimum required",
        "minimum tipping angle",
        "not applicable",
        "operational criteria",
        "seafastening force",
        "significant wave",
        "solitary wave theory",
        "upper bound",
        "wave period",
    ];
    if phrase_terms.iter().any(|term| lower.contains(term)) {
        return true;
    }

    let high_risk_single_terms = [
        "acceleration",
        "equation",
        "month",
        "n/a",
        "return",
        "see [",
        "tonnes",
        "year",
    ];
    if high_risk_single_terms
        .iter()
        .any(|term| lower.contains(term))
    {
        return true;
    }

    let table_terms = [
        "any",
        "barge",
        "bridles",
        "cargo",
        "category",
        "criteria",
        "days",
        "derate",
        "during",
        "equipment",
        "height",
        "hence",
        "lashing",
        "links",
        "load",
        "objects",
        "pennants",
        "plates",
        "shackles",
        "sockets",
        "smys",
        "towlines",
        "upend",
        "vessels",
        "visual",
    ];
    let matches = table_terms
        .iter()
        .filter(|term| lower.contains(*term))
        .count();
    matches >= 2
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
        blocks.push(Block::special(
            1_000_000 + img_ref.image_index,
            img_ref.bbox,
            BlockKind::Image {
                path: Some(rel_path),
            },
            img_ref.page_num,
            0.0,
            "image".to_string(),
        ));
    }
    Ok(blocks)
}

fn write_document(doc: &Document, input_path: &Path, args: &ConvertArgs) -> anyhow::Result<()> {
    let output_dir = batch::output_dir_for(input_path, args.options.output.as_deref());
    let renderer = MarkdownRenderer::with_style(
        !args.options.no_images,
        Some(output_dir.join("images")),
        args.options.effective_markdown_style(),
    );
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
    use crate::formula::detect::{FormulaCandidate, FormulaStatus};

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
            words: Vec::new(),
            sidecar: FormulaSidecarAttempt::not_attempted(),
            reason: "test".into(),
            crop_path: None,
        }
    }

    #[test]
    fn auto_mode_promotes_only_high_confidence_formula_candidates() {
        let high = formula_candidate("E = mc^2", 70);
        let low = formula_candidate("a + b", 69);

        let blocks = formula_candidates_to_blocks(
            0,
            vec![high, low],
            FormulaMode::Auto,
            true,
            FormulaEmitMode::Auto,
        );

        assert_eq!(blocks.len(), 1);
        assert!(matches!(blocks[0].kind, BlockKind::Formula { .. }));
        assert_eq!(blocks[0].text, "E = mc^2");
    }

    #[test]
    fn auto_policy_rejects_replacement_character_formula_text() {
        let candidate = formula_candidate("E = � + ???", 90);

        let blocks = formula_candidates_to_blocks(
            0,
            vec![candidate],
            FormulaMode::Auto,
            true,
            FormulaEmitMode::Auto,
        );

        assert!(
            blocks.is_empty(),
            "unsafe local formula text should not emit"
        );
    }

    #[test]
    fn auto_policy_rejects_malformed_matrix_like_fragments() {
        let candidate = formula_candidate("(1 2 4)(𝑥 𝑦) = ( 11) 5", 90);

        let blocks = formula_candidates_to_blocks(
            0,
            vec![candidate],
            FormulaMode::Auto,
            true,
            FormulaEmitMode::Auto,
        );

        assert!(
            blocks.is_empty(),
            "malformed matrix fragments should not emit"
        );
    }

    #[test]
    fn conservative_policy_rejects_local_heuristic_formula_text() {
        let candidate = formula_candidate("E = mc^2", 90);

        let blocks = formula_candidates_to_blocks(
            0,
            vec![candidate],
            FormulaMode::Auto,
            true,
            FormulaEmitMode::Conservative,
        );

        assert!(
            blocks.is_empty(),
            "conservative policy requires recovered LaTeX"
        );
    }

    #[test]
    fn sidecar_policy_skips_prose_like_high_confidence_candidates() {
        let candidate = formula_candidate(
            "We used the Adam optimizer with beta1 = 0.9 and beta2 = 0.98 during training",
            85,
        );

        assert!(!should_send_to_formula_sidecar(&candidate));
        assert_eq!(
            formula_sidecar_rejection_reason(&candidate),
            Some("candidate has too many words for sidecar OCR")
        );
    }

    #[test]
    fn sidecar_policy_skips_variable_definition_lines() {
        let candidate = formula_candidate(
            "γWeight = Unweighed object weight margin factor as per Table 5-2",
            77,
        );

        assert!(!should_send_to_formula_sidecar(&candidate));
        assert_eq!(
            formula_sidecar_rejection_reason(&candidate),
            Some("candidate looks like a variable definition line")
        );
    }

    #[test]
    fn sidecar_policy_skips_standards_table_and_prose_lines() {
        let table = formula_candidate(
            "Delta plates, master links and shackles <5 years < 12 months",
            73,
        );
        let prose = formula_candidate("h/λ > 0.3 Linear wave theory (or Stokes 5th order)", 77);
        let range = formula_candidate("1000t ≤ W 5000t ≤ W 20000t ≤", 77);

        assert!(!should_send_to_formula_sidecar(&table));
        assert_eq!(
            formula_sidecar_rejection_reason(&table),
            Some("candidate looks like a table/range comparison")
        );
        assert!(!should_send_to_formula_sidecar(&prose));
        assert_eq!(
            formula_sidecar_rejection_reason(&prose),
            Some("candidate looks like standards table/prose content")
        );
        assert!(!should_send_to_formula_sidecar(&range));
        assert_eq!(
            formula_sidecar_rejection_reason(&range),
            Some("candidate looks like a table/range comparison")
        );
    }

    #[test]
    fn sidecar_policy_keeps_compact_standards_formulas() {
        let weight = formula_candidate("WReport, Factored ≤ Wud/γWeight", 87);
        let padeye = formula_candidate("Rpad= (Rpl × tpl+2 × Rch × tch)/t", 85);

        assert!(should_send_to_formula_sidecar(&weight));
        assert!(should_send_to_formula_sidecar(&padeye));
    }

    #[test]
    fn sidecar_policy_keeps_numbered_and_reasonable_visual_formula_candidates() {
        let mut numbered = formula_candidate("E = mc^2 (1)", 96);
        numbered.equation_number = Some("(1)".into());
        let mut visual = formula_candidate("", 68);
        visual.bbox = Bbox::new(120.0, 140.0, 420.0, 162.0);
        visual.backend = Some("visual-page-render".into());
        visual.reason = "visual-isolated-equation-band+centered".into();

        assert!(should_send_to_formula_sidecar(&numbered));
        assert!(should_send_to_formula_sidecar(&visual));
    }

    #[test]
    fn sidecar_policy_rejects_wide_visual_formula_candidates() {
        let mut visual = formula_candidate("", 68);
        visual.bbox = Bbox::new(20.0, 140.0, 510.0, 170.0);
        visual.backend = Some("visual-page-render".into());
        visual.reason = "visual-isolated-equation-band+centered+horizontal-rule".into();

        assert!(!should_send_to_formula_sidecar(&visual));
        assert_eq!(
            formula_sidecar_rejection_reason(&visual),
            Some("visual-only crop too wide or ambiguous for sidecar OCR")
        );
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
    fn visual_only_candidate_becomes_review_block_without_math_rendering() {
        let mut candidate = formula_candidate("", 68);
        candidate.backend = Some("visual-page-render".into());
        candidate.reason = "visual-isolated-equation-band+cue:Hence:".into();
        candidate.crop_path = Some("debug/formulas/page1_formula1.png".into());

        let blocks = formula_candidates_to_blocks(
            0,
            vec![candidate],
            FormulaMode::Auto,
            false,
            FormulaEmitMode::Auto,
        );

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
