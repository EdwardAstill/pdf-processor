use crate::document::types::{BlockKind, RawTextBlock};
use regex::Regex;
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(dead_code)]
pub(crate) enum TableDetectionStrategy {
    LineGrid,
    TextAlignment,
    ExplicitRegion,
}

fn page_number_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*[-–—]?\s*\d+\s*[-–—]?\s*$|^\s*[Pp]age\s+\d+\s*$").unwrap())
}

fn ordered_list_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(\d+[.)]\s+|\(?[a-zA-Z][.)]\s+)").unwrap())
}

fn unordered_list_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*[•·▪▸►\-\*]\s+").unwrap())
}

fn caption_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^\s*(figure|fig\.?|table|tbl\.?|algorithm|listing|exhibit)\s+[\dIVXivx]+[.:)]",
        )
        .unwrap()
    })
}

fn affiliation_keyword_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)\b(university|institute|department|school|college|laborator(?:y|ies)|research|openai|google|microsoft|meta|deepmind|anthropic|inc\.?|corp\.?|llc|san francisco|california|toronto|eth zurich)\b",
        )
        .unwrap()
    })
}

fn numeric_cell_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^\s*\(?-?(?:\d{1,3}(?:[.,]\d{3})+(?:[.,]\d{2})?|\d+(?:[.,]\d{2})|\d+)\)?\s*$")
            .unwrap()
    })
}

fn numbered_section_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(\d+(?:\.\d+)*)(?:[.)])?\s+(.+\S)\s*$").unwrap())
}

/// Detect table cells by finding blocks arranged in a 2D grid.
/// Uses a region-based text-alignment strategy: identifies candidate table
/// regions among non-heading/non-caption blocks, then validates each region.
pub(crate) fn detect_table_cells_with_font_size(
    blocks: &[RawTextBlock],
    body_font_size: f32,
) -> std::collections::HashMap<usize, BlockKind> {
    detect_table_cells_with_strategy(
        blocks,
        body_font_size,
        TableDetectionStrategy::TextAlignment,
    )
}

#[cfg(test)]
pub(crate) fn detect_table_cells(
    blocks: &[RawTextBlock],
) -> std::collections::HashMap<usize, BlockKind> {
    detect_table_cells_with_font_size(blocks, 0.0)
}

pub(crate) fn detect_table_cells_with_strategy(
    blocks: &[RawTextBlock],
    body_font_size: f32,
    strategy: TableDetectionStrategy,
) -> std::collections::HashMap<usize, BlockKind> {
    match strategy {
        TableDetectionStrategy::LineGrid | TableDetectionStrategy::ExplicitRegion => {
            detect_text_alignment_table_cells(blocks, body_font_size)
        }
        TableDetectionStrategy::TextAlignment => {
            detect_text_alignment_table_cells(blocks, body_font_size)
        }
    }
}

fn detect_text_alignment_table_cells(
    blocks: &[RawTextBlock],
    body_font_size: f32,
) -> std::collections::HashMap<usize, BlockKind> {
    use std::collections::HashMap;

    let mut result = HashMap::new();

    if blocks.len() < 4 {
        return result;
    }

    let candidate_blocks: Vec<&RawTextBlock> = blocks
        .iter()
        .filter(|b| !is_non_table_block(b, body_font_size))
        .collect();

    if candidate_blocks.len() < 4 {
        return result;
    }

    let regions = find_table_regions(&candidate_blocks);

    for region in &regions {
        if region.len() < 4 {
            continue;
        }

        for (block_id, kind) in detect_table_in_region(region) {
            result.insert(block_id, kind);
        }
    }

    result
}

fn is_non_table_block(block: &RawTextBlock, body_font_size: f32) -> bool {
    let text = block.text.trim();

    if text.is_empty() {
        return true;
    }

    if caption_re().is_match(text) {
        return true;
    }

    if page_number_re().is_match(text) && block.bbox.width() < 40.0 && text.len() <= 6 {
        return true;
    }

    if ordered_list_re().is_match(text) || unordered_list_re().is_match(text) {
        return true;
    }

    if looks_like_author_affiliation_block(text) {
        return true;
    }

    if body_font_size > 0.0 && block.font_size / body_font_size >= 1.15 {
        return true;
    }

    if body_font_size > 0.0 && looks_like_numbered_section_heading(text, block, body_font_size) {
        return true;
    }

    text.len() > 200
}

fn looks_like_numbered_section_heading(
    text: &str,
    block: &RawTextBlock,
    body_font_size: f32,
) -> bool {
    let Some(captures) = numbered_section_re().captures(text) else {
        return false;
    };
    let title = captures.get(2).map(|m| m.as_str().trim()).unwrap_or("");
    if title.is_empty() || title.len() > 100 || title.ends_with('.') {
        return false;
    }
    block.font_size / body_font_size.max(0.1) >= 1.05 || is_likely_heading_text(title)
}

