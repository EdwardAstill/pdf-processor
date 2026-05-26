#![allow(dead_code)]

use crate::document::types::{
    Block, BlockKind, DetectedTable, Document, ExtractedImage, Page, Section, TableRender,
};
use crate::error::PdfpResult;
use crate::layout::table_inference::{
    detect_structured_region, looks_like_key_value_label, normalize_field_label,
    normalize_field_value, FormField, ParsedNumericRow, StructuredRegion, StructuredRegionKind,
};
use crate::render::media::{
    build_page_media_plan, build_render_context, should_suppress_repeated_text_block, RenderContext,
};
use crate::render::scholarly::{is_abstract_heading, render_scholarly_first_page};
use crate::render::text::{
    append_paragraph, append_plain_text, escape_comment_attr, escape_table_cell, inline_wrap,
    normalize_front_matter_text, normalize_heading_text, normalize_paragraph_text,
};
use std::path::{Path, PathBuf};

/// The output of rendering a document — markdown text plus structured sections.
pub struct RenderedDocument {
    /// Full document markdown (all pages concatenated).
    pub markdown: String,
    /// Document split into sections (by heading boundaries).
    pub sections: Vec<Section>,
    /// Images extracted to disk during rendering.
    pub images: Vec<ExtractedImage>,
    /// Source document path.
    pub source_path: PathBuf,
}

pub struct MarkdownRenderer {
    /// Whether to extract images to disk. False = skip image extraction.
    pub extract_images: bool,
    /// Directory to write extracted images into (e.g. `output/images/`).
    pub image_output_dir: Option<PathBuf>,
}

impl MarkdownRenderer {
    pub fn new(extract_images: bool, image_output_dir: Option<PathBuf>) -> Self {
        Self {
            extract_images,
            image_output_dir,
        }
    }

    pub fn render_document(&self, doc: &Document) -> PdfpResult<RenderedDocument> {
        let mut all_markdown = String::new();
        let mut all_images: Vec<ExtractedImage> = Vec::new();
        let mut image_counter = 0usize;
        let render_ctx = build_render_context(doc);

        for page in &doc.pages {
            let (page_md, mut page_images) =
                self.render_page(page, &render_ctx, &doc.source_path, &mut image_counter)?;
            all_markdown.push_str(&page_md);
            all_images.append(&mut page_images);
        }

        let sections = split_into_sections(&all_markdown, doc);

        Ok(RenderedDocument {
            markdown: all_markdown,
            sections,
            images: all_images,
            source_path: doc.source_path.clone(),
        })
    }

