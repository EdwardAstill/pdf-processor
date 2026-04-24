use crate::document::types::{Block, BlockKind, RawPage, RawTextBlock};
use crate::pdf::metadata::PageMetadata;
use regex::Regex;
use std::sync::OnceLock;

// --- Regex patterns (compiled once) ---

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

fn code_block_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    // Heuristic: starts with common code patterns
    RE.get_or_init(|| {
        Regex::new(r"^\s*(```|~~~|def |fn |pub |class |import |from |#include|int |void |return )")
            .unwrap()
    })
}

fn scholarly_metadata_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)^\s*(arxiv:\S+|doi:\S+|https?://doi\.org/\S+|preprint\b|accepted at\b|published as\b)",
        )
        .unwrap()
    })
}

fn scholarly_note_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"(?i)(permission to make digital|all rights reserved|copyright held by|acm isbn|provided proper attribution|correspondence to:)",
        )
        .unwrap()
    })
}

fn numbered_section_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(\d+(?:\.\d+)*)(?:[.)])?\s+(.+\S)\s*$").unwrap())
}

fn struct_role_to_heading_level(role: &str) -> Option<u8> {
    match role {
        "H1" | "Title" => Some(1),
        "H2" => Some(2),
        "H3" => Some(3),
        "H4" => Some(4),
        "H5" => Some(5),
        "H6" => Some(6),
        _ => None,
    }
}

pub struct ClassifierConfig {
    /// Body font size (mode across document). Used as baseline for heading detection.
    pub body_font_size: f32,
    /// Font size >= body * this ratio is a heading candidate. Default: 1.15
    pub heading_size_ratio: f32,
    /// Top/bottom fraction of page height considered header/footer zone. Default: 0.07
    pub header_footer_zone: f32,
}

impl Default for ClassifierConfig {
    fn default() -> Self {
        Self {
            body_font_size: 10.0,
            heading_size_ratio: 1.15,
            header_footer_zone: 0.07,
        }
    }
}

pub struct Classifier {
    config: ClassifierConfig,
}

impl Classifier {
    /// Create a classifier with body font size computed from the document's pages.
    pub fn new_for_document(raw_pages: &[RawPage]) -> Self {
        let body_font_size = compute_body_font_size(raw_pages);
        Self {
            config: ClassifierConfig {
                body_font_size,
                ..Default::default()
            },
        }
    }

    #[allow(dead_code)]
    pub fn with_config(config: ClassifierConfig) -> Self {
        Self { config }
    }

    /// Classify all blocks on a page, returning `Block`s with `BlockKind` assigned.
    #[allow(dead_code)]
    pub fn classify_page(&self, raw_blocks: Vec<RawTextBlock>, page: &RawPage) -> Vec<Block> {
        self.classify_page_with_metadata(raw_blocks, page, None)
    }

    /// Classify a page with an optional `PageMetadata` sidecar providing
    /// font-weight and struct-tree signals. When `metadata` is `None`, this
    /// is equivalent to [`classify_page`].
    pub fn classify_page_with_metadata(
        &self,
        raw_blocks: Vec<RawTextBlock>,
        page: &RawPage,
        metadata: Option<&PageMetadata>,
    ) -> Vec<Block> {
        // First pass: detect table cells (pass body font size for heading exclusion)
        let table_cells = super::table::detect_table_cells_with_font_size(
            &raw_blocks,
            self.config.body_font_size,
        );

        raw_blocks
            .into_iter()
            .map(|rb| {
                let kind = if let Some(tc) = table_cells.get(&rb.block_id) {
                    tc.clone()
                } else {
                    self.classify_block_with_metadata(&rb, page, metadata)
                };
                Block {
                    id: rb.block_id,
                    bbox: rb.bbox,
                    text: rb.text.clone(),
                    kind,
                    font_size: rb.font_size,
                    font_name: rb.font_name.clone(),
                    page_num: rb.page_num,
                    reading_order: rb.reading_order,
                }
            })
            .collect()
    }

    #[allow(dead_code)]
    fn classify_block(&self, block: &RawTextBlock, page: &RawPage) -> BlockKind {
        self.classify_block_with_metadata(block, page, None)
    }

