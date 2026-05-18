//! Pure geometry-based helpers used by the PDF pipeline to merge detected
//! regions into a single reading-ordered stream and to suppress overlapping or
//! redundant blocks.
//!
//! Everything here is a pure function over `Block`, `Bbox`, `TableCandidate`,
//! and `FormulaCandidate` — no IO, no mupdf, no globals — so the helpers can
//! be unit-tested in isolation as Stage 8 begins tuning heading/formula and
//! table behaviour.

use std::cmp::Ordering;

use crate::document::types::{Bbox, Block, BlockKind};
use crate::formula::FormulaCandidate;
use crate::layout::table::TableCandidate;

/// Fraction of `a` that overlaps `b`. Used when the question is "is this small
/// thing mostly inside that big thing".
pub(super) fn bbox_overlap_ratio(a: Bbox, b: Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    let intersection = ((x1 - x0).max(0.0)) * ((y1 - y0).max(0.0));
    intersection / a.area().max(1.0)
}

/// Fraction of the smaller bbox covered by the intersection — symmetric, used
/// when neither bbox is privileged (e.g. two table candidates competing for
/// the same region).
pub(super) fn bbox_overlap_smaller(a: Bbox, b: Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    let intersection = ((x1 - x0).max(0.0)) * ((y1 - y0).max(0.0));
    intersection / a.area().min(b.area()).max(1.0)
}

/// Greedy suppression: when candidates claim the same region, keep the best
/// table evidence rather than whichever detector happened to run first.
pub(super) fn suppress_overlapping_table_candidates(
    candidates: Vec<TableCandidate>,
) -> Vec<TableCandidate> {
    let mut kept: Vec<TableCandidate> = Vec::new();
    for candidate in candidates {
        let mut replacement = Some(candidate);
        for existing in &mut kept {
            let candidate_ref = replacement.as_ref().expect("candidate still available");
            if bbox_overlap_smaller(candidate_ref.table.bbox, existing.table.bbox) > 0.65 {
                if table_candidate_cmp(candidate_ref, existing).is_gt() {
                    *existing = replacement.take().expect("candidate still available");
                }
                break;
            }
        }
        if let Some(candidate) = replacement {
            kept.push(candidate);
        }
    }
    kept.sort_by(|left, right| {
        left.table
            .bbox
            .y0
            .partial_cmp(&right.table.bbox.y0)
            .unwrap_or(Ordering::Equal)
            .then_with(|| {
                left.table
                    .bbox
                    .x0
                    .partial_cmp(&right.table.bbox.x0)
                    .unwrap_or(Ordering::Equal)
            })
    });
    kept
}

fn table_candidate_cmp(left: &TableCandidate, right: &TableCandidate) -> Ordering {
    left.ranking_score()
        .partial_cmp(&right.ranking_score())
        .unwrap_or(Ordering::Equal)
        .then_with(|| {
            right
                .table
                .bbox
                .area()
                .partial_cmp(&left.table.bbox.area())
                .unwrap_or(Ordering::Equal)
        })
}

/// Build the bbox list that the formula detector should treat as forbidden
/// regions: detected table interiors plus furniture (running headers/footers).
pub(super) fn formula_excluded_regions(
    tables: &[TableCandidate],
    furniture_bboxes: &[Bbox],
) -> Vec<Bbox> {
    let mut excluded = Vec::with_capacity(tables.len() + furniture_bboxes.len());
    excluded.extend(tables.iter().map(|candidate| candidate.table.bbox));
    excluded.extend_from_slice(furniture_bboxes);
    excluded
}

/// Drop text blocks that are claimed by a detected table — either by id (the
/// block was used to build the table) or by >55% bbox overlap.
pub(super) fn suppress_text_covered_by_tables(
    blocks: Vec<Block>,
    candidates: &[TableCandidate],
) -> Vec<Block> {
    if candidates.is_empty() {
        return blocks;
    }

    blocks
        .into_iter()
        .filter(|block| {
            if matches!(block.kind, BlockKind::Heading { .. }) {
                return true;
            }
            !candidates.iter().any(|candidate| {
                candidate.source_block_ids.contains(&block.id)
                    || bbox_overlap_ratio(block.bbox, candidate.table.bbox) > 0.55
            })
        })
        .collect()
}

