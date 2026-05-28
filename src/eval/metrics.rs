use crate::document::types::{Bbox, BlockKind, Page};
use crate::eval::fixtures::PageExpectation;

const TABLE_REGION_IOU_THRESHOLD: f32 = 0.50;

#[derive(Debug, Clone, Default)]
pub struct PageMetrics {
    pub formula_found: usize,
    pub formula_expected: usize,
    pub formula_matches: usize,
    pub formula_false_positives: usize,
    pub formula_recall: f32,
    pub formula_precision: f32,
    pub formula_detection_found: usize,
    pub formula_detection_expected: usize,
    pub formula_detection_matches: usize,
    pub formula_detection_false_positives: usize,
    pub formula_detection_recall: f32,
    pub formula_detection_precision: f32,
    pub formula_latex_snippets_expected: usize,
    pub formula_latex_snippets_matched: usize,
    pub formula_latex_snippet_recall: f32,
    pub heading_found: usize,
    pub heading_matches: usize,
    pub heading_expected: usize,
    pub heading_false_positives: usize,
    pub heading_accuracy: f32,
    pub heading_precision: f32,
    pub table_found: bool,
    pub table_expected: bool,
    pub table_true_positive: bool,
    pub table_false_positive: bool,
    pub table_regions_found: usize,
    pub table_regions_expected: usize,
    pub table_region_matches: usize,
    pub table_region_false_positives: usize,
    pub table_region_false_negatives: usize,
    pub table_region_recall: f32,
    pub table_region_precision: f32,
    pub decorative_images_expected: usize,
    pub decorative_images_emitted: usize,
    pub decorative_images_suppressed: usize,
    pub decorative_image_suppression_rate: f32,
    pub meaningful_figures_found: usize,
    pub meaningful_figures_expected: usize,
    pub meaningful_figure_matches: usize,
    pub meaningful_figure_retention_rate: f32,
    pub figure_caption_pairs_found: usize,
    pub figure_caption_pairs_expected: usize,
    pub figure_caption_pairing_rate: f32,
    pub vector_only_regions_expected: usize,
    pub vector_only_regions_acknowledged: usize,
    pub vector_only_region_acknowledgement_rate: f32,
}

#[derive(Debug, Clone, Default)]
pub struct DocMetrics {
    pub pages: usize,
    pub formula_found: usize,
    pub formula_expected: usize,
    pub formula_matches: usize,
    pub formula_false_positives: usize,
    pub formula_page_recall_sum: f32,
    pub formula_page_precision_sum: f32,
    pub formula_detection_found: usize,
    pub formula_detection_expected: usize,
    pub formula_detection_matches: usize,
    pub formula_detection_false_positives: usize,
    pub formula_detection_page_recall_sum: f32,
    pub formula_detection_page_precision_sum: f32,
    pub formula_latex_snippets_expected: usize,
    pub formula_latex_snippets_matched: usize,
    pub formula_latex_snippet_page_recall_sum: f32,
    pub heading_found: usize,
    pub heading_matches: usize,
    pub heading_expected: usize,
    pub heading_false_positives: usize,
    pub heading_page_accuracy_sum: f32,
    pub heading_page_precision_sum: f32,
    pub table_pages_found: usize,
    pub table_pages_expected: usize,
    pub table_pages_true_positive: usize,
    pub table_pages_false_positive: usize,
    pub table_regions_found: usize,
    pub table_regions_expected: usize,
    pub table_region_matches: usize,
    pub table_region_false_positives: usize,
    pub table_region_false_negatives: usize,
    pub table_region_page_recall_sum: f32,
    pub table_region_page_precision_sum: f32,
    pub decorative_images_expected: usize,
    pub decorative_images_emitted: usize,
    pub decorative_images_suppressed: usize,
    pub meaningful_figures_found: usize,
    pub meaningful_figures_expected: usize,
    pub meaningful_figure_matches: usize,
    pub figure_caption_pairs_found: usize,
    pub figure_caption_pairs_expected: usize,
    pub vector_only_regions_expected: usize,
    pub vector_only_regions_acknowledged: usize,
    pub decorative_image_page_suppression_sum: f32,
    pub meaningful_figure_page_retention_sum: f32,
    pub figure_caption_page_pairing_sum: f32,
    pub vector_only_page_acknowledgement_sum: f32,
}