fn is_likely_heading_text(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() || words.len() > 12 {
        return false;
    }
    let cap_count = words
        .iter()
        .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
        .count();
    cap_count as f32 / words.len() as f32 >= 0.6
}

fn looks_like_author_affiliation_block(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    if trimmed.contains('@') || affiliation_keyword_re().is_match(trimmed) {
        return true;
    }

    let non_empty_lines: Vec<&str> = trimmed
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();
    if non_empty_lines.len() >= 2 {
        return looks_like_author_name_line(non_empty_lines[0]);
    }

    false
}

fn looks_like_author_name_line(text: &str) -> bool {
    let words: Vec<&str> = text
        .split(|c: char| c.is_whitespace() || matches!(c, ',' | ';'))
        .filter(|word| !word.is_empty())
        .collect();
    if !(2..=12).contains(&words.len()) {
        return false;
    }

    let stopwords = [
        "a", "an", "and", "are", "as", "at", "by", "for", "from", "in", "is", "of", "on", "or",
        "the", "to", "with",
    ];

    let capitalized = words
        .iter()
        .filter(|word| {
            word.chars()
                .find(|c| c.is_alphabetic())
                .map(|c| c.is_uppercase())
                .unwrap_or(false)
        })
        .count();
    let stopword_count = words
        .iter()
        .filter(|word| stopwords.contains(&word.to_ascii_lowercase().as_str()))
        .count();
    let lowercase_content_words = words
        .iter()
        .filter(|word| {
            let lower = word.to_ascii_lowercase();
            !stopwords.contains(&lower.as_str())
                && word
                    .chars()
                    .find(|c| c.is_alphabetic())
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
        })
        .count();

    capitalized * 5 >= words.len() * 4 && stopword_count == 0 && lowercase_content_words == 0
}

fn table_column_anchor(block: &RawTextBlock) -> f32 {
    if numeric_cell_re().is_match(block.text.trim()) {
        block.bbox.x1
    } else {
        block.bbox.x0
    }
}