    fn classify_block_with_metadata(
        &self,
        block: &RawTextBlock,
        page: &RawPage,
        metadata: Option<&PageMetadata>,
    ) -> BlockKind {
        let text = block.text.trim();

        if text.is_empty() {
            return BlockKind::Paragraph; // treat empty as paragraph, will be filtered by renderer
        }

        // Header/footer zone detection
        if self.is_in_header_zone(block, page) {
            if page_number_re().is_match(text) {
                return BlockKind::PageNumber;
            }
            return BlockKind::RunningHeader;
        }
        if self.is_in_footer_zone(block, page) {
            if page_number_re().is_match(text) {
                return BlockKind::PageNumber;
            }
            return BlockKind::RunningFooter;
        }

        // Page number (anywhere on page)
        if page_number_re().is_match(text) {
            return BlockKind::PageNumber;
        }

        // Caption
        if caption_re().is_match(text) {
            return BlockKind::Caption;
        }

        // Code block
        if code_block_re().is_match(text) {
            return BlockKind::CodeBlock;
        }

        if text.eq_ignore_ascii_case("abstract") {
            return BlockKind::Heading { level: 2 };
        }

        if let Some(level) = numbered_section_heading_level(text, block, &self.config) {
            return BlockKind::Heading { level };
        }

        // List items
        if ordered_list_re().is_match(text) {
            let depth = indent_depth(block, page);
            return BlockKind::ListItem {
                ordered: true,
                depth,
            };
        }
        if unordered_list_re().is_match(text) {
            let depth = indent_depth(block, page);
            return BlockKind::ListItem {
                ordered: false,
                depth,
            };
        }

        // Scholarly metadata and permission/copyright notes frequently use
        // large fonts on page 1, but should not be promoted to headings.
        if scholarly_metadata_re().is_match(text) || scholarly_note_re().is_match(text) {
            return BlockKind::Paragraph;
        }

        // Heading detection — prefer struct-tree role, then font-size ratio,
        // then bold-at-body-size as a last resort when metadata is present.
        if let Some(md) = metadata {
            if let Some(role) = md.struct_role_for_bbox(&block.bbox) {
                if let Some(level) = struct_role_to_heading_level(role) {
                    return BlockKind::Heading { level };
                }
            }
        }

        let ratio = block.font_size / self.config.body_font_size;
        if ratio >= self.config.heading_size_ratio {
            let level = self.font_size_to_heading_level(block.font_size);
            return BlockKind::Heading { level };
        }

        // Bold-at-body-size heading signal — only when metadata is available.
        // A short, bold, non-sentence-terminated line at body font size is
        // almost always a subsection heading in documents that don't use
        // size hierarchy.
        if let Some(md) = metadata {
            if let Some(font) = md.font_for_bbox(&block.bbox) {
                if font.is_bold() && text.len() <= 120 && !text.ends_with('.') {
                    return BlockKind::Heading { level: 4 };
                }
            }
        }

        // Short, single-line text at larger-ish size (section headers with same body size)
        // Heuristic: <= 80 chars, no trailing period, all-caps or title-case dominant
        // Only apply if we couldn't detect via size — weak signal, be conservative
        if text.len() <= 80 && !text.ends_with('.') && is_likely_heading_text(text) && ratio >= 0.99
        {
            // Skip for now to avoid false positives
        }

        BlockKind::Paragraph
    }

    fn font_size_to_heading_level(&self, font_size: f32) -> u8 {
        let ratio = font_size / self.config.body_font_size;
        if ratio >= 2.0 {
            1
        } else if ratio >= 1.6 {
            2
        } else if ratio >= 1.35 {
            3
        } else if ratio >= 1.15 {
            4
        } else {
            5
        }
    }

    fn is_in_header_zone(&self, block: &RawTextBlock, page: &RawPage) -> bool {
        block.bbox.y1 <= page.height * self.config.header_footer_zone
    }

    fn is_in_footer_zone(&self, block: &RawTextBlock, page: &RawPage) -> bool {
        block.bbox.y0 >= page.height * (1.0 - self.config.header_footer_zone)
    }
}

/// Compute the body font size as the statistical mode across all blocks in the document.
/// Uses 0.5pt histogram bins.
fn compute_body_font_size(raw_pages: &[RawPage]) -> f32 {
    use std::collections::HashMap;

    let mut histogram: HashMap<u32, usize> = HashMap::new();

    for page in raw_pages {
        for block in &page.blocks {
            if block.font_size > 0.0 {
                // Bin to nearest 0.5pt: multiply by 2, round, store as integer key
                let key = (block.font_size * 2.0).round() as u32;
                *histogram.entry(key).or_insert(0) += 1;
            }
        }
    }

    if histogram.is_empty() {
        return 10.0; // fallback
    }

    let mode_key = histogram
        .into_iter()
        .max_by_key(|(_, count)| *count)
        .map(|(key, _)| key)
        .unwrap_or(20); // 20 → 10.0pt

    mode_key as f32 / 2.0
}