impl DocMetrics {
    pub fn formula_recall(&self) -> f32 {
        ratio_or_full(self.formula_matches, self.formula_expected)
    }

    pub fn formula_precision(&self) -> f32 {
        ratio_or_full(self.formula_matches, self.formula_found)
    }

    pub fn formula_detection_recall(&self) -> f32 {
        ratio_or_full(
            self.formula_detection_matches,
            self.formula_detection_expected,
        )
    }

    pub fn formula_detection_precision(&self) -> f32 {
        ratio_or_full(self.formula_detection_matches, self.formula_detection_found)
    }

    pub fn formula_latex_snippet_recall(&self) -> f32 {
        ratio_or_full(
            self.formula_latex_snippets_matched,
            self.formula_latex_snippets_expected,
        )
    }

    pub fn heading_accuracy(&self) -> f32 {
        ratio_or_full(self.heading_matches, self.heading_expected)
    }

    pub fn heading_precision(&self) -> f32 {
        ratio_or_full(self.heading_matches, self.heading_found)
    }

    pub fn table_recall(&self) -> f32 {
        ratio_or_full(self.table_pages_true_positive, self.table_pages_expected)
    }

    pub fn table_precision(&self) -> f32 {
        ratio_or_full(self.table_pages_true_positive, self.table_pages_found)
    }

    pub fn table_region_recall(&self) -> f32 {
        ratio_or_full(self.table_region_matches, self.table_regions_expected)
    }

    pub fn table_region_precision(&self) -> f32 {
        ratio_or_full(self.table_region_matches, self.table_regions_found)
    }

    pub fn decorative_image_suppression_rate(&self) -> f32 {
        ratio_or_full(
            self.decorative_images_suppressed,
            self.decorative_images_expected,
        )
    }

    pub fn meaningful_figure_retention_rate(&self) -> f32 {
        ratio_or_full(
            self.meaningful_figure_matches,
            self.meaningful_figures_expected,
        )
    }

    pub fn figure_caption_pairing_rate(&self) -> f32 {
        ratio_or_full(
            self.figure_caption_pairs_found,
            self.figure_caption_pairs_expected,
        )
    }

    pub fn vector_only_region_acknowledgement_rate(&self) -> f32 {
        ratio_or_full(
            self.vector_only_regions_acknowledged,
            self.vector_only_regions_expected,
        )
    }
}