/// Drop text blocks that fall inside a furniture region (running headers,
/// footers, watermarks).
pub(super) fn suppress_text_covered_by_furniture(
    blocks: Vec<Block>,
    furniture_bboxes: &[Bbox],
) -> Vec<Block> {
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

/// Drop text blocks that are visually covered by an emitted formula block —
/// the formula already carries the same source text rendered as LaTeX.
pub(super) fn suppress_text_covered_by_formulas(
    blocks: Vec<Block>,
    candidates: &[Block],
) -> Vec<Block> {
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

/// Drop formula candidates that fall inside a high-confidence table region —
/// the table extractor will render that text as cells.
pub(super) fn suppress_formula_candidates_overlapping_tables(
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

/// Interleave detected formulas into the reading-ordered text stream by Y
/// position, re-assigning `reading_order` so the merged result is dense and
/// contiguous.
pub(super) fn merge_text_and_formulas(
    mut text: Vec<Block>,
    mut formulas: Vec<Block>,
) -> Vec<Block> {
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

/// Same shape as `merge_text_and_formulas`, applied to detected tables.
pub(super) fn merge_text_and_tables(mut text: Vec<Block>, mut tables: Vec<Block>) -> Vec<Block> {
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

/// Merge images into the reading stream by Y position. Images use only the Y
/// coordinate (no X tiebreaker) because their bboxes are typically taller than
/// text lines and X-based ties are rare and visually unimportant.
pub(super) fn merge_text_and_images(mut text: Vec<Block>, mut images: Vec<Block>) -> Vec<Block> {
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

/// Concatenate two media block lists in order — embedded images first, then
/// snapshot figures. No reading-order assignment: the result is consumed by
/// `merge_text_and_images` which assigns order during the text merge.
pub(super) fn merge_media_blocks(mut left: Vec<Block>, right: Vec<Block>) -> Vec<Block> {
    left.extend(right);
    left
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::{Bbox, Block, BlockKind, DetectedTable, TableRender};
    use crate::formula::detect::{FormulaCandidate, FormulaStatus};
    use crate::layout::table::{TableEvidence, TableEvidenceSource};
    use std::collections::BTreeSet;

    fn paragraph(id: usize, bbox: Bbox, reading_order: usize, text: &str) -> Block {
        Block {
            id,
            bbox,
            text: text.into(),
            kind: BlockKind::Paragraph,
            font_size: 10.0,
            font_name: String::new(),
            page_num: 0,
            reading_order,
            bold: false,
            italic: false,
        }
    }

    fn formula_block(id: usize, bbox: Bbox, latex: &str) -> Block {
        Block {
            id,
            bbox,
            text: String::new(),
            kind: BlockKind::Formula {
                latex: latex.into(),
                display: true,
            },
            font_size: 0.0,
            font_name: "formula".into(),
            page_num: 0,
            reading_order: 0,
            bold: false,
            italic: false,
        }
    }

    fn table_candidate(bbox: Bbox, confidence: f32, source_ids: &[usize]) -> TableCandidate {
        TableCandidate {
            table: DetectedTable {
                bbox,
                rows: Vec::new(),
                confidence,
                render: TableRender::Markdown,
            },
            source_block_ids: source_ids.iter().copied().collect::<BTreeSet<_>>(),
            evidence: TableEvidence {
                source: TableEvidenceSource::NumericRows,
                row_consistency: confidence,
                column_alignment: confidence,
                numeric_density: confidence,
                row_count: 4,
                ruling_intersections: 0,
                caption_score: 0.0,
                broad_page_penalty: 0.0,
                prose_penalty: 0.0,
                debug_reasons: vec!["test".to_string()],
            },
        }
    }

    fn formula_candidate(bbox: Bbox, confidence: u8) -> FormulaCandidate {
        FormulaCandidate {
            page_num: 0,
            formula_index: 0,
            bbox,
            source_text: "x + y".into(),
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
    fn overlap_ratio_is_zero_for_disjoint_bboxes() {
        let a = Bbox::new(0.0, 0.0, 10.0, 10.0);
        let b = Bbox::new(100.0, 100.0, 110.0, 110.0);
        assert_eq!(bbox_overlap_ratio(a, b), 0.0);
    }

    #[test]
    fn overlap_ratio_is_one_when_a_is_fully_inside_b() {
        let a = Bbox::new(10.0, 10.0, 20.0, 20.0);
        let b = Bbox::new(0.0, 0.0, 100.0, 100.0);
        assert!((bbox_overlap_ratio(a, b) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn overlap_smaller_is_symmetric_and_picks_small_side() {
        let small = Bbox::new(0.0, 0.0, 10.0, 10.0);
        let big = Bbox::new(0.0, 0.0, 100.0, 100.0);
        // The smaller bbox is fully inside, so the ratio against the smaller area is 1.0.
        assert!((bbox_overlap_smaller(small, big) - 1.0).abs() < 1e-5);
        assert!((bbox_overlap_smaller(big, small) - 1.0).abs() < 1e-5);
    }

    #[test]
    fn formula_exclusions_include_tables_and_furniture() {
        let table = table_candidate(Bbox::new(80.0, 100.0, 520.0, 240.0), 0.86, &[]);
        let footer = Bbox::new(0.0, 780.0, 595.0, 842.0);

        let excluded = formula_excluded_regions(&[table], &[footer]);

        assert_eq!(excluded, vec![Bbox::new(80.0, 100.0, 520.0, 240.0), footer]);
    }

    #[test]
    fn furniture_bboxes_suppress_text_blocks() {
        let footer_text = paragraph(
            42,
            Bbox::new(40.0, 790.0, 300.0, 805.0),
            0,
            "Downloaded by ACME",
        );
        let body_text = paragraph(43, Bbox::new(40.0, 120.0, 300.0, 140.0), 1, "Body text");
        let furniture = Bbox::new(0.0, 780.0, 595.0, 842.0);

        let filtered =
            suppress_text_covered_by_furniture(vec![footer_text, body_text], &[furniture]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "Body text");
    }

    #[test]
    fn table_overlap_suppresses_text_blocks_by_id_and_by_bbox() {
        let table = table_candidate(Bbox::new(80.0, 100.0, 520.0, 240.0), 0.9, &[10]);
        let by_id = paragraph(10, Bbox::new(0.0, 0.0, 5.0, 5.0), 0, "cell-by-id");
        let by_overlap = paragraph(11, Bbox::new(100.0, 120.0, 500.0, 220.0), 1, "cell-by-bbox");
        let outside = paragraph(12, Bbox::new(40.0, 400.0, 200.0, 420.0), 2, "outside");

        let filtered = suppress_text_covered_by_tables(vec![by_id, by_overlap, outside], &[table]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].text, "outside");
    }

    #[test]
    fn formula_blocks_suppress_overlapping_text_blocks() {
        let formula = formula_block(99, Bbox::new(120.0, 140.0, 500.0, 162.0), "F = ma");
        let overlapping = paragraph(42, Bbox::new(121.0, 141.0, 499.0, 161.0), 0, "F = ma");

        let filtered = suppress_text_covered_by_formulas(vec![overlapping], &[formula]);

        assert!(filtered.is_empty());
    }

    #[test]
    fn formula_suppression_ignores_non_formula_blocks() {
        // A non-formula block in the `candidates` list must not suppress anything.
        let pretend = paragraph(99, Bbox::new(120.0, 140.0, 500.0, 162.0), 0, "F = ma");
        let overlapping = paragraph(42, Bbox::new(121.0, 141.0, 499.0, 161.0), 0, "F = ma");

        let filtered = suppress_text_covered_by_formulas(vec![overlapping.clone()], &[pretend]);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn suppresses_formula_candidates_inside_strong_tables() {
        let table = table_candidate(Bbox::new(80.0, 100.0, 520.0, 240.0), 0.86, &[]);
        let inside = formula_candidate(Bbox::new(120.0, 140.0, 500.0, 162.0), 80);
        let mut outside = formula_candidate(Bbox::new(160.0, 320.0, 460.0, 342.0), 88);
        outside.source_text = "F = m a".into();
        outside.equation_number = Some("(1)".into());

        let filtered =
            suppress_formula_candidates_overlapping_tables(vec![inside, outside], &[table]);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].source_text, "F = m a");
        // re-indexed during suppression
        assert_eq!(filtered[0].formula_index, 0);
    }

    #[test]
    fn weak_tables_do_not_suppress_formula_candidates() {
        let weak_table = table_candidate(Bbox::new(80.0, 100.0, 520.0, 240.0), 0.50, &[]);
        let candidate = formula_candidate(Bbox::new(120.0, 140.0, 500.0, 162.0), 80);

        let filtered =
            suppress_formula_candidates_overlapping_tables(vec![candidate], &[weak_table]);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn overlapping_table_candidates_keep_the_best_candidate() {
        let broad_first = table_candidate(Bbox::new(60.0, 80.0, 540.0, 320.0), 0.62, &[]);
        let strong_narrow = table_candidate(Bbox::new(82.0, 102.0, 518.0, 238.0), 0.88, &[]);
        let disjoint = table_candidate(Bbox::new(80.0, 400.0, 520.0, 540.0), 0.9, &[]);

        let kept =
            suppress_overlapping_table_candidates(vec![broad_first, strong_narrow, disjoint]);

        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].table.bbox, Bbox::new(82.0, 102.0, 518.0, 238.0));
        assert_eq!(kept[1].table.bbox, Bbox::new(80.0, 400.0, 520.0, 540.0));
    }

    #[test]
    fn merge_text_and_formulas_interleaves_by_y_and_renumbers() {
        let text_a = paragraph(1, Bbox::new(0.0, 100.0, 100.0, 120.0), 0, "a");
        let text_b = paragraph(2, Bbox::new(0.0, 300.0, 100.0, 320.0), 1, "b");
        let formula_mid = formula_block(900, Bbox::new(0.0, 200.0, 100.0, 220.0), "x = 1");

        let merged = merge_text_and_formulas(vec![text_a, text_b], vec![formula_mid]);

        let ordered: Vec<&str> = merged
            .iter()
            .map(|b| match &b.kind {
                BlockKind::Formula { latex, .. } => latex.as_str(),
                _ => b.text.as_str(),
            })
            .collect();
        assert_eq!(ordered, vec!["a", "x = 1", "b"]);
        for (idx, block) in merged.iter().enumerate() {
            assert_eq!(block.reading_order, idx);
        }
    }

    #[test]
    fn merge_text_and_formulas_passthrough_when_no_formulas() {
        let text_a = paragraph(1, Bbox::new(0.0, 100.0, 100.0, 120.0), 5, "a");
        let merged = merge_text_and_formulas(vec![text_a], Vec::new());
        assert_eq!(merged.len(), 1);
        // passthrough does NOT renumber when formulas are empty — caller relies on this
        // to keep upstream reading_order for the simple path.
        assert_eq!(merged[0].reading_order, 5);
    }

    #[test]
    fn merge_text_and_tables_places_table_before_lower_text() {
        let text_a = paragraph(1, Bbox::new(0.0, 100.0, 100.0, 120.0), 0, "a");
        let text_b = paragraph(2, Bbox::new(0.0, 400.0, 100.0, 420.0), 1, "b");
        let table_block = Block {
            id: 500,
            bbox: Bbox::new(0.0, 200.0, 100.0, 300.0),
            text: String::new(),
            kind: BlockKind::CoordinateTable {
                table: DetectedTable {
                    bbox: Bbox::new(0.0, 200.0, 100.0, 300.0),
                    rows: Vec::new(),
                    confidence: 0.9,
                    render: TableRender::Markdown,
                },
            },
            font_size: 0.0,
            font_name: "table".into(),
            page_num: 0,
            reading_order: 0,
            bold: false,
            italic: false,
        };

        let merged = merge_text_and_tables(vec![text_a, text_b], vec![table_block]);

        assert!(matches!(merged[1].kind, BlockKind::CoordinateTable { .. }));
        for (idx, block) in merged.iter().enumerate() {
            assert_eq!(block.reading_order, idx);
        }
    }

    #[test]
    fn merge_text_and_images_interleaves_by_y_only() {
        let text_a = paragraph(1, Bbox::new(0.0, 100.0, 100.0, 120.0), 0, "a");
        let text_b = paragraph(2, Bbox::new(0.0, 400.0, 100.0, 420.0), 1, "b");
        let image = Block {
            id: 700,
            bbox: Bbox::new(0.0, 200.0, 100.0, 350.0),
            text: String::new(),
            kind: BlockKind::Image {
                path: Some("img.png".into()),
            },
            font_size: 0.0,
            font_name: "image".into(),
            page_num: 0,
            reading_order: 0,
            bold: false,
            italic: false,
        };

        let merged = merge_text_and_images(vec![text_a, text_b], vec![image]);

        assert!(matches!(merged[1].kind, BlockKind::Image { .. }));
        assert_eq!(merged[0].text, "a");
        assert_eq!(merged[2].text, "b");
    }

    #[test]
    fn merge_media_blocks_concatenates_in_order() {
        let left = vec![paragraph(1, Bbox::new(0.0, 0.0, 1.0, 1.0), 0, "L")];
        let right = vec![paragraph(2, Bbox::new(0.0, 0.0, 1.0, 1.0), 0, "R")];

        let merged = merge_media_blocks(left, right);

        assert_eq!(
            merged.iter().map(|b| b.text.as_str()).collect::<Vec<_>>(),
            vec!["L", "R"]
        );
    }
}