    fn render_page(
        &self,
        page: &Page,
        render_ctx: &RenderContext,
        _source_pdf: &Path,
        _image_counter: &mut usize,
    ) -> PdfpResult<(String, Vec<ExtractedImage>)> {
        let mut md = String::new();
        let images: Vec<ExtractedImage> = Vec::new();

        // Emit a page boundary marker (1-indexed, invisible when rendered)
        md.push_str(&format!("<!-- page:{} -->\n", page.page_num + 1));

        // Hybrid override: if this page was routed through the external
        // backend, emit its markdown verbatim and skip the block-based render.
        if let Some(override_md) = &page.override_markdown {
            md.push_str(override_md.trim_end());
            md.push_str("\n\n");
            return Ok((md, images));
        }

        // Warn if page has no extractable text (scanned image or empty page)
        if page.blocks.is_empty() {
            md.push_str(&format!(
                "<!-- WARNING: page {} has no extractable text (scanned image or empty page) -->\n",
                page.page_num + 1
            ));
        }

        // Sort blocks by reading_order
        let mut blocks: Vec<&Block> = page.blocks.iter().collect();
        blocks.sort_by_key(|b| b.reading_order);
        let scholarly_front_page = page.page_num == 0
            && blocks
                .iter()
                .any(|block| is_abstract_heading(block.text.trim()));
        let media_plan = build_page_media_plan(page, &blocks, render_ctx, scholarly_front_page);

        let mut i = 0;
        if let Some(front_matter) =
            render_scholarly_first_page(&blocks, page, &media_plan, append_rendered_block)
        {
            md.push_str(&front_matter.markdown);
            i = front_matter.next_index;
        }

        while i < blocks.len() {
            if let Some(structured) = detect_structured_region(&blocks, i) {
                render_structured_region(&mut md, &blocks, i, &structured);
                i = structured.next_index;
                continue;
            }

            let block = blocks[i];
            if let Some(override_md) = &block.override_markdown {
                md.push_str(override_md.trim_end());
                md.push_str("\n\n");
                i += 1;
                continue;
            }
            if should_suppress_repeated_text_block(block, page, render_ctx) {
                i += 1;
                continue;
            }
            match &block.kind {
                BlockKind::Heading { level } => {
                    md.push_str(&render_heading(&block.text, *level));
                    i += 1;
                }
                BlockKind::Paragraph => {
                    let text = normalize_paragraph_text(&block.text);
                    if !text.is_empty() {
                        md.push_str(&inline_wrap(&text, block.bold, block.italic));
                        md.push_str("\n\n");
                    }
                    i += 1;
                }
                BlockKind::ListItem { .. } => {
                    // Collect consecutive list items
                    let mut list_blocks = vec![block];
                    let mut j = i + 1;
                    while j < blocks.len() {
                        if matches!(&blocks[j].kind, BlockKind::ListItem { .. }) {
                            list_blocks.push(blocks[j]);
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    md.push_str(&render_list(&list_blocks));
                    md.push('\n');
                    i = j;
                }
                BlockKind::TableCell { .. } => {
                    // Collect all consecutive table cells
                    let mut table_blocks = vec![block];
                    let mut j = i + 1;
                    while j < blocks.len() {
                        if matches!(&blocks[j].kind, BlockKind::TableCell { .. }) {
                            table_blocks.push(blocks[j]);
                            j += 1;
                        } else {
                            break;
                        }
                    }
                    md.push_str(&render_table(&table_blocks));
                    md.push('\n');
                    i = j;
                }
                BlockKind::CoordinateTable { table } => {
                    md.push_str(&render_coordinate_table(table));
                    md.push('\n');
                    i += 1;
                }
                BlockKind::Caption => {
                    md.push('*');
                    md.push_str(block.text.trim());
                    md.push_str("*\n\n");
                    i += 1;
                }
                BlockKind::CodeBlock => {
                    md.push_str("```\n");
                    md.push_str(block.text.trim());
                    md.push_str("\n```\n\n");
                    i += 1;
                }
                BlockKind::Image { path } => {
                    if media_plan.kept_block_ids.contains(&block.id) {
                        if let Some(p) = path {
                            md.push_str(&format!("![image]({})\n\n", p));
                        }
                    }
                    i += 1;
                }
                BlockKind::Figure { path, caption } => {
                    if media_plan.kept_block_ids.contains(&block.id) {
                        if let Some(p) = path {
                            md.push_str(&format!("![image]({})\n\n", p));
                        }
                        if let Some(c) = caption {
                            md.push('*');
                            md.push_str(c.trim());
                            md.push_str("*\n\n");
                        }
                    }
                    i += 1;
                }
                BlockKind::Formula { latex, display } => {
                    if *display {
                        md.push_str(&format!("$$ {} $$\n\n", latex.trim()));
                    } else {
                        md.push_str(&format!("${}$\n\n", latex.trim()));
                    }
                    i += 1;
                }
                BlockKind::FormulaReview { reason, crop_path } => {
                    md.push_str(&format!(
                        "<!-- formula-review: page={} reason=\"{}\"",
                        page.page_num + 1,
                        escape_comment_attr(reason)
                    ));
                    if let Some(path) = crop_path {
                        md.push_str(&format!(" crop=\"{}\"", escape_comment_attr(path)));
                    }
                    md.push_str(" -->\n\n");
                    i += 1;
                }
                // Skip navigation artifacts
                BlockKind::PageNumber
                | BlockKind::RunningHeader
                | BlockKind::RunningFooter
                | BlockKind::Artifact => {
                    i += 1;
                }
            }
        }

        Ok((md, images))
    }
}

fn render_heading(text: &str, level: u8) -> String {
    let prefix = "#".repeat(level.clamp(1, 6) as usize);
    format!("{} {}\n\n", prefix, normalize_heading_text(text.trim()))
}

fn render_list(blocks: &[&Block]) -> String {
    let mut result = String::new();
    let mut ordered_counters: std::collections::HashMap<u8, usize> =
        std::collections::HashMap::new();

    for block in blocks {
        if let BlockKind::ListItem { ordered, depth } = &block.kind {
            let indent = "  ".repeat(*depth as usize);
            let item_text = inline_wrap(block.text.trim(), block.bold, block.italic);
            if *ordered {
                let counter = ordered_counters.entry(*depth).or_insert(0);
                *counter += 1;
                result.push_str(&format!("{}{}. {}\n", indent, counter, item_text));
            } else {
                result.push_str(&format!("{}- {}\n", indent, item_text));
            }
        }
    }
    result
}

fn render_table(blocks: &[&Block]) -> String {
    let (grid, max_col) = build_table_grid(blocks);
    if grid.is_empty() {
        return String::new();
    }

    if let Some(rendered) = render_key_value_grid(&grid, max_col + 1) {
        return rendered;
    }

    render_table_grid(&grid, max_col + 1)
}

fn render_coordinate_table(table: &DetectedTable) -> String {
    match &table.render {
        TableRender::Markdown => render_detected_markdown_table(&table.rows),
        TableRender::Layout { text } => {
            let mut rendered = String::from("```text\n");
            rendered.push_str(text.trim_end());
            rendered.push_str("\n```\n");
            rendered
        }
    }
}

fn render_detected_markdown_table(rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return String::new();
    }
    let col_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if col_count == 0 {
        return String::new();
    }

    let mut rendered = String::new();
    for (row_idx, row) in rows.iter().enumerate() {
        rendered.push('|');
        for col in 0..col_count {
            let cell = row.get(col).map(String::as_str).unwrap_or("");
            rendered.push_str(&format!(" {} |", escape_table_cell(cell.trim())));
        }
        rendered.push('\n');
        if row_idx == 0 {
            rendered.push('|');
            for _ in 0..col_count {
                rendered.push_str(" --- |");
            }
            rendered.push('\n');
        }
    }
    rendered
}

fn build_table_grid(
    blocks: &[&Block],
) -> (
    std::collections::BTreeMap<usize, std::collections::BTreeMap<usize, String>>,
    usize,
) {
    use std::collections::BTreeMap;

    let mut grid: BTreeMap<usize, BTreeMap<usize, String>> = BTreeMap::new();
    let mut max_col = 0usize;

    for block in blocks {
        if let BlockKind::TableCell { row, col } = block.kind {
            grid.entry(row)
                .or_default()
                .insert(col, block.text.trim().to_string());
            max_col = max_col.max(col);
        }
    }

    (grid, max_col)
}

fn render_table_grid(
    grid: &std::collections::BTreeMap<usize, std::collections::BTreeMap<usize, String>>,
    col_count: usize,
) -> String {
    let mut result = String::new();
    let mut first_row = true;

    for row_cells in grid.values() {
        result.push('|');
        for col in 0..col_count {
            let cell = row_cells.get(&col).map(String::as_str).unwrap_or("");
            result.push_str(&format!(" {} |", cell));
        }
        result.push('\n');

        // Insert separator after header row
        if first_row {
            result.push('|');
            for _ in 0..col_count {
                result.push_str(" --- |");
            }
            result.push('\n');
            first_row = false;
        }
    }

    result
}

fn render_key_value_grid(
    grid: &std::collections::BTreeMap<usize, std::collections::BTreeMap<usize, String>>,
    col_count: usize,
) -> Option<String> {
    if col_count != 2 || grid.len() < 2 {
        return None;
    }

    let mut rendered = String::new();
    let mut pair_count = 0usize;

    for row_cells in grid.values() {
        let key = row_cells.get(&0).map(String::as_str).unwrap_or("").trim();
        let value = row_cells.get(&1).map(String::as_str).unwrap_or("").trim();

        if key.is_empty() || value.is_empty() || !looks_like_key_value_label(key) {
            return None;
        }

        rendered.push_str(&format!(
            "- {}: {}\n",
            normalize_field_label(key),
            normalize_field_value(value)
        ));
        pair_count += 1;
    }

    if pair_count >= 2 {
        rendered.push('\n');
        Some(rendered)
    } else {
        None
    }
}

fn render_structured_region(
    markdown: &mut String,
    blocks: &[&Block],
    start: usize,
    region: &StructuredRegion,
) {
    match &region.kind {
        StructuredRegionKind::TableCells => {
            markdown.push_str(&render_table(&blocks[start..region.next_index]));
            markdown.push('\n');
        }
        StructuredRegionKind::NumericTable {
            headers,
            rows,
            total,
        } => markdown.push_str(&render_inferred_numeric_table(
            headers,
            rows,
            total.as_deref(),
        )),
        StructuredRegionKind::FormFields { fields } => {
            markdown.push_str(&render_inferred_form_fields(fields));
        }
    }
}

fn render_inferred_numeric_table(
    headers: &[String],
    rows: &[ParsedNumericRow],
    total: Option<&str>,
) -> String {
    let mut markdown = String::new();
    markdown.push('|');
    for header in headers {
        markdown.push_str(&format!(" {} |", header));
    }
    markdown.push('\n');
    markdown.push('|');
    for _ in headers {
        markdown.push_str(" --- |");
    }
    markdown.push('\n');

    for parsed in rows {
        markdown.push('|');
        markdown.push_str(&format!(" {} |", escape_table_cell(&parsed.label)));
        for value in &parsed.values {
            markdown.push_str(&format!(" {} |", escape_table_cell(value)));
        }
        markdown.push('\n');
    }

    if let Some(total) = total {
        markdown.push('\n');
        markdown.push_str(&format!("Total: {}\n", total));
    }

    markdown.push('\n');
    markdown
}

fn render_inferred_form_fields(fields: &[FormField]) -> String {
    let mut markdown = String::new();
    for field in fields {
        markdown.push_str(&format!(
            "- {}: {}\n",
            field.label,
            field.value.as_deref().unwrap_or("________")
        ));
    }
    markdown.push('\n');
    markdown
}

fn append_rendered_block(markdown: &mut String, block: &Block, force_plain_text: bool) {
    match &block.kind {
        BlockKind::Heading { level } if !force_plain_text => {
            markdown.push_str(&render_heading(&block.text, *level));
        }
        BlockKind::Caption => {
            markdown.push('*');
            markdown.push_str(block.text.trim());
            markdown.push_str("*\n\n");
        }
        BlockKind::CodeBlock => {
            markdown.push_str("```\n");
            markdown.push_str(block.text.trim());
            markdown.push_str("\n```\n\n");
        }
        BlockKind::Image { path } => {
            if let Some(path) = path {
                markdown.push_str(&format!("![image]({})\n\n", path));
            }
        }
        BlockKind::Figure { path, caption } => {
            if let Some(path) = path {
                markdown.push_str(&format!("![image]({})\n\n", path));
            }
            if let Some(caption) = caption {
                markdown.push('*');
                markdown.push_str(caption.trim());
                markdown.push_str("*\n\n");
            }
        }
        BlockKind::Formula { latex, display } => {
            if *display {
                markdown.push_str(&format!("$$ {} $$\n\n", latex.trim()));
            } else {
                markdown.push_str(&format!("${}$\n\n", latex.trim()));
            }
        }
        BlockKind::FormulaReview { reason, crop_path } => {
            markdown.push_str(&format!(
                "<!-- formula-review: reason=\"{}\"",
                escape_comment_attr(reason)
            ));
            if let Some(path) = crop_path {
                markdown.push_str(&format!(" crop=\"{}\"", escape_comment_attr(path)));
            }
            markdown.push_str(" -->\n\n");
        }
        BlockKind::CoordinateTable { table } if !force_plain_text => {
            markdown.push_str(&render_coordinate_table(table));
            markdown.push('\n');
        }
        BlockKind::ListItem { ordered, depth } => {
            let indent = "  ".repeat(*depth as usize);
            let item_text = inline_wrap(block.text.trim(), block.bold, block.italic);
            if *ordered {
                markdown.push_str(&format!("{}1. {}\n\n", indent, item_text));
            } else {
                markdown.push_str(&format!("{}- {}\n\n", indent, item_text));
            }
        }
        BlockKind::PageNumber
        | BlockKind::RunningHeader
        | BlockKind::RunningFooter
        | BlockKind::Artifact => {}
        _ if force_plain_text => {
            let text = normalize_front_matter_text(block.text.trim());
            append_plain_text(markdown, &text);
        }
        _ => append_paragraph(
            markdown,
            &inline_wrap(
                &normalize_front_matter_text(block.text.trim()),
                block.bold,
                block.italic,
            ),
        ),
    }
}

/// Split markdown into sections using heading boundaries.
/// Falls back to a single section if no headings are found.
pub fn split_into_sections(markdown: &str, doc: &Document) -> Vec<Section> {
    let mut sections: Vec<Section> = Vec::new();
    let mut current_title = String::from("Document");
    let mut current_level = 1u8;
    let mut current_content = String::new();
    let mut current_page_start = 1usize;
    let mut current_page = 1usize;
    let mut found_heading = false;

    for line in markdown.lines() {
        // Track page transitions via embedded markers
        if let Some(page_num) = parse_page_marker(line) {
            current_page = page_num;
            // Don't add the marker to content
            continue;
        }

        if let Some((level, title)) = parse_heading_line(line) {
            // Flush previous section
            if found_heading || !current_content.trim().is_empty() {
                sections.push(Section {
                    title: current_title.clone(),
                    level: current_level,
                    content: current_content.trim().to_string(),
                    page_start: current_page_start,
                    page_end: current_page,
                });
            }
            current_title = title;
            current_level = level;
            current_content = String::new();
            current_page_start = current_page;
            found_heading = true;
        } else {
            current_content.push_str(line);
            current_content.push('\n');
        }
    }

    // Flush last section
    if found_heading || !current_content.trim().is_empty() {
        sections.push(Section {
            title: current_title,
            level: current_level,
            content: current_content.trim().to_string(),
            page_start: current_page_start,
            page_end: doc.pages.len(),
        });
    }

    if sections.is_empty() {
        sections.push(Section {
            title: String::from("Document"),
            level: 1,
            content: markdown.trim().to_string(),
            page_start: 1,
            page_end: doc.pages.len(),
        });
    }

    sections
}

/// Detect an embedded page boundary marker of the form `<!-- page:N -->`.
/// Returns the 1-indexed page number, or `None` if the line is not a marker.
fn parse_page_marker(line: &str) -> Option<usize> {
    let trimmed = line.trim();
    if trimmed.starts_with("<!-- page:") && trimmed.ends_with(" -->") {
        let inner = &trimmed["<!-- page:".len()..trimmed.len() - " -->".len()];
        inner.trim().parse::<usize>().ok()
    } else {
        None
    }
}

/// Parse a markdown heading line. Returns `(level, title)` or `None`.
fn parse_heading_line(line: &str) -> Option<(u8, String)> {
    if !line.starts_with('#') {
        return None;
    }
    let trimmed = line.trim_start_matches('#');
    let level = (line.len() - trimmed.len()) as u8;
    if (1..=6).contains(&level) {
        let title = trimmed.trim().to_string();
        if !title.is_empty() {
            return Some((level, title));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::{Bbox, Block, BlockKind, Document, DocumentMetadata, Page};
    use crate::layout::table_inference::{normalize_numeric_row, strip_leading_list_marker};
    use std::path::PathBuf;

    fn make_block(id: usize, text: &str, kind: BlockKind, reading_order: usize) -> Block {
        make_block_at(
            id,
            text,
            kind,
            Bbox::new(0.0, 0.0, 100.0, 20.0),
            12.0,
            reading_order,
        )
    }

    fn make_block_at(
        id: usize,
        text: &str,
        kind: BlockKind,
        bbox: Bbox,
        font_size: f32,
        reading_order: usize,
    ) -> Block {
        Block {
            override_markdown: None,
            id,
            bbox,
            text: text.to_string(),
            kind,
            font_size,
            font_name: "Helvetica".to_string(),
            page_num: 0,
            reading_order,
            bold: false,
            italic: false,
        }
    }

    fn make_doc(pages: Vec<Page>) -> Document {
        Document {
            source_path: PathBuf::from("test.pdf"),
            pages,
            metadata: DocumentMetadata::default(),
        }
    }

    fn make_page(page_num: usize, width: f32, height: f32, blocks: Vec<Block>) -> Page {
        Page {
            page_num,
            width,
            height,
            blocks,
            override_markdown: None,
        }
    }

    fn make_page_override(
        page_num: usize,
        width: f32,
        height: f32,
        blocks: Vec<Block>,
        override_md: &str,
    ) -> Page {
        Page {
            page_num,
            width,
            height,
            blocks,
            override_markdown: Some(override_md.to_string()),
        }
    }

    #[test]
    fn renders_heading_paragraph() {
        let page = make_page(
            0,
            595.0,
            842.0,
            vec![
                make_block(0, "Introduction", BlockKind::Heading { level: 1 }, 0),
                make_block(1, "Hello world.", BlockKind::Paragraph, 1),
            ],
        );
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(result.markdown.contains("# Introduction"));
        assert!(result.markdown.contains("Hello world."));
    }

    #[test]
    fn renders_formula_review_marker() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![make_block(
                0,
                "",
                BlockKind::FormulaReview {
                    reason: "visual-isolated-equation-band+cue:Hence:".into(),
                    crop_path: Some("debug/formulas/page1_formula1.png".into()),
                },
                0,
            )],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(result.markdown.contains("<!-- formula-review: page=1"));
        assert!(result.markdown.contains("visual-isolated-equation-band"));
        assert!(result
            .markdown
            .contains("crop=\"debug/formulas/page1_formula1.png\""));
        assert!(!result.markdown.contains("$$"));
    }

    #[test]
    fn renders_formula_with_latex_as_display_math() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![make_block(
                0,
                "F = ma",
                BlockKind::Formula {
                    latex: "F = ma".into(),
                    display: true,
                },
                0,
            )],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(
            result.markdown.contains("$$ F = ma $$"),
            "display formula should emit $$ ... $$, got: {}",
            result.markdown
        );
    }

    #[test]
    fn reflows_artificial_blank_lines_inside_paragraphs() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![make_block(
                0,
                "This is one\n\nparagraph with\n\nartificial breaks.",
                BlockKind::Paragraph,
                0,
            )],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(result
            .markdown
            .contains("This is one paragraph with artificial breaks."));
    }