pub fn compute_page_metrics(page: &Page, expected: &PageExpectation) -> PageMetrics {
    let formula_found = if expected.skip_text_metrics {
        0
    } else {
        page.blocks
            .iter()
            .filter(|block| {
                matches!(
                    block.kind,
                    BlockKind::Formula { .. } | BlockKind::FormulaReview { .. }
                )
            })
            .count()
    };
    let formula_expected = expected.expected_formula_count;
    let formula_matches = formula_found.min(formula_expected);
    let formula_false_positives = formula_found.saturating_sub(formula_matches);

    let actual_headings: Vec<_> = if expected.skip_text_metrics {
        Vec::new()
    } else {
        page.blocks
            .iter()
            .filter_map(|block| match block.kind {
                BlockKind::Heading { level } => Some((level, block.text.trim())),
                _ => None,
            })
            .collect()
    };
    let heading_matches = expected
        .expected_headings
        .iter()
        .filter(|heading| {
            actual_headings.iter().any(|(level, text)| {
                *level == heading.level && text.eq_ignore_ascii_case(heading.text.trim())
            })
        })
        .count();
    let heading_expected = expected.expected_headings.len();
    let heading_found = actual_headings.len();
    let heading_false_positives = heading_found.saturating_sub(heading_matches);

    let actual_table_regions: Vec<Bbox> = if expected.skip_table_metrics {
        Vec::new()
    } else {
        page.blocks
            .iter()
            .filter_map(|block| match &block.kind {
                BlockKind::CoordinateTable { table } => Some(table.bbox),
                _ => None,
            })
            .collect()
    };
    let table_found = !actual_table_regions.is_empty();
    let table_expected = expected.expected_tables > 0;
    let table_true_positive = table_found && table_expected;
    let table_false_positive = table_found && !table_expected;

    let expected_table_regions: Vec<Bbox> = expected
        .expected_table_regions
        .iter()
        .map(|region| region.bbox())
        .collect();
    let region_counts = if expected_table_regions.is_empty() {
        TableRegionCounts::default()
    } else {
        match_table_regions(&actual_table_regions, &expected_table_regions)
    };

    let raw_image_blocks_found = page
        .blocks
        .iter()
        .filter(|block| matches!(block.kind, BlockKind::Image { .. }))
        .count();
    let meaningful_figures_found = page
        .blocks
        .iter()
        .filter(|block| matches!(block.kind, BlockKind::Figure { .. }))
        .count();
    let meaningful_figure_matches =
        meaningful_figures_found.min(expected.expected_meaningful_figures);
    let captioned_figures_found = page
        .blocks
        .iter()
        .filter(|block| match &block.kind {
            BlockKind::Figure {
                caption: Some(caption),
                ..
            } => !caption.trim().is_empty(),
            _ => false,
        })
        .count();
    let figure_caption_pairs_found = captioned_figures_found.min(expected.expected_figure_captions);
    let uncaptioned_figures_found =
        meaningful_figures_found.saturating_sub(captioned_figures_found);
    let remaining_meaningful_slots = expected
        .expected_meaningful_figures
        .saturating_sub(captioned_figures_found);
    let decorative_figure_leakage =
        uncaptioned_figures_found.saturating_sub(remaining_meaningful_slots);
    let decorative_images_emitted = (raw_image_blocks_found + decorative_figure_leakage)
        .min(expected.expected_decorative_images);
    let decorative_images_suppressed = expected
        .expected_decorative_images
        .saturating_sub(decorative_images_emitted);
    let vector_only_regions_acknowledged =
        meaningful_figures_found.min(expected.expected_vector_only_regions);

    PageMetrics {
        formula_found,
        formula_expected,
        formula_matches,
        formula_false_positives,
        formula_recall: ratio_or_full(formula_matches, formula_expected),
        formula_precision: ratio_or_full(formula_matches, formula_found),
        formula_detection_found: formula_found,
        formula_detection_expected: formula_expected,
        formula_detection_matches: formula_matches,
        formula_detection_false_positives: formula_false_positives,
        formula_detection_recall: ratio_or_full(formula_matches, formula_expected),
        formula_detection_precision: ratio_or_full(formula_matches, formula_found),
        formula_latex_snippets_expected: expected.expected_formula_latex_snippets.len(),
        formula_latex_snippets_matched: 0,
        formula_latex_snippet_recall: ratio_or_full(
            0,
            expected.expected_formula_latex_snippets.len(),
        ),
        heading_found,
        heading_matches,
        heading_expected,
        heading_false_positives,
        heading_accuracy: ratio_or_full(heading_matches, heading_expected),
        heading_precision: ratio_or_full(heading_matches, heading_found),
        table_found,
        table_expected,
        table_true_positive,
        table_false_positive,
        table_regions_found: region_counts.found,
        table_regions_expected: region_counts.expected,
        table_region_matches: region_counts.matches,
        table_region_false_positives: region_counts.false_positives,
        table_region_false_negatives: region_counts.false_negatives,
        table_region_recall: ratio_or_full(region_counts.matches, region_counts.expected),
        table_region_precision: ratio_or_full(region_counts.matches, region_counts.found),
        decorative_images_expected: expected.expected_decorative_images,
        decorative_images_emitted,
        decorative_images_suppressed,
        decorative_image_suppression_rate: ratio_or_full(
            decorative_images_suppressed,
            expected.expected_decorative_images,
        ),
        meaningful_figures_found,
        meaningful_figures_expected: expected.expected_meaningful_figures,
        meaningful_figure_matches,
        meaningful_figure_retention_rate: ratio_or_full(
            meaningful_figure_matches,
            expected.expected_meaningful_figures,
        ),
        figure_caption_pairs_found,
        figure_caption_pairs_expected: expected.expected_figure_captions,
        figure_caption_pairing_rate: ratio_or_full(
            figure_caption_pairs_found,
            expected.expected_figure_captions,
        ),
        vector_only_regions_expected: expected.expected_vector_only_regions,
        vector_only_regions_acknowledged,
        vector_only_region_acknowledgement_rate: ratio_or_full(
            vector_only_regions_acknowledged,
            expected.expected_vector_only_regions,
        ),
    }
}

