use pdf_processor::document::types::{Bbox, RawPage, RawWord};
use pdf_processor::formula::detect::detect_formula_candidates;
use pdf_processor::layout::furniture::detect_furniture_bboxes;

fn word(text: &str, x0: f32, y0: f32, x1: f32, y1: f32) -> RawWord {
    RawWord {
        text: text.to_string(),
        bbox: Bbox::new(x0, y0, x1, y1),
        font_size: 9.0,
        page_num: 0,
        block_id: 0,
        line_id: 0,
        baseline_y: y1,
    }
}

fn make_page(page_num: usize, footer_text: &[&str], body_text: &[&str], height: f32) -> RawPage {
    let mut words = Vec::new();

    for (idx, text) in body_text.iter().enumerate() {
        words.push(word(
            text,
            50.0,
            50.0 + idx as f32 * 15.0,
            200.0,
            62.0 + idx as f32 * 15.0,
        ));
    }

    let footer_y = height - 20.0;
    let mut x = 50.0;
    for text in footer_text {
        words.push(word(text, x, footer_y, x + 60.0, footer_y + 12.0));
        x += 65.0;
    }

    RawPage {
        page_num,
        width: 595.0,
        height,
        blocks: Vec::new(),
        words,
        image_refs: Vec::new(),
    }
}

#[test]
fn repeated_footer_detected_as_furniture() {
    let footer = &["Downloaded", "by", "ACME", "Corp"];
    let pages: Vec<_> = (0..5)
        .map(|idx| make_page(idx, footer, &["Body", "text", "here"], 842.0))
        .collect();

    let mask = detect_furniture_bboxes(&pages);

    for page_num in 0..5usize {
        assert!(
            mask.get(&page_num).is_some_and(|bboxes| !bboxes.is_empty()),
            "page {page_num} should have furniture entries"
        );
    }
}

#[test]
fn unique_page_text_not_marked_furniture() {
    let pages: Vec<_> = (0..5)
        .map(|idx| {
            let unique = format!("Unique content page {idx}");
            make_page(idx, &[], &[&unique], 842.0)
        })
        .collect();

    let mask = detect_furniture_bboxes(&pages);

    for page_num in 0..5usize {
        let count = mask.get(&page_num).map(Vec::len).unwrap_or(0);
        assert_eq!(
            count, 0,
            "unique body text should not be marked furniture on page {page_num}"
        );
    }
}

#[test]
fn single_page_document_produces_empty_mask() {
    let pages = vec![make_page(0, &["footer", "text"], &["body"], 842.0)];

    let mask = detect_furniture_bboxes(&pages);

    assert!(mask.get(&0).is_none_or(Vec::is_empty));
}

#[test]
fn mask_keys_use_raw_page_numbers() {
    let footer = &["Downloaded", "by", "ACME", "Corp"];
    let pages = vec![
        make_page(7, footer, &["Body", "text", "here"], 842.0),
        make_page(8, footer, &["More", "body", "text"], 842.0),
    ];

    let mask = detect_furniture_bboxes(&pages);

    assert!(mask.contains_key(&7));
    assert!(mask.contains_key(&8));
    assert!(!mask.contains_key(&0));
    assert!(!mask.contains_key(&1));
}

#[test]
fn watermark_words_not_flagged_as_formulas_when_excluded() {
    let height = 842.0_f32;
    let footer_y = height - 15.0;
    let page = RawPage {
        page_num: 0,
        width: 595.0,
        height,
        blocks: Vec::new(),
        image_refs: Vec::new(),
        words: vec![
            word("Downloaded", 50.0, footer_y, 130.0, footer_y + 10.0),
            word("by", 132.0, footer_y, 150.0, footer_y + 10.0),
            word("ACME", 152.0, footer_y, 195.0, footer_y + 10.0),
            word("Corp", 197.0, footer_y, 230.0, footer_y + 10.0),
            word("on", 232.0, footer_y, 248.0, footer_y + 10.0),
            word("2024-01-15", 250.0, footer_y, 320.0, footer_y + 10.0),
        ],
    };
    let furniture_bboxes = vec![Bbox::new(0.0, footer_y - 5.0, 595.0, height)];

    let candidates = detect_formula_candidates(&page, &furniture_bboxes);

    assert!(
        candidates.is_empty(),
        "watermark line should be suppressed by furniture mask"
    );
}

#[test]
fn excluded_watermark_word_is_removed_before_formula_line_grouping() {
    let page = RawPage {
        page_num: 0,
        width: 595.0,
        height: 842.0,
        blocks: Vec::new(),
        image_refs: Vec::new(),
        words: vec![
            word("F", 220.0, 300.0, 230.0, 312.0),
            word("=", 235.0, 300.0, 245.0, 312.0),
            word("ma", 250.0, 300.0, 270.0, 312.0),
            word("Downloaded", 540.0, 300.0, 590.0, 312.0),
        ],
    };
    let furniture_bboxes = vec![Bbox::new(535.0, 295.0, 595.0, 317.0)];

    let candidates = detect_formula_candidates(&page, &furniture_bboxes);

    assert!(!candidates.is_empty(), "real formula should remain");
    assert!(
        candidates
            .iter()
            .all(|candidate| !candidate.source_text.contains("Downloaded")),
        "furniture word should be excluded before line grouping: {candidates:#?}"
    );
}

#[test]
fn repeated_side_watermark_word_detected_as_furniture() {
    let pages: Vec<_> = (0..5)
        .map(|page_num| {
            let mut page = make_page(page_num, &[], &["Body", "formula", "F", "="], 842.0);
            page.words.push(word(
                "Downloaded",
                540.0,
                120.0 + page_num as f32 * 25.0,
                590.0,
                132.0 + page_num as f32 * 25.0,
            ));
            page
        })
        .collect();

    let mask = detect_furniture_bboxes(&pages);

    for page_num in 0..5usize {
        assert!(
            mask.get(&page_num).is_some_and(|bboxes| {
                bboxes
                    .iter()
                    .any(|bbox| bbox.x0 >= 530.0 && bbox.x1 >= 590.0)
            }),
            "page {page_num} should mark repeated right-side watermark word"
        );
    }
}