fn find_table_regions<'a>(blocks: &[&'a RawTextBlock]) -> Vec<Vec<&'a RawTextBlock>> {
    if blocks.is_empty() {
        return vec![];
    }

    let mut sorted: Vec<&RawTextBlock> = blocks.to_vec();
    sorted.sort_by(|a, b| {
        a.bbox
            .y0
            .partial_cmp(&b.bbox.y0)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let y_positions: Vec<f32> = sorted.iter().map(|b| b.bbox.y0).collect();
    let y_clusters = cluster_positions(&y_positions, 6.0);

    if y_clusters.len() < 2 {
        return vec![];
    }

    let mut row_assignments: Vec<(usize, &RawTextBlock)> = Vec::new();
    for &block in &sorted {
        if let Some(row) = nearest_cluster(block.bbox.y0, &y_clusters, 6.0) {
            row_assignments.push((row, block));
        }
    }

    let mut rows_map: std::collections::BTreeMap<usize, Vec<&RawTextBlock>> =
        std::collections::BTreeMap::new();
    for (row, block) in &row_assignments {
        rows_map.entry(*row).or_default().push(*block);
    }

    let table_rows: Vec<usize> = rows_map
        .iter()
        .filter(|(_, blocks_in_row)| blocks_in_row.len() >= 2)
        .map(|(row, _)| *row)
        .collect();

    if table_rows.len() < 2 {
        return vec![];
    }

    let mut regions: Vec<Vec<&'a RawTextBlock>> = Vec::new();
    let mut current_region: Vec<&RawTextBlock> = Vec::new();
    let mut prev_row: Option<usize> = None;

    for &row_idx in &table_rows {
        let is_contiguous = match prev_row {
            Some(prev) => row_idx <= prev + 2,
            None => true,
        };

        if is_contiguous {
            for &block in rows_map.get(&row_idx).unwrap() {
                current_region.push(block);
            }
        } else {
            if current_region.len() >= 4 {
                regions.push(current_region);
            }
            current_region = Vec::new();
            for &block in rows_map.get(&row_idx).unwrap() {
                current_region.push(block);
            }
        }
        prev_row = Some(row_idx);
    }
    if current_region.len() >= 4 {
        regions.push(current_region);
    }

    regions
}

fn detect_table_in_region(region: &[&RawTextBlock]) -> std::collections::HashMap<usize, BlockKind> {
    use std::collections::HashMap;

    let mut result = HashMap::new();

    let x_positions: Vec<f32> = region.iter().map(|b| table_column_anchor(b)).collect();
    let x_clusters = cluster_positions(&x_positions, 12.0);
    let y_positions: Vec<f32> = region.iter().map(|b| b.bbox.y0).collect();
    let y_clusters = cluster_positions(&y_positions, 6.0);

    if x_clusters.len() < 2 || y_clusters.len() < 2 || x_clusters.len() > 10 {
        return result;
    }

    for block in region.iter() {
        let col = nearest_cluster(table_column_anchor(block), &x_clusters, 12.0);
        let row = nearest_cluster(block.bbox.y0, &y_clusters, 6.0);
        if let (Some(col), Some(row)) = (col, row) {
            result.insert(block.block_id, BlockKind::TableCell { row, col });
        }
    }

    if result.len() < 4 {
        result.clear();
        return result;
    }

    let mut col_counts: HashMap<usize, usize> = HashMap::new();
    for kind in result.values() {
        if let BlockKind::TableCell { col, .. } = kind {
            *col_counts.entry(*col).or_insert(0) += 1;
        }
    }
    let cols_with_2_plus = col_counts.values().filter(|&&c| c >= 2).count();
    if cols_with_2_plus < 2 {
        result.clear();
        return result;
    }

    let assigned_blocks: Vec<&&RawTextBlock> = region
        .iter()
        .filter(|b| result.contains_key(&b.block_id))
        .collect();

    if let (Some(min_y), Some(max_y)) = (
        assigned_blocks.iter().map(|b| b.bbox.y0).reduce(f32::min),
        assigned_blocks.iter().map(|b| b.bbox.y1).reduce(f32::max),
    ) {
        let table_height = max_y - min_y;
        if table_height < 0.0 {
            result.clear();
        }
    }

    result
}

fn cluster_positions(positions: &[f32], tolerance: f32) -> Vec<f32> {
    let mut sorted = positions.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted.dedup_by(|a, b| (*b - *a).abs() < 0.1);

    let mut clusters: Vec<Vec<f32>> = Vec::new();
    for pos in sorted {
        if let Some(cluster) = clusters.last_mut() {
            let centre = cluster.iter().sum::<f32>() / cluster.len() as f32;
            if (pos - centre).abs() <= tolerance {
                cluster.push(pos);
                continue;
            }
        }
        clusters.push(vec![pos]);
    }

    clusters
        .iter()
        .map(|c| c.iter().sum::<f32>() / c.len() as f32)
        .collect()
}

fn nearest_cluster(value: f32, clusters: &[f32], tolerance: f32) -> Option<usize> {
    clusters
        .iter()
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            (value - *a)
                .abs()
                .partial_cmp(&(value - *b).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .and_then(|(i, &centre)| {
            if (value - centre).abs() <= tolerance {
                Some(i)
            } else {
                None
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::Bbox;

    fn make_block_id(
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        text: &str,
        block_id: usize,
    ) -> RawTextBlock {
        RawTextBlock {
            bbox: Bbox::new(x0, y0, x1, y1),
            text: text.to_string(),
            font_size: 10.0,
            font_name: "unknown".to_string(),
            page_num: 0,
            block_id,
            reading_order: 0,
        }
    }

    #[test]
    fn text_alignment_strategy_detects_simple_grid() {
        let blocks = vec![
            make_block_id(50.0, 100.0, 150.0, 120.0, "A1", 0),
            make_block_id(200.0, 100.0, 300.0, 120.0, "A2", 1),
            make_block_id(50.0, 130.0, 150.0, 150.0, "B1", 2),
            make_block_id(200.0, 130.0, 300.0, 150.0, "B2", 3),
        ];

        let cells =
            detect_table_cells_with_strategy(&blocks, 10.0, TableDetectionStrategy::TextAlignment);

        assert_eq!(cells.len(), 4);
        assert!(cells
            .values()
            .all(|kind| matches!(kind, BlockKind::TableCell { .. })));
    }

    #[test]
    fn strategy_placeholders_preserve_current_behavior() {
        let blocks = vec![
            make_block_id(50.0, 100.0, 150.0, 120.0, "A1", 0),
            make_block_id(200.0, 100.0, 300.0, 120.0, "A2", 1),
            make_block_id(50.0, 130.0, 150.0, 150.0, "B1", 2),
            make_block_id(200.0, 130.0, 300.0, 150.0, "B2", 3),
        ];

        let line_grid =
            detect_table_cells_with_strategy(&blocks, 10.0, TableDetectionStrategy::LineGrid);
        let explicit =
            detect_table_cells_with_strategy(&blocks, 10.0, TableDetectionStrategy::ExplicitRegion);

        assert_eq!(line_grid.len(), 4);
        assert_eq!(explicit.len(), 4);
    }
}
