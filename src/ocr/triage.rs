use serde::Serialize;

use crate::document::types::RawPage;

const MIN_TEXT_AREA_FRACTION: f32 = 0.02;
const REPLACEMENT_CHAR_THRESHOLD: usize = 8;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct OcrTriageReport {
    pub pages_total: usize,
    pub pages_with_readable_text: usize,
    pub image_only_pages: usize,
    pub low_density_pages: usize,
    pub suspicious_replacement_chars: usize,
    pub pages_needing_ocr: Vec<usize>,
}

pub fn triage_raw_pages(pages: &[RawPage]) -> OcrTriageReport {
    let mut report = OcrTriageReport {
        pages_total: pages.len(),
        pages_with_readable_text: 0,
        image_only_pages: 0,
        low_density_pages: 0,
        suspicious_replacement_chars: 0,
        pages_needing_ocr: Vec::new(),
    };

    for page in pages {
        let readable_text = page
            .blocks
            .iter()
            .any(|block| !block.text.trim().is_empty());
        let image_only = !readable_text && !page.image_refs.is_empty();
        let page_area = (page.width * page.height).max(1.0);
        let text_area: f32 = page.blocks.iter().map(|block| block.bbox.area()).sum();
        let text_area_fraction = text_area / page_area;
        let low_density = page.blocks.is_empty() || text_area_fraction < MIN_TEXT_AREA_FRACTION;
        let replacement_chars = page
            .blocks
            .iter()
            .map(|block| block.text.matches('\u{fffd}').count())
            .sum::<usize>();

        if readable_text {
            report.pages_with_readable_text += 1;
        }
        if image_only {
            report.image_only_pages += 1;
        }
        if low_density {
            report.low_density_pages += 1;
        }
        report.suspicious_replacement_chars += replacement_chars;

        if image_only
            || (!readable_text && low_density)
            || replacement_chars >= REPLACEMENT_CHAR_THRESHOLD
        {
            report.pages_needing_ocr.push(page.page_num + 1);
        }
    }

    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::types::{Bbox, RawTextBlock};

    fn raw_page(blocks: Vec<RawTextBlock>, image_count: usize) -> RawPage {
        RawPage {
            page_num: 0,
            width: 100.0,
            height: 100.0,
            blocks,
            words: Vec::new(),
            image_refs: (0..image_count)
                .map(|image_index| crate::document::types::ImageRef {
                    page_num: 0,
                    bbox: Bbox::new(0.0, 0.0, 100.0, 100.0),
                    image_index,
                    bytes: Vec::new(),
                    format: "png".to_string(),
                })
                .collect(),
        }
    }

    fn text_block(text: &str, bbox: Bbox) -> RawTextBlock {
        RawTextBlock {
            bbox,
            text: text.to_string(),
            font_size: 12.0,
            font_name: "Times".to_string(),
            page_num: 0,
            block_id: 0,
            reading_order: 0,
        }
    }

    #[test]
    fn image_only_page_needs_ocr() {
        let report = triage_raw_pages(&[raw_page(Vec::new(), 1)]);
        assert_eq!(report.image_only_pages, 1);
        assert_eq!(report.pages_needing_ocr, vec![1]);
    }

    #[test]
    fn readable_text_page_does_not_need_ocr() {
        let report = triage_raw_pages(&[raw_page(
            vec![text_block("hello world", Bbox::new(0.0, 0.0, 80.0, 40.0))],
            0,
        )]);
        assert_eq!(report.pages_with_readable_text, 1);
        assert!(report.pages_needing_ocr.is_empty());
    }

    #[test]
    fn many_replacement_chars_need_ocr() {
        let report = triage_raw_pages(&[raw_page(
            vec![text_block("��������", Bbox::new(0.0, 0.0, 80.0, 40.0))],
            0,
        )]);
        assert_eq!(report.suspicious_replacement_chars, 8);
        assert_eq!(report.pages_needing_ocr, vec![1]);
    }
}
