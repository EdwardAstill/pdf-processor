use crate::cli::TableMode;
use crate::document::types::{Bbox, BlockKind, DetectedTable, RawTextBlock, RawWord, TableRender};
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

#[derive(Clone, Debug)]
pub(crate) struct TableCandidate {
    pub table: DetectedTable,
    pub source_block_ids: std::collections::BTreeSet<usize>,
}

#[derive(Clone, Debug)]
struct WordRow<'a> {
    words: Vec<&'a RawWord>,
    bbox: Bbox,
    baseline_y: f32,
}

pub(crate) fn detect_coordinate_tables(
    words: &[RawWord],
    page_width: f32,
    mode: TableMode,
) -> Vec<TableCandidate> {
    if matches!(mode, TableMode::Off) || words.len() < 12 {
        return Vec::new();
    }

    let rows = build_word_rows(words);
    if rows.len() < 4 {
        return Vec::new();
    }

    let mut candidates = Vec::new();
    let mut idx = 0usize;
    while idx < rows.len() {
        if !looks_like_data_row(&rows[idx]) {
            idx += 1;
            continue;
        }

        let start = idx;
        let mut end = idx + 1;
        while end < rows.len()
            && (looks_like_data_row(&rows[end]) || weak_continuation_row(&rows[end]))
            && rows[end].baseline_y - rows[end - 1].baseline_y <= median_font_size(&rows[end]) * 2.6
        {
            end += 1;
        }

        let data_rows: Vec<usize> = (start..end)
            .filter(|row_idx| looks_like_data_row(&rows[*row_idx]))
            .collect();
        if data_rows.len() < 3 {
            idx = end;
            continue;
        }

        let header_start = find_header_start(&rows, start);
        let region_rows = &rows[header_start..end];
        if let Some(candidate) =
            build_table_candidate(region_rows, start - header_start, page_width, mode)
        {
            candidates.push(candidate);
        }
        idx = end;
    }

    suppress_overlapping_tables(candidates)
}