/// Estimate indent depth from x position (0 = leftmost, higher = more indented).
fn indent_depth(block: &RawTextBlock, page: &RawPage) -> u8 {
    let x_fraction = block.bbox.x0 / page.width;
    if x_fraction < 0.15 {
        0
    } else if x_fraction < 0.25 {
        1
    } else {
        2
    }
}

/// Heuristic: is this text likely a heading by its content alone?
/// Checks for all-caps or title-case (most words capitalised, short).
fn is_likely_heading_text(text: &str) -> bool {
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() || words.len() > 12 {
        return false;
    }
    let cap_count = words
        .iter()
        .filter(|w| w.chars().next().map(|c| c.is_uppercase()).unwrap_or(false))
        .count();
    // Title case: >= 60% of words capitalised
    cap_count as f32 / words.len() as f32 >= 0.6
}

fn numbered_section_heading_level(
    text: &str,
    block: &RawTextBlock,
    config: &ClassifierConfig,
) -> Option<u8> {
    let captures = numbered_section_re().captures(text)?;
    let label = captures.get(1)?.as_str();
    let title = captures.get(2)?.as_str().trim();
    if title.is_empty() || title.len() > 100 || title.ends_with('.') {
        return None;
    }

    let ratio = block.font_size / config.body_font_size.max(0.1);
    if ratio < 1.05 && !is_likely_heading_text(title) {
        return None;
    }

    let depth = label.split('.').filter(|part| !part.is_empty()).count();
    if depth == 0 {
        return None;
    }

    Some((depth as u8 + 1).clamp(2, 6))
}

#[cfg(test)]
fn detect_table_cells(blocks: &[RawTextBlock]) -> std::collections::HashMap<usize, BlockKind> {
    super::table::detect_table_cells(blocks)
}