    #[test]
    fn paragraph_reflow_preserves_mathish_subscript_and_superscript_markup() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![make_block(
                0,
                "For x^{2}\n+\ny_{i}, the term remains stable.",
                BlockKind::Paragraph,
                0,
            )],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(result
            .markdown
            .contains("For x^{2} + y_{i}, the term remains stable."));
    }

    #[test]
    fn skips_page_numbers_and_running_headers() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block(0, "1", BlockKind::PageNumber, 0),
                make_block(1, "Chapter 1", BlockKind::RunningHeader, 1),
                make_block(2, "Tagged artifact", BlockKind::Artifact, 2),
                make_block(3, "Body text.", BlockKind::Paragraph, 3),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(!result.markdown.contains("Chapter 1"));
        assert!(!result.markdown.contains("Tagged artifact"));
        assert!(result.markdown.contains("Body text."));
    }

    #[test]
    fn splits_sections_by_headings() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block(0, "Intro", BlockKind::Heading { level: 1 }, 0),
                make_block(1, "Some intro text.", BlockKind::Paragraph, 1),
                make_block(2, "Methods", BlockKind::Heading { level: 2 }, 2),
                make_block(3, "We did things.", BlockKind::Paragraph, 3),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert_eq!(result.sections.len(), 2);
        assert_eq!(result.sections[0].title, "Intro");
        assert_eq!(result.sections[0].level, 1);
        assert_eq!(result.sections[1].title, "Methods");
        assert_eq!(result.sections[1].level, 2);
    }

    #[test]
    fn renders_unordered_list() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block(
                    0,
                    "Apple",
                    BlockKind::ListItem {
                        ordered: false,
                        depth: 0,
                    },
                    0,
                ),
                make_block(
                    1,
                    "Banana",
                    BlockKind::ListItem {
                        ordered: false,
                        depth: 0,
                    },
                    1,
                ),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(result.markdown.contains("- Apple"));
        assert!(result.markdown.contains("- Banana"));
    }

    #[test]
    fn renders_table() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block(0, "Name", BlockKind::TableCell { row: 0, col: 0 }, 0),
                make_block(1, "Age", BlockKind::TableCell { row: 0, col: 1 }, 1),
                make_block(2, "Alice", BlockKind::TableCell { row: 1, col: 0 }, 2),
                make_block(3, "30", BlockKind::TableCell { row: 1, col: 1 }, 3),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(result.markdown.contains("| Name | Age |"));
        assert!(result.markdown.contains("| --- |"));
        assert!(result.markdown.contains("| Alice | 30 |"));
    }

    #[test]
    fn renders_key_value_table_as_field_list() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block(
                    0,
                    "Invoice Number",
                    BlockKind::TableCell { row: 0, col: 0 },
                    0,
                ),
                make_block(1, "2020-10", BlockKind::TableCell { row: 0, col: 1 }, 1),
                make_block(
                    2,
                    "Invoice Date",
                    BlockKind::TableCell { row: 1, col: 0 },
                    2,
                ),
                make_block(
                    3,
                    "January 8, 2020",
                    BlockKind::TableCell { row: 1, col: 1 },
                    3,
                ),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(result.markdown.contains("- Invoice Number: 2020-10"));
        assert!(result.markdown.contains("- Invoice Date: January 8, 2020"));
        assert!(!result.markdown.contains("| --- |"));
    }

    #[test]
    fn renders_implicit_invoice_rows_as_table() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block(
                    0,
                    "ITEM DESCRIPTION QUANTITY PRICE (€) AMOUNT (€)",
                    BlockKind::Paragraph,
                    0,
                ),
                make_block(1, "1 Super Kite 2 20.00 40.00", BlockKind::Paragraph, 1),
                make_block(2, "2 Turbo Flyer 5 40.00 200.00", BlockKind::Paragraph, 2),
                make_block(3, "3 Giga Trash 1 180.00 180.00", BlockKind::Paragraph, 3),
                make_block(4, "420.00", BlockKind::Paragraph, 4),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(result
            .markdown
            .contains("| Item | Quantity | Price | Amount |"));
        assert!(result
            .markdown
            .contains("| 1 Super Kite | 2 | 20.00 | 40.00 |"));
        assert!(result.markdown.contains("Total: 420.00"));
    }

    #[test]
    fn renders_form_labels_as_labeled_entries() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block(0, "Text, required:", BlockKind::Paragraph, 0),
                make_block(1, "Text, digits:", BlockKind::Paragraph, 1),
                make_block(2, "Check boxes:", BlockKind::Paragraph, 2),
                make_block(3, "A   B   C   D", BlockKind::Paragraph, 3),
                make_block(4, "Radio buttons:", BlockKind::Paragraph, 4),
                make_block(5, "yes   no", BlockKind::Paragraph, 5),
                make_block(6, "Drop-down:", BlockKind::Paragraph, 6),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(result.markdown.contains("- Text, required: ________"));
        assert!(result.markdown.contains("- Check boxes: A | B | C | D"));
        assert!(result.markdown.contains("- Radio buttons: yes | no"));
        assert!(result.markdown.contains("- Drop-down: ________"));
    }

    #[test]
    fn normalizes_zero_shorthand_rows_to_dominant_width() {
        let row = ParsedNumericRow {
            label: "11) Variazioni rimanenze materie prime".to_string(),
            values: vec!["0000".to_string()],
        };
        let normalized = normalize_numeric_row(row, 4);
        assert_eq!(normalized.values, vec!["0", "0", "0", "0"]);
    }

    #[test]
    fn parse_heading_line_valid() {
        assert_eq!(
            parse_heading_line("## Hello"),
            Some((2, "Hello".to_string()))
        );
        assert_eq!(parse_heading_line("# Top"), Some((1, "Top".to_string())));
        assert_eq!(parse_heading_line("Not a heading"), None);
        assert_eq!(parse_heading_line("##"), None); // no title after hashes
    }

    #[test]
    fn normalize_heading_text_inserts_missing_space_after_section_number() {
        assert_eq!(normalize_heading_text("1Introduction"), "1 Introduction");
        assert_eq!(normalize_heading_text("2Background"), "2 Background");
        assert_eq!(normalize_heading_text("3.2Subheading"), "3.2 Subheading");
        assert_eq!(
            normalize_heading_text("Appendix A"),
            "Appendix A",
            "non-numbered headings should be unchanged"
        );
    }

    #[test]
    fn strip_leading_list_marker_removes_simple_number_prefixes() {
        assert_eq!(
            strip_leading_list_marker("1. Revenue 120 220"),
            "Revenue 120 220"
        );
        assert_eq!(
            strip_leading_list_marker("12) Accantonamenti per rischi000240.000"),
            "Accantonamenti per rischi000240.000"
        );
        assert_eq!(
            strip_leading_list_marker("17-bis) Utili e perdite"),
            "17-bis) Utili e perdite",
            "non-simple labels should be preserved"
        );
    }

    #[test]
    fn scanned_page_warning_emitted() {
        // A page with no blocks should produce a WARNING comment
        let page = Page {
            page_num: 2, // 0-indexed → page 3 in output
            width: 595.0,
            height: 842.0,
            blocks: vec![],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(
            result.markdown.contains(
                "<!-- WARNING: page 3 has no extractable text (scanned image or empty page) -->"
            ),
            "Expected scanned page warning in markdown, got: {}",
            result.markdown
        );
        // Should also have the page marker
        assert!(result.markdown.contains("<!-- page:3 -->"));
    }

    #[test]
    fn no_warning_for_page_with_blocks() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![make_block(0, "Some text", BlockKind::Paragraph, 0)],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();
        assert!(
            !result.markdown.contains("WARNING"),
            "Should not have WARNING for page with blocks"
        );
    }

    #[test]
    fn scholarly_front_matter_demotes_metadata_and_author_tables() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block_at(
                    0,
                    "arXiv:1706.03762v7  [cs.CL]  2 Aug 2023",
                    BlockKind::Heading { level: 1 },
                    Bbox::new(50.0, 24.0, 545.0, 42.0),
                    16.0,
                    0,
                ),
                make_block_at(
                    1,
                    "Attention Is All You Need",
                    BlockKind::Heading { level: 2 },
                    Bbox::new(70.0, 60.0, 525.0, 90.0),
                    24.0,
                    1,
                ),
                make_block_at(
                    2,
                    "Ashish Vaswani\nGoogle Brain\navaswani@google.com",
                    BlockKind::TableCell { row: 0, col: 0 },
                    Bbox::new(60.0, 118.0, 220.0, 170.0),
                    11.0,
                    2,
                ),
                make_block_at(
                    3,
                    "Noam Shazeer\nGoogle Brain\nnoam@google.com",
                    BlockKind::TableCell { row: 0, col: 1 },
                    Bbox::new(250.0, 118.0, 410.0, 170.0),
                    11.0,
                    3,
                ),
                make_block_at(
                    4,
                    "Abstract",
                    BlockKind::Heading { level: 4 },
                    Bbox::new(80.0, 200.0, 220.0, 220.0),
                    14.0,
                    4,
                ),
                make_block_at(
                    5,
                    "The Transformer replaces recurrence with attention.",
                    BlockKind::Paragraph,
                    Bbox::new(60.0, 228.0, 520.0, 290.0),
                    11.0,
                    5,
                ),
                make_block_at(
                    6,
                    "1. Introduction",
                    BlockKind::Paragraph,
                    Bbox::new(60.0, 330.0, 250.0, 350.0),
                    13.0,
                    6,
                ),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(result.markdown.contains("# Attention Is All You Need"));
        assert!(result
            .markdown
            .contains("arXiv:1706.03762v7  [cs.CL]  2 Aug 2023"));
        assert!(!result.markdown.contains("# arXiv:1706.03762v7"));
        assert!(!result.markdown.contains("| --- |"));
        assert!(result.markdown.contains("Ashish Vaswani\nGoogle Brain"));
        assert!(result.markdown.contains("## Abstract"));
    }

    #[test]
    fn scholarly_front_matter_splits_inline_abstract_heading() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block_at(
                    0,
                    "Practical Attacks against Black-box Code Completion Engines",
                    BlockKind::Heading { level: 3 },
                    Bbox::new(40.0, 40.0, 555.0, 76.0),
                    23.0,
                    0,
                ),
                make_block_at(
                    1,
                    "Slobodan Jenko",
                    BlockKind::Paragraph,
                    Bbox::new(80.0, 96.0, 260.0, 116.0),
                    12.0,
                    1,
                ),
                make_block_at(
                    2,
                    "arXiv:2408.02509v1  [cs.CR]  5 Aug 2024",
                    BlockKind::Paragraph,
                    Bbox::new(100.0, 132.0, 470.0, 150.0),
                    11.0,
                    2,
                ),
                make_block_at(
                    3,
                    "Abstract—Modern code completion engines can be attacked.",
                    BlockKind::Paragraph,
                    Bbox::new(60.0, 192.0, 530.0, 236.0),
                    11.0,
                    3,
                ),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(result.markdown.contains("## Abstract"));
        assert!(result
            .markdown
            .contains("Modern code completion engines can be attacked."));
        assert!(!result.markdown.contains("Abstract—Modern"));
    }

    #[test]
    fn suppresses_repeated_edge_text_after_first_occurrence() {
        let make_page = |page_num: usize, body: &str| Page {
            page_num,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block_at(
                    page_num * 10,
                    "www.pdfa.org",
                    BlockKind::Paragraph,
                    Bbox::new(40.0, 10.0, 180.0, 28.0),
                    10.0,
                    0,
                ),
                make_block_at(
                    page_num * 10 + 1,
                    body,
                    BlockKind::Paragraph,
                    Bbox::new(60.0, 220.0, 520.0, 280.0),
                    12.0,
                    1,
                ),
            ],
            override_markdown: None,
        };

        let doc = make_doc(vec![
            make_page(0, "Page one body."),
            make_page(1, "Page two body."),
            make_page(2, "Page three body."),
        ]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert_eq!(result.markdown.matches("www.pdfa.org").count(), 1);
        assert!(result.markdown.contains("Page one body."));
        assert!(result.markdown.contains("Page two body."));
        assert!(result.markdown.contains("Page three body."));
    }

    #[test]
    fn scholarly_front_matter_drops_decorative_images_and_keeps_captioned_figure() {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![
                make_block_at(
                    0,
                    "Attention Is All You Need",
                    BlockKind::Heading { level: 1 },
                    Bbox::new(60.0, 40.0, 520.0, 82.0),
                    24.0,
                    0,
                ),
                make_block_at(
                    1,
                    "Ashish Vaswani",
                    BlockKind::Paragraph,
                    Bbox::new(80.0, 104.0, 260.0, 124.0),
                    12.0,
                    1,
                ),
                make_block_at(
                    2,
                    "",
                    BlockKind::Image {
                        path: Some("images/logo.png".to_string()),
                    },
                    Bbox::new(470.0, 24.0, 540.0, 72.0),
                    12.0,
                    2,
                ),
                make_block_at(
                    3,
                    "Abstract",
                    BlockKind::Heading { level: 2 },
                    Bbox::new(70.0, 170.0, 190.0, 194.0),
                    14.0,
                    3,
                ),
                make_block_at(
                    4,
                    "The Transformer replaces recurrence with attention.",
                    BlockKind::Paragraph,
                    Bbox::new(60.0, 210.0, 520.0, 270.0),
                    11.0,
                    4,
                ),
                make_block_at(
                    5,
                    "",
                    BlockKind::Figure {
                        path: Some("images/figure.png".to_string()),
                        caption: Some("Figure 1: Model overview".to_string()),
                    },
                    Bbox::new(110.0, 300.0, 470.0, 520.0),
                    12.0,
                    5,
                ),
                make_block_at(
                    6,
                    "1. Introduction",
                    BlockKind::Paragraph,
                    Bbox::new(60.0, 560.0, 240.0, 582.0),
                    13.0,
                    6,
                ),
            ],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        let result = renderer.render_document(&doc).unwrap();

        assert!(!result.markdown.contains("images/logo.png"));
        assert!(result.markdown.contains("images/figure.png"));
        assert!(result.markdown.contains("*Figure 1: Model overview*"));
    }

    fn make_flagged_para(text: &str, bold: bool, italic: bool) -> Block {
        let mut block = make_block(0, text, BlockKind::Paragraph, 0);
        block.bold = bold;
        block.italic = italic;
        block
    }

    fn render_single_block(block: Block) -> String {
        let page = Page {
            page_num: 0,
            width: 595.0,
            height: 842.0,
            blocks: vec![block],
            override_markdown: None,
        };
        let doc = make_doc(vec![page]);
        let renderer = MarkdownRenderer::new(false, None);
        renderer.render_document(&doc).unwrap().markdown
    }

    #[test]
    fn bold_paragraph_renders_with_double_asterisks() {
        let md = render_single_block(make_flagged_para("Important note.", true, false));
        assert!(
            md.contains("**Important note.**"),
            "bold paragraph must be wrapped in **…**; got: {md}"
        );
    }

    #[test]
    fn italic_paragraph_renders_with_single_asterisks() {
        let md = render_single_block(make_flagged_para("Side note.", false, true));
        assert!(
            md.contains("*Side note.*") && !md.contains("**"),
            "italic paragraph must be wrapped in *…*; got: {md}"
        );
    }

    #[test]
    fn bold_italic_paragraph_renders_with_triple_asterisks() {
        let md = render_single_block(make_flagged_para("Very important.", true, true));
        assert!(
            md.contains("***Very important.***"),
            "bold+italic must be wrapped in ***…***; got: {md}"
        );
    }

    #[test]
    fn plain_paragraph_unchanged_by_inline_wrap() {
        let md = render_single_block(make_flagged_para("Normal text.", false, false));
        assert!(
            md.contains("Normal text.") && !md.contains('*'),
            "plain paragraph must not gain asterisks; got: {md}"
        );
    }

    #[test]
    fn bold_list_item_renders_with_markers_inside_bullet() {
        let mut block = make_block(
            0,
            "Bold bullet",
            BlockKind::ListItem {
                ordered: false,
                depth: 0,
            },
            0,
        );
        block.bold = true;
        let md = render_single_block(block);
        assert!(
            md.contains("- **Bold bullet**"),
            "bold list item must place markers after the bullet glyph; got: {md}"
        );
    }
}