fn build_word_rows(words: &[RawWord]) -> Vec<WordRow<'_>> {
    let mut sorted: Vec<&RawWord> = words
        .iter()
        .filter(|word| !word.text.trim().is_empty() && word.bbox.width() > 0.0)
        .collect();
    sorted.sort_by(|left, right| {
        left.baseline_y
            .partial_cmp(&right.baseline_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut rows: Vec<WordRow<'_>> = Vec::new();
    for word in sorted {
        let tolerance = (word.font_size * 0.55).max(3.0);
        if let Some(row) = rows
            .iter_mut()
            .find(|row| (word.baseline_y - row.baseline_y).abs() <= tolerance)
        {
            row.baseline_y = (row.baseline_y * row.words.len() as f32 + word.baseline_y)
                / (row.words.len() as f32 + 1.0);
            row.bbox = row.bbox.union(&word.bbox);
            row.words.push(word);
            continue;
        }

        rows.push(WordRow {
            words: vec![word],
            bbox: word.bbox,
            baseline_y: word.baseline_y,
        });
    }

    for row in &mut rows {
        row.words.sort_by(|left, right| {
            left.bbox
                .x0
                .partial_cmp(&right.bbox.x0)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }
    rows.sort_by(|left, right| {
        left.baseline_y
            .partial_cmp(&right.baseline_y)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    rows
}

fn looks_like_data_row(row: &WordRow<'_>) -> bool {
    if row.words.len() < 4 {
        return false;
    }
    let numeric = row
        .words
        .iter()
        .filter(|word| looks_like_table_value(&word.text))
        .count();
    let stock_like = row
        .words
        .iter()
        .filter(|word| word.text.chars().filter(|ch| ch.is_ascii_digit()).count() >= 6)
        .count();
    numeric * 100 / row.words.len().max(1) >= 55 || (stock_like >= 1 && numeric >= 4)
}

fn weak_continuation_row(row: &WordRow<'_>) -> bool {
    row.words.len() >= 4
        && row
            .words
            .iter()
            .filter(|word| looks_like_table_value(&word.text))
            .count()
            >= 3
}

fn looks_like_table_value(text: &str) -> bool {
    let trimmed = text.trim_matches(|ch: char| matches!(ch, ',' | ';' | ':' | '†' | '‡' | '*'));
    if trimmed.is_empty() {
        return false;
    }
    let digit_count = trimmed.chars().filter(|ch| ch.is_ascii_digit()).count();
    if digit_count == 0 {
        return matches!(trimmed, "-" | "–" | "—");
    }
    let allowed = trimmed.chars().all(|ch| {
        ch.is_ascii_digit()
            || ch.is_ascii_alphabetic()
            || matches!(
                ch,
                '.' | ',' | '/' | '-' | '–' | '—' | '"' | '\'' | '+' | '±'
            )
    });
    allowed && digit_count * 2 >= trimmed.chars().filter(|ch| !ch.is_whitespace()).count()
}

fn median_font_size(row: &WordRow<'_>) -> f32 {
    let mut sizes: Vec<f32> = row.words.iter().map(|word| word.font_size).collect();
    sizes.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sizes.get(sizes.len() / 2).copied().unwrap_or(10.0).max(1.0)
}

fn find_header_start(rows: &[WordRow<'_>], data_start: usize) -> usize {
    let mut header_start = data_start;
    let mut cursor = data_start;
    let mut included = 0usize;
    while cursor > 0 && included < 4 {
        let prev = cursor - 1;
        if rows[cursor].baseline_y - rows[prev].baseline_y > median_font_size(&rows[cursor]) * 2.8 {
            break;
        }
        if !looks_like_header_row(&rows[prev]) {
            break;
        }
        header_start = prev;
        cursor = prev;
        included += 1;
    }
    header_start
}

fn looks_like_header_row(row: &WordRow<'_>) -> bool {
    if row.words.is_empty() || row.words.len() > 24 {
        return false;
    }
    let text = row_text(row).to_ascii_lowercase();
    let keywords = [
        "nominal",
        "working",
        "load",
        "limit",
        "stock",
        "weight",
        "dimensions",
        "replacement",
        "size",
        "each",
        "(in)",
        "(kg)",
        "(mm)",
        "(t)",
        "tol",
        "pin",
        "bolt",
    ];
    keywords.iter().any(|keyword| text.contains(keyword))
}

fn build_table_candidate(
    rows: &[WordRow<'_>],
    first_data_idx: usize,
    page_width: f32,
    mode: TableMode,
) -> Option<TableCandidate> {
    let data_rows = &rows[first_data_idx..];
    let mut count_histogram = std::collections::BTreeMap::new();
    for row in data_rows {
        *count_histogram.entry(row.words.len()).or_insert(0usize) += 1;
    }
    let (column_count, matching_rows) = count_histogram
        .into_iter()
        .max_by_key(|(cols, hits)| (*hits, *cols))?;
    if column_count < 3 || matching_rows < 3 {
        return None;
    }

    let mut centers = vec![0.0; column_count];
    let mut counts = vec![0usize; column_count];
    for row in data_rows
        .iter()
        .filter(|row| row.words.len() == column_count)
    {
        for (idx, word) in row.words.iter().enumerate() {
            centers[idx] += word.bbox.center_x();
            counts[idx] += 1;
        }
    }
    for (center, count) in centers.iter_mut().zip(counts.iter()) {
        if *count == 0 {
            return None;
        }
        *center /= *count as f32;
    }

    let header = build_header(rows, first_data_idx, &centers);
    let mut table_rows = Vec::new();
    table_rows.push(header);
    for row in data_rows {
        table_rows.push(assign_row_to_columns(row, &centers));
    }

    let confidence = table_confidence(data_rows, column_count, matching_rows, page_width);
    let bbox = rows
        .iter()
        .map(|row| row.bbox)
        .reduce(|bbox, row_bbox| bbox.union(&row_bbox))?;
    let layout_text = render_layout_text(rows);
    let render = match mode {
        TableMode::Layout => TableRender::Layout { text: layout_text },
        TableMode::Native => TableRender::Markdown,
        TableMode::Auto if confidence >= 0.70 => TableRender::Markdown,
        TableMode::Auto => TableRender::Layout { text: layout_text },
        TableMode::Off => return None,
    };

    let source_block_ids = rows
        .iter()
        .flat_map(|row| row.words.iter().map(|word| word.block_id))
        .collect();

    Some(TableCandidate {
        table: DetectedTable {
            bbox,
            rows: table_rows,
            confidence,
            render,
        },
        source_block_ids,
    })
}

fn build_header(rows: &[WordRow<'_>], first_data_idx: usize, centers: &[f32]) -> Vec<String> {
    let mut cells = vec![Vec::<String>::new(); centers.len()];
    for row in &rows[..first_data_idx] {
        for word in &row.words {
            if let Some(col) = nearest_center(word.bbox.center_x(), centers) {
                cells[col].push(clean_header_token(&word.text));
            }
        }
    }

    cells
        .into_iter()
        .enumerate()
        .map(|(idx, tokens)| {
            let text = collapse_header_tokens(&tokens);
            if text.is_empty() {
                format!("Column {}", idx + 1)
            } else {
                text
            }
        })
        .collect()
}

fn clean_header_token(token: &str) -> String {
    token.trim().trim_matches('|').to_string()
}

fn collapse_header_tokens(tokens: &[String]) -> String {
    let mut out: Vec<String> = Vec::new();
    for token in tokens {
        let trimmed = token.trim();
        if trimmed.is_empty() || out.iter().any(|existing| existing == trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out.join(" ")
        .replace("Stock No.", "Stock No.")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn assign_row_to_columns(row: &WordRow<'_>, centers: &[f32]) -> Vec<String> {
    if row.words.len() == centers.len() {
        return row.words.iter().map(|word| word.text.clone()).collect();
    }

    let mut cells = vec![Vec::<String>::new(); centers.len()];
    for word in &row.words {
        if let Some(col) = nearest_center(word.bbox.center_x(), centers) {
            cells[col].push(word.text.clone());
        }
    }
    cells.into_iter().map(|cell| cell.join(" ")).collect()
}

fn nearest_center(value: f32, centers: &[f32]) -> Option<usize> {
    centers
        .iter()
        .enumerate()
        .min_by(|(_, left), (_, right)| {
            (value - **left)
                .abs()
                .partial_cmp(&(value - **right).abs())
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .map(|(idx, _)| idx)
}

fn table_confidence(
    data_rows: &[WordRow<'_>],
    column_count: usize,
    matching_rows: usize,
    page_width: f32,
) -> f32 {
    let row_consistency = matching_rows as f32 / data_rows.len().max(1) as f32;
    let numeric_density = data_rows
        .iter()
        .flat_map(|row| row.words.iter())
        .filter(|word| looks_like_table_value(&word.text))
        .count() as f32
        / data_rows
            .iter()
            .map(|row| row.words.len())
            .sum::<usize>()
            .max(1) as f32;
    let width_ratio = data_rows
        .iter()
        .map(|row| row.bbox.width())
        .fold(0.0f32, f32::max)
        / page_width.max(1.0);
    (row_consistency * 0.45 + numeric_density * 0.35 + width_ratio.min(1.0) * 0.20).clamp(0.0, 1.0)
        + (column_count >= 6) as u8 as f32 * 0.05
}

fn render_layout_text(rows: &[WordRow<'_>]) -> String {
    let min_x = rows
        .iter()
        .map(|row| row.bbox.x0)
        .fold(f32::INFINITY, f32::min);
    let font_size = rows.iter().map(median_font_size).sum::<f32>() / rows.len().max(1) as f32;
    let points_per_char = (font_size * 0.45).max(3.5);

    let mut lines = Vec::new();
    for row in rows {
        let mut line = String::new();
        for word in &row.words {
            let target = ((word.bbox.x0 - min_x) / points_per_char).round().max(0.0) as usize;
            if line.len() < target {
                line.push_str(&" ".repeat(target - line.len()));
            } else if !line.ends_with(' ') && !line.is_empty() {
                line.push(' ');
            }
            line.push_str(word.text.trim());
        }
        lines.push(line.trim_end().to_string());
    }
    lines.join("\n")
}

fn suppress_overlapping_tables(candidates: Vec<TableCandidate>) -> Vec<TableCandidate> {
    let mut kept: Vec<TableCandidate> = Vec::new();
    'candidate: for candidate in candidates {
        for existing in &kept {
            if overlap_ratio(candidate.table.bbox, existing.table.bbox) > 0.65 {
                continue 'candidate;
            }
        }
        kept.push(candidate);
    }
    kept
}

fn overlap_ratio(a: Bbox, b: Bbox) -> f32 {
    let x0 = a.x0.max(b.x0);
    let y0 = a.y0.max(b.y0);
    let x1 = a.x1.min(b.x1);
    let y1 = a.y1.min(b.y1);
    let intersection = ((x1 - x0).max(0.0)) * ((y1 - y0).max(0.0));
    let smaller = a.area().min(b.area()).max(1.0);
    intersection / smaller
}

fn row_text(row: &WordRow<'_>) -> String {
    row.words
        .iter()
        .map(|word| word.text.as_str())
        .collect::<Vec<_>>()
        .join(" ")
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

    fn word(text: &str, col: usize, row: usize) -> RawWord {
        let x0 = 50.0 + col as f32 * 42.0;
        let y0 = 100.0 + row as f32 * 14.0;
        RawWord {
            bbox: Bbox::new(x0, y0, x0 + 24.0, y0 + 10.0),
            text: text.to_string(),
            font_size: 9.0,
            page_num: 0,
            block_id: row,
            line_id: row,
            baseline_y: y0 + 8.0,
        }
    }

    fn row_words(row: usize, values: &[&str]) -> Vec<RawWord> {
        values
            .iter()
            .enumerate()
            .map(|(col, value)| word(value, col, row))
            .collect()
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

    #[test]
    fn detects_wide_coordinate_table_from_words() {
        let mut words = Vec::new();
        words.extend(row_words(
            0,
            &[
                "Nominal", "Working", "Stock", "Weight", "A", "B", "C", "D", "E", "F", "G", "H",
            ],
        ));
        for row in 1..=4 {
            words.extend(row_words(
                row,
                &[
                    "1/4", "1/2", "1018017", ".06", "12", "1.5", "8", "29", "6", "20", "16", "-",
                ],
            ));
        }

        let tables = detect_coordinate_tables(&words, 600.0, TableMode::Native);

        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].table.rows[0].len(), 12);
        assert!(matches!(tables[0].table.render, TableRender::Markdown));
    }

    #[test]
    fn coordinate_table_layout_mode_preserves_fixed_width_text() {
        let mut words = Vec::new();
        words.extend(row_words(0, &["Nominal", "Working", "Stock", "Weight"]));
        for row in 1..=3 {
            words.extend(row_words(row, &["1/4", "1/2", "1018017", ".06"]));
        }

        let tables = detect_coordinate_tables(&words, 600.0, TableMode::Layout);

        assert_eq!(tables.len(), 1);
        match &tables[0].table.render {
            TableRender::Layout { text } => {
                assert!(text.contains("Nominal"));
                assert!(text.contains("1018017"));
            }
            other => panic!("expected layout table, got {other:?}"),
        }
    }

    #[test]
    fn coordinate_table_ignores_normal_paragraph_words() {
        let mut words = Vec::new();
        for row in 0..5 {
            words.extend(row_words(
                row,
                &[
                    "This",
                    "is",
                    "ordinary",
                    "paragraph",
                    "text",
                    "without",
                    "data",
                ],
            ));
        }

        let tables = detect_coordinate_tables(&words, 600.0, TableMode::Auto);

        assert!(tables.is_empty());
    }
}