#[cfg(test)]
fn detect_table_cells_with_font_size(
    blocks: &[RawTextBlock],
    body_font_size: f32,
) -> std::collections::HashMap<usize, BlockKind> {
    super::table::detect_table_cells_with_font_size(blocks, body_font_size)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::Bbox;

    fn make_page(width: f32, height: f32, blocks: Vec<RawTextBlock>) -> RawPage {
        RawPage {
            page_num: 0,
            width,
            height,
            blocks,
            image_refs: vec![],
        }
    }

    fn make_block(x0: f32, y0: f32, x1: f32, y1: f32, text: &str, font_size: f32) -> RawTextBlock {
        make_block_id(x0, y0, x1, y1, text, font_size, 0)
    }

    fn make_block_id(
        x0: f32,
        y0: f32,
        x1: f32,
        y1: f32,
        text: &str,
        font_size: f32,
        block_id: usize,
    ) -> RawTextBlock {
        RawTextBlock {
            bbox: Bbox::new(x0, y0, x1, y1),
            text: text.to_string(),
            font_size,
            font_name: "unknown".to_string(),
            page_num: 0,
            block_id,
            reading_order: 0,
        }
    }

    #[test]
    fn heading_detected_by_font_size() {
        let page = make_page(600.0, 800.0, vec![]);
        let config = ClassifierConfig {
            body_font_size: 10.0,
            heading_size_ratio: 1.15,
            header_footer_zone: 0.07,
        };
        let clf = Classifier::with_config(config);
        let block = make_block(50.0, 100.0, 400.0, 130.0, "Introduction", 18.0);
        let kind = clf.classify_block(&block, &page);
        assert!(matches!(kind, BlockKind::Heading { level: 2 }));
    }

    #[test]
    fn academic_numbered_top_level_section_becomes_h2_not_list_item() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = classifier_with_body(10.0);
        let block = make_block(50.0, 180.0, 360.0, 198.0, "1. Introduction", 13.0);

        assert_eq!(
            clf.classify_block(&block, &page),
            BlockKind::Heading { level: 2 }
        );
    }

    #[test]
    fn academic_numbered_subsection_becomes_h3_not_h4() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = classifier_with_body(10.0);
        let block = make_block(
            50.0,
            220.0,
            430.0,
            238.0,
            "3.1 Encoder and Decoder Stacks",
            12.0,
        );

        assert_eq!(
            clf.classify_block(&block, &page),
            BlockKind::Heading { level: 3 }
        );
    }

    #[test]
    fn abstract_heading_becomes_h2() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = classifier_with_body(10.0);
        let block = make_block(50.0, 140.0, 180.0, 158.0, "Abstract", 12.0);

        assert_eq!(
            clf.classify_block(&block, &page),
            BlockKind::Heading { level: 2 }
        );
    }

    #[test]
    fn paragraph_at_body_size() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = Classifier::with_config(ClassifierConfig::default());
        let block = make_block(
            50.0,
            200.0,
            550.0,
            215.0,
            "This is a normal paragraph.",
            10.0,
        );
        assert_eq!(clf.classify_block(&block, &page), BlockKind::Paragraph);
    }

    #[test]
    fn page_number_standalone_digit() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = Classifier::with_config(ClassifierConfig::default());
        let block = make_block(280.0, 400.0, 320.0, 415.0, "42", 10.0);
        assert_eq!(clf.classify_block(&block, &page), BlockKind::PageNumber);
    }

    #[test]
    fn running_header_in_top_zone() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = Classifier::with_config(ClassifierConfig::default());
        // y1 = 30 <= 800 * 0.07 = 56
        let block = make_block(50.0, 10.0, 400.0, 30.0, "Chapter 1: Overview", 9.0);
        assert_eq!(clf.classify_block(&block, &page), BlockKind::RunningHeader);
    }

    #[test]
    fn ordered_list_item() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = Classifier::with_config(ClassifierConfig::default());
        let block = make_block(50.0, 200.0, 500.0, 215.0, "1. First item", 10.0);
        assert!(matches!(
            clf.classify_block(&block, &page),
            BlockKind::ListItem { ordered: true, .. }
        ));
    }

    #[test]
    fn unordered_list_item() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = Classifier::with_config(ClassifierConfig::default());
        let block = make_block(50.0, 200.0, 500.0, 215.0, "• Bullet point", 10.0);
        assert!(matches!(
            clf.classify_block(&block, &page),
            BlockKind::ListItem { ordered: false, .. }
        ));
    }

    #[test]
    fn caption_detected() {
        let page = make_page(600.0, 800.0, vec![]);
        let clf = Classifier::with_config(ClassifierConfig::default());
        let block = make_block(50.0, 400.0, 500.0, 415.0, "Figure 1. A diagram.", 9.0);
        assert_eq!(clf.classify_block(&block, &page), BlockKind::Caption);
    }

    #[test]
    fn body_font_size_computed_as_mode() {
        let block = |fs: f32| RawTextBlock {
            bbox: Bbox::new(0.0, 0.0, 100.0, 20.0),
            text: "x".to_string(),
            font_size: fs,
            font_name: "unknown".to_string(),
            page_num: 0,
            block_id: 0,
            reading_order: 0,
        };
        let pages = vec![RawPage {
            page_num: 0,
            width: 600.0,
            height: 800.0,
            blocks: vec![
                block(12.0),
                block(12.0),
                block(12.0),
                block(18.0),
                block(24.0),
            ],
            image_refs: vec![],
        }];
        assert_eq!(compute_body_font_size(&pages), 12.0);
    }

    #[test]
    fn table_cell_detection_2x2() {
        let blocks = vec![
            make_block_id(50.0, 100.0, 150.0, 120.0, "A1", 10.0, 0),
            make_block_id(200.0, 100.0, 300.0, 120.0, "A2", 10.0, 1),
            make_block_id(50.0, 130.0, 150.0, 150.0, "B1", 10.0, 2),
            make_block_id(200.0, 130.0, 300.0, 150.0, "B2", 10.0, 3),
        ];
        let cells = detect_table_cells(&blocks);
        assert_eq!(cells.len(), 4);
        assert!(cells
            .values()
            .all(|k| matches!(k, BlockKind::TableCell { .. })));
    }

    #[test]
    fn table_detection_realistic_engineering_page() {
        // Simulates an engineering standards page:
        // - Title block at top (large font, heading)
        // - 3x4 table data grid (12 cells, ~60% of blocks)
        // - Notes block below
        // Total: 14 blocks, table is the majority
        let mut blocks = Vec::new();
        let mut id = 0;

        // Title: "TABLE E1.1 Selection Table" — large font (heading)
        blocks.push(make_block_id(
            50.0,
            30.0,
            500.0,
            55.0,
            "TABLE E1.1 Selection Table for Application",
            14.0,
            id,
        ));
        id += 1;

        // Table header row (3 columns)
        blocks.push(make_block_id(
            50.0,
            80.0,
            180.0,
            100.0,
            "Cross Section",
            10.0,
            id,
        ));
        id += 1;
        blocks.push(make_block_id(
            200.0,
            80.0,
            350.0,
            100.0,
            "Limit State",
            10.0,
            id,
        ));
        id += 1;
        blocks.push(make_block_id(
            370.0,
            80.0,
            520.0,
            100.0,
            "Reference",
            10.0,
            id,
        ));
        id += 1;

        // Data row 1
        blocks.push(make_block_id(
            50.0, 110.0, 180.0, 130.0, "W-shape", 10.0, id,
        ));
        id += 1;
        blocks.push(make_block_id(
            200.0,
            110.0,
            350.0,
            130.0,
            "Flexural Yielding",
            10.0,
            id,
        ));
        id += 1;
        blocks.push(make_block_id(
            370.0,
            110.0,
            520.0,
            130.0,
            "Section F2",
            10.0,
            id,
        ));
        id += 1;

        // Data row 2
        blocks.push(make_block_id(
            50.0, 140.0, 180.0, 160.0, "Channel", 10.0, id,
        ));
        id += 1;
        blocks.push(make_block_id(200.0, 140.0, 350.0, 160.0, "LTB", 10.0, id));
        id += 1;
        blocks.push(make_block_id(
            370.0,
            140.0,
            520.0,
            160.0,
            "Section F3",
            10.0,
            id,
        ));
        id += 1;

        // Data row 3
        blocks.push(make_block_id(50.0, 170.0, 180.0, 190.0, "HSS", 10.0, id));
        id += 1;
        blocks.push(make_block_id(
            200.0,
            170.0,
            350.0,
            190.0,
            "Local Buckling",
            10.0,
            id,
        ));
        id += 1;
        blocks.push(make_block_id(
            370.0,
            170.0,
            520.0,
            190.0,
            "Section F7",
            10.0,
            id,
        ));
        id += 1;

        // Notes below table
        blocks.push(make_block_id(
            50.0,
            210.0,
            520.0,
            240.0,
            "User Note: See Commentary for further discussion.",
            10.0,
            id,
        ));

        // Use body_font_size=10.0 so the 14pt title is detected as heading and excluded
        let cells = detect_table_cells_with_font_size(&blocks, 10.0);

        // Should detect 12 table cells (3 cols x 4 rows)
        assert!(
            cells.len() >= 12,
            "Expected >= 12 table cells, got {} (cells: {:?})",
            cells.len(),
            cells
        );
        assert!(cells
            .values()
            .all(|k| matches!(k, BlockKind::TableCell { .. })));

        // Title (id=0) and notes (id=13) should NOT be in the table
        assert!(!cells.contains_key(&0), "Title should not be a table cell");
        assert!(!cells.contains_key(&13), "Notes should not be a table cell");
    }

    #[test]
    fn table_detection_full_page_table_not_rejected() {
        // A full-page table (occupying ~70% of page height) should still be detected.
        // Previously Guard 2 (40% height limit) would reject this.
        let mut blocks = Vec::new();
        let mut id = 0;

        // 5 columns x 8 rows = 40 cells spanning y=100 to y=660 on an 800pt page
        for row in 0..8 {
            for col in 0..5 {
                let x0 = 50.0 + col as f32 * 100.0;
                let y0 = 100.0 + row as f32 * 80.0;
                blocks.push(make_block_id(
                    x0,
                    y0,
                    x0 + 80.0,
                    y0 + 20.0,
                    &format!("R{}C{}", row, col),
                    10.0,
                    id,
                ));
                id += 1;
            }
        }

        let cells = detect_table_cells(&blocks);
        assert!(
            cells.len() >= 40,
            "Full-page table should be detected, got {} cells",
            cells.len()
        );
    }

    #[test]
    fn table_detection_excludes_headings_and_captions() {
        // Blocks with heading font sizes or caption patterns should be excluded
        let blocks = vec![
            // Heading (large font)
            make_block_id(50.0, 30.0, 400.0, 55.0, "CHAPTER 5", 18.0, 0),
            // Caption
            make_block_id(
                50.0,
                60.0,
                400.0,
                75.0,
                "Table 5.1: Material Properties",
                10.0,
                1,
            ),
            // 2x2 table
            make_block_id(50.0, 100.0, 180.0, 120.0, "Steel", 10.0, 2),
            make_block_id(200.0, 100.0, 350.0, 120.0, "Fy = 50 ksi", 10.0, 3),
            make_block_id(50.0, 130.0, 180.0, 150.0, "Aluminum", 10.0, 4),
            make_block_id(200.0, 130.0, 350.0, 150.0, "Fy = 35 ksi", 10.0, 5),
        ];

        let cells = detect_table_cells_with_font_size(&blocks, 10.0);

        // Table cells should be detected (ids 2-5)
        assert_eq!(cells.len(), 4, "Should detect 4 table cells");
        // Heading and caption should not be table cells
        assert!(
            !cells.contains_key(&0),
            "Heading should not be a table cell"
        );
        assert!(
            !cells.contains_key(&1),
            "Caption should not be a table cell"
        );
    }

    #[test]
    fn table_detection_handles_right_aligned_numeric_columns() {
        let blocks = vec![
            make_block_id(40.0, 80.0, 220.0, 100.0, "Line item", 10.0, 0),
            make_block_id(290.0, 80.0, 360.0, 100.0, "2024", 10.0, 1),
            make_block_id(380.0, 80.0, 450.0, 100.0, "2023", 10.0, 2),
            make_block_id(40.0, 112.0, 220.0, 132.0, "Revenue", 10.0, 3),
            make_block_id(315.0, 112.0, 360.0, 132.0, "120.062.000", 10.0, 4),
            make_block_id(405.0, 112.0, 450.0, 132.0, "124.406.000", 10.0, 5),
            make_block_id(40.0, 144.0, 220.0, 164.0, "EBITDA", 10.0, 6),
            make_block_id(322.0, 144.0, 360.0, 164.0, "8.509.000", 10.0, 7),
            make_block_id(412.0, 144.0, 450.0, 164.0, "10.808.000", 10.0, 8),
        ];

        let cells = detect_table_cells_with_font_size(&blocks, 10.0);
        assert_eq!(cells.len(), 9, "all right-aligned cells should be kept");

        let row0_cols: Vec<usize> = (0..=2)
            .map(|id| match cells.get(&id) {
                Some(BlockKind::TableCell { col, .. }) => *col,
                other => panic!("expected table cell for id {id}, got {other:?}"),
            })
            .collect();
        assert_eq!(row0_cols, vec![0, 1, 2]);
    }

    // ========================================================================
    // Metadata-aware classification (Phase 3)
    // ========================================================================

    use crate::pdf::metadata::{FontInfo, PageMetadata, StructTag};

    fn classifier_with_body(size: f32) -> Classifier {
        Classifier::with_config(ClassifierConfig {
            body_font_size: size,
            ..Default::default()
        })
    }

    fn page_for_classifier_tests() -> RawPage {
        RawPage {
            page_num: 0,
            width: 612.0,
            height: 792.0,
            blocks: Vec::new(),
            image_refs: Vec::new(),
        }
    }

    #[test]
    fn metadata_none_is_identical_to_classify_block() {
        let clf = classifier_with_body(10.0);
        let page = page_for_classifier_tests();
        let block = make_block(100.0, 100.0, 500.0, 115.0, "A section header at 11pt", 11.0);

        let without = clf.classify_block(&block, &page);
        let with_none = clf.classify_block_with_metadata(&block, &page, None);
        assert_eq!(without, with_none);
    }

    #[test]
    fn scholarly_metadata_does_not_become_heading() {
        let clf = classifier_with_body(10.0);
        let page = page_for_classifier_tests();
        let block = make_block(
            100.0,
            100.0,
            500.0,
            120.0,
            "arXiv:1706.03762v7  [cs.CL]  2 Aug 2023",
            16.0,
        );

        assert_eq!(
            clf.classify_block(&block, &page),
            BlockKind::Paragraph,
            "scholarly metadata should stay paragraph even at large font size"
        );
    }

    #[test]
    fn author_cells_with_emails_are_not_table_cells() {
        let blocks = vec![
            make_block_id(
                50.0,
                80.0,
                220.0,
                140.0,
                "Ashish Vaswani\nGoogle Brain\navaswani@google.com",
                10.0,
                0,
            ),
            make_block_id(
                260.0,
                80.0,
                430.0,
                140.0,
                "Noam Shazeer\nGoogle Brain\nnoam@google.com",
                10.0,
                1,
            ),
            make_block_id(50.0, 160.0, 430.0, 180.0, "Abstract", 12.0, 2),
            make_block_id(
                50.0,
                200.0,
                430.0,
                220.0,
                "The Transformer replaces recurrence with attention.",
                10.0,
                3,
            ),
        ];

        let cells = detect_table_cells_with_font_size(&blocks, 10.0);
        assert!(
            cells.is_empty(),
            "author contact blocks should not be classified as markdown tables"
        );
    }

    #[test]
    fn bold_at_body_size_becomes_heading_with_metadata() {
        let clf = classifier_with_body(10.0);
        let page = page_for_classifier_tests();
        // Body-size (10pt) short line — ordinarily a Paragraph.
        let block = make_block(100.0, 100.0, 500.0, 112.0, "Methods", 10.0);

        let without_md = clf.classify_block_with_metadata(&block, &page, None);
        assert_eq!(
            without_md,
            BlockKind::Paragraph,
            "no metadata → stays paragraph"
        );

        // Now with metadata saying that bbox is bold (weight 700).
        let mut md = PageMetadata::default();
        md.fonts.push((
            block.bbox,
            FontInfo {
                family: "Helvetica-Bold".to_string(),
                weight: 700,
                italic: false,
            },
        ));

        let with_md = clf.classify_block_with_metadata(&block, &page, Some(&md));
        assert_eq!(with_md, BlockKind::Heading { level: 4 });
    }

    #[test]
    fn bold_long_sentence_is_not_heading() {
        let clf = classifier_with_body(10.0);
        let page = page_for_classifier_tests();
        // Bold long paragraph-ish — ends with period. Don't upgrade.
        let block = make_block(
            100.0,
            100.0,
            500.0,
            115.0,
            "This is a perfectly ordinary paragraph written in bold because the author enjoys emphasis, and it ends with a sentence-terminating period.",
            10.0,
        );
        let mut md = PageMetadata::default();
        md.fonts.push((
            block.bbox,
            FontInfo {
                family: "Times-Bold".to_string(),
                weight: 700,
                italic: false,
            },
        ));
        assert_eq!(
            clf.classify_block_with_metadata(&block, &page, Some(&md)),
            BlockKind::Paragraph,
            "bold running prose is not a heading"
        );
    }

    #[test]
    fn struct_tree_role_h2_wins_over_size_ratio() {
        let clf = classifier_with_body(10.0);
        let page = page_for_classifier_tests();
        // Same-size-as-body block — not a heading by size.
        let block = make_block(100.0, 100.0, 500.0, 115.0, "Background", 10.0);

        let mut md = PageMetadata::default();
        md.struct_tags.push(StructTag {
            bbox: block.bbox,
            role: "H2".to_string(),
            alt: None,
        });

        assert_eq!(
            clf.classify_block_with_metadata(&block, &page, Some(&md)),
            BlockKind::Heading { level: 2 }
        );
    }

    #[test]
    fn struct_tree_title_maps_to_h1() {
        let clf = classifier_with_body(10.0);
        let page = page_for_classifier_tests();
        let block = make_block(100.0, 100.0, 500.0, 115.0, "My Thesis", 10.0);

        let mut md = PageMetadata::default();
        md.struct_tags.push(StructTag {
            bbox: block.bbox,
            role: "Title".to_string(),
            alt: None,
        });

        assert_eq!(
            clf.classify_block_with_metadata(&block, &page, Some(&md)),
            BlockKind::Heading { level: 1 }
        );
    }

    #[test]
    fn unknown_struct_role_falls_through_to_size_detection() {
        let clf = classifier_with_body(10.0);
        let page = page_for_classifier_tests();
        let block = make_block(100.0, 100.0, 500.0, 115.0, "Some text", 10.0);

        let mut md = PageMetadata::default();
        md.struct_tags.push(StructTag {
            bbox: block.bbox,
            role: "NonsenseRole".to_string(),
            alt: None,
        });

        // Should fall through — size is body, so Paragraph.
        assert_eq!(
            clf.classify_block_with_metadata(&block, &page, Some(&md)),
            BlockKind::Paragraph
        );
    }
}