#[derive(Debug, Clone, Default)]
pub struct FormulaDebugPageMetrics {
    pub candidates: usize,
    pub emitted: usize,
    pub latex_values: Vec<String>,
}

pub fn apply_formula_debug_metrics(
    metrics: &mut PageMetrics,
    expected: &PageExpectation,
    debug: &FormulaDebugPageMetrics,
) {
    if let Some(expected_detection) = expected.expected_formula_detection_count {
        metrics.formula_detection_found = debug.candidates;
        metrics.formula_detection_expected = expected_detection;
        metrics.formula_detection_matches = debug.candidates.min(expected_detection);
        let budgeted_expected = expected_detection + expected.formula_false_positive_budget;
        metrics.formula_detection_false_positives =
            debug.candidates.saturating_sub(budgeted_expected);
        metrics.formula_detection_recall = ratio_or_full(
            metrics.formula_detection_matches,
            metrics.formula_detection_expected,
        );
        metrics.formula_detection_precision =
            if metrics.formula_detection_found <= budgeted_expected {
                1.0
            } else {
                ratio_or_full(
                    metrics.formula_detection_matches,
                    metrics.formula_detection_found,
                )
            };
    }

    if !expected.expected_formula_latex_snippets.is_empty() {
        metrics.formula_latex_snippets_expected = expected.expected_formula_latex_snippets.len();
        metrics.formula_latex_snippets_matched = expected
            .expected_formula_latex_snippets
            .iter()
            .filter(|snippet| {
                debug
                    .latex_values
                    .iter()
                    .any(|latex| latex.contains(snippet.as_str()))
            })
            .count();
        metrics.formula_latex_snippet_recall = ratio_or_full(
            metrics.formula_latex_snippets_matched,
            metrics.formula_latex_snippets_expected,
        );
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct TableRegionCounts {
    found: usize,
    expected: usize,
    matches: usize,
    false_positives: usize,
    false_negatives: usize,
}

fn match_table_regions(actual: &[Bbox], expected: &[Bbox]) -> TableRegionCounts {
    let mut pairs = Vec::new();
    for (actual_idx, actual_bbox) in actual.iter().enumerate() {
        for (expected_idx, expected_bbox) in expected.iter().enumerate() {
            let iou = bbox_iou(*actual_bbox, *expected_bbox);
            if iou >= TABLE_REGION_IOU_THRESHOLD {
                pairs.push((actual_idx, expected_idx, iou));
            }
        }
    }
    pairs.sort_by(|left, right| {
        right
            .2
            .partial_cmp(&left.2)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut matched_actual = vec![false; actual.len()];
    let mut matched_expected = vec![false; expected.len()];
    let mut matches = 0usize;
    for (actual_idx, expected_idx, _) in pairs {
        if matched_actual[actual_idx] || matched_expected[expected_idx] {
            continue;
        }
        matched_actual[actual_idx] = true;
        matched_expected[expected_idx] = true;
        matches += 1;
    }

    TableRegionCounts {
        found: actual.len(),
        expected: expected.len(),
        matches,
        false_positives: actual.len().saturating_sub(matches),
        false_negatives: expected.len().saturating_sub(matches),
    }
}

fn bbox_iou(a: Bbox, b: Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    let intersection = ((x1 - x0).max(0.0)) * ((y1 - y0).max(0.0));
    if intersection <= 0.0 {
        return 0.0;
    }
    let union = a.area() + b.area() - intersection;
    intersection / union.max(1.0)
}

pub fn aggregate(page_metrics: &[PageMetrics]) -> DocMetrics {
    let mut doc = DocMetrics {
        pages: page_metrics.len(),
        ..Default::default()
    };
    for metrics in page_metrics {
        doc.formula_found += metrics.formula_found;
        doc.formula_matches += metrics.formula_matches;
        doc.formula_expected += metrics.formula_expected;
        doc.formula_false_positives += metrics.formula_false_positives;
        doc.formula_page_recall_sum += metrics.formula_recall;
        doc.formula_page_precision_sum += metrics.formula_precision;
        doc.formula_detection_found += metrics.formula_detection_found;
        doc.formula_detection_expected += metrics.formula_detection_expected;
        doc.formula_detection_matches += metrics.formula_detection_matches;
        doc.formula_detection_false_positives += metrics.formula_detection_false_positives;
        doc.formula_detection_page_recall_sum += metrics.formula_detection_recall;
        doc.formula_detection_page_precision_sum += metrics.formula_detection_precision;
        doc.formula_latex_snippets_expected += metrics.formula_latex_snippets_expected;
        doc.formula_latex_snippets_matched += metrics.formula_latex_snippets_matched;
        doc.formula_latex_snippet_page_recall_sum += metrics.formula_latex_snippet_recall;
        doc.heading_found += metrics.heading_found;
        doc.heading_matches += metrics.heading_matches.min(metrics.heading_expected);
        doc.heading_expected += metrics.heading_expected;
        doc.heading_false_positives += metrics.heading_false_positives;
        doc.heading_page_accuracy_sum += metrics.heading_accuracy;
        doc.heading_page_precision_sum += metrics.heading_precision;
        if metrics.table_found {
            doc.table_pages_found += 1;
        }
        if metrics.table_expected {
            doc.table_pages_expected += 1;
        }
        if metrics.table_true_positive {
            doc.table_pages_true_positive += 1;
        }
        if metrics.table_false_positive {
            doc.table_pages_false_positive += 1;
        }
        doc.table_regions_found += metrics.table_regions_found;
        doc.table_regions_expected += metrics.table_regions_expected;
        doc.table_region_matches += metrics.table_region_matches;
        doc.table_region_false_positives += metrics.table_region_false_positives;
        doc.table_region_false_negatives += metrics.table_region_false_negatives;
        doc.table_region_page_recall_sum += metrics.table_region_recall;
        doc.table_region_page_precision_sum += metrics.table_region_precision;
        doc.decorative_images_expected += metrics.decorative_images_expected;
        doc.decorative_images_emitted += metrics.decorative_images_emitted;
        doc.decorative_images_suppressed += metrics.decorative_images_suppressed;
        doc.meaningful_figures_found += metrics.meaningful_figures_found;
        doc.meaningful_figures_expected += metrics.meaningful_figures_expected;
        doc.meaningful_figure_matches += metrics.meaningful_figure_matches;
        doc.figure_caption_pairs_found += metrics.figure_caption_pairs_found;
        doc.figure_caption_pairs_expected += metrics.figure_caption_pairs_expected;
        doc.vector_only_regions_expected += metrics.vector_only_regions_expected;
        doc.vector_only_regions_acknowledged += metrics.vector_only_regions_acknowledged;
        doc.decorative_image_page_suppression_sum += metrics.decorative_image_suppression_rate;
        doc.meaningful_figure_page_retention_sum += metrics.meaningful_figure_retention_rate;
        doc.figure_caption_page_pairing_sum += metrics.figure_caption_pairing_rate;
        doc.vector_only_page_acknowledgement_sum += metrics.vector_only_region_acknowledgement_rate;
    }
    doc
}

pub fn print_report(doc_name: &str, doc: &DocMetrics) {
    println!(
        "{doc_name}\n  pages evaluated:    {}\n  formula recall:     {:.1}% ({}/{})\n  formula precision:  {:.1}% ({}/{}, fp {})\n  heading accuracy:   {:.1}% ({}/{})\n  heading precision:  {:.1}% ({}/{}, fp {})\n  table recall:       {:.1}% ({}/{})\n  table precision:    {:.1}% ({}/{}, fp {})\n",
        doc.pages,
        100.0 * doc.formula_recall(),
        doc.formula_matches,
        doc.formula_expected,
        100.0 * doc.formula_precision(),
        doc.formula_matches,
        doc.formula_found,
        doc.formula_false_positives,
        100.0 * doc.heading_accuracy(),
        doc.heading_matches.min(doc.heading_expected),
        doc.heading_expected,
        100.0 * doc.heading_precision(),
        doc.heading_matches,
        doc.heading_found,
        doc.heading_false_positives,
        100.0 * doc.table_recall(),
        doc.table_pages_true_positive,
        doc.table_pages_expected,
        100.0 * doc.table_precision(),
        doc.table_pages_true_positive,
        doc.table_pages_found,
        doc.table_pages_false_positive,
    );
    if doc.formula_detection_expected > 0 || doc.formula_latex_snippets_expected > 0 {
        println!(
            "  formula detection recall:    {:.1}% ({}/{})\n  formula detection precision: {:.1}% ({}/{}, fp {})\n  formula LaTeX snippet recall: {:.1}% ({}/{})\n",
            100.0 * doc.formula_detection_recall(),
            doc.formula_detection_matches,
            doc.formula_detection_expected,
            100.0 * doc.formula_detection_precision(),
            doc.formula_detection_matches,
            doc.formula_detection_found,
            doc.formula_detection_false_positives,
            100.0 * doc.formula_latex_snippet_recall(),
            doc.formula_latex_snippets_matched,
            doc.formula_latex_snippets_expected,
        );
    }
    if doc.table_regions_expected > 0 {
        println!(
            "  table region recall:    {:.1}% ({}/{})\n  table region precision: {:.1}% ({}/{}, fp {}, fn {})\n",
            100.0 * doc.table_region_recall(),
            doc.table_region_matches,
            doc.table_regions_expected,
            100.0 * doc.table_region_precision(),
            doc.table_region_matches,
            doc.table_regions_found,
            doc.table_region_false_positives,
            doc.table_region_false_negatives,
        );
    }
    if doc.decorative_images_expected
        + doc.meaningful_figures_expected
        + doc.figure_caption_pairs_expected
        + doc.vector_only_regions_expected
        > 0
    {
        println!(
            "  decorative suppression: {:.1}% ({}/{}, emitted {})\n  meaningful figure retention: {:.1}% ({}/{})\n  figure-caption pairing: {:.1}% ({}/{})\n  vector-only acknowledgement: {:.1}% ({}/{})\n",
            100.0 * doc.decorative_image_suppression_rate(),
            doc.decorative_images_suppressed,
            doc.decorative_images_expected,
            doc.decorative_images_emitted,
            100.0 * doc.meaningful_figure_retention_rate(),
            doc.meaningful_figure_matches,
            doc.meaningful_figures_expected,
            100.0 * doc.figure_caption_pairing_rate(),
            doc.figure_caption_pairs_found,
            doc.figure_caption_pairs_expected,
            100.0 * doc.vector_only_region_acknowledgement_rate(),
            doc.vector_only_regions_acknowledged,
            doc.vector_only_regions_expected,
        );
    }
}

fn ratio_or_full(found: usize, expected: usize) -> f32 {
    if expected == 0 {
        1.0
    } else {
        found.min(expected) as f32 / expected as f32
    }
}
