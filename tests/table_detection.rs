use pdf_processor::document::types::{Bbox, RawWord};
use pdf_processor::layout::drawing_ops::{extract_lines, HLine, VLine};
use pdf_processor::layout::table_detector::detect_table_regions;

fn word_at_text(text: &str, x0: f32, y0: f32, x1: f32, y1: f32) -> RawWord {
    RawWord {
        bbox: Bbox::new(x0, y0, x1, y1),
        text: text.to_string(),
        font_size: 10.0,
        page_num: 0,
        block_id: 0,
        line_id: 0,
        baseline_y: y1 - 2.0,
    }
}

fn word_at(x0: f32, y0: f32, x1: f32, y1: f32) -> RawWord {
    word_at_text("cell", x0, y0, x1, y1)
}

#[test]
fn hline_struct_has_expected_fields() {
    let h = HLine {
        x0: 50.0,
        x1: 400.0,
        y: 200.0,
        thickness: 1.0,
    };

    assert!(h.length() > 300.0);
    assert!(h.is_significant());
}

#[test]
fn vline_struct_has_expected_fields() {
    let v = VLine {
        x: 100.0,
        y0: 150.0,
        y1: 400.0,
        thickness: 1.0,
    };

    assert!(v.length() > 200.0);
    assert!(v.is_significant());
}

#[test]
fn detects_table_between_two_hlines() {
    let hlines = vec![
        HLine {
            x0: 50.0,
            x1: 450.0,
            y: 200.0,
            thickness: 1.0,
        },
        HLine {
            x0: 50.0,
            x1: 450.0,
            y: 240.0,
            thickness: 1.0,
        },
    ];
    let words = vec![
        word_at(60.0, 205.0, 120.0, 218.0),
        word_at(200.0, 205.0, 260.0, 218.0),
        word_at(350.0, 205.0, 420.0, 218.0),
    ];

    let regions = detect_table_regions(&hlines, &[], &words, 595.0, 842.0);

    assert!(
        !regions.is_empty(),
        "two hlines bounding text should produce a table region"
    );
    let r = &regions[0];
    assert!(
        r.y0 <= 200.0 && r.y1 >= 240.0,
        "region should span the two hlines"
    );
}

#[test]
fn no_table_when_hlines_are_isolated_rules() {
    let hlines = vec![HLine {
        x0: 50.0,
        x1: 450.0,
        y: 100.0,
        thickness: 0.5,
    }];

    let regions = detect_table_regions(&hlines, &[], &[], 595.0, 842.0);

    assert!(
        regions.is_empty(),
        "single rule with no bounded text is not a table"
    );
}

#[test]
fn whitespace_inference_detects_column_aligned_numbers() {
    let words = vec![
        word_at_text("10", 50.0, 300.0, 90.0, 312.0),
        word_at_text("20", 180.0, 300.0, 220.0, 312.0),
        word_at_text("30", 320.0, 300.0, 360.0, 312.0),
        word_at_text("11", 50.0, 316.0, 90.0, 328.0),
        word_at_text("21", 180.0, 316.0, 220.0, 328.0),
        word_at_text("31", 320.0, 316.0, 360.0, 328.0),
        word_at_text("12", 50.0, 332.0, 90.0, 344.0),
        word_at_text("22", 180.0, 332.0, 220.0, 344.0),
        word_at_text("32", 320.0, 332.0, 360.0, 344.0),
    ];

    let regions = detect_table_regions(&[], &[], &words, 595.0, 842.0);

    assert!(
        !regions.is_empty(),
        "3x3 column-aligned text should be detected as table"
    );
}

#[test]
#[ignore = "requires DNV PDF"]
fn dnv_page_69_produces_table_candidates() {
    use mupdf::Document;
    use pdf_processor::pdf::extractor::PdfExtractor;
    use std::path::Path;

    let pdf_path = Path::new(
        "/home/eastill/projects/literature/standards/pdfs/best/07 - DNV-ST-N001_2018 - Marine operations and marine warranty.pdf",
    );
    let (raw_pages, _) = PdfExtractor::extract(pdf_path).unwrap();
    let raw_page = raw_pages
        .iter()
        .find(|p| p.page_num == 68)
        .expect("DNV page 69 should exist");

    let doc = Document::open(pdf_path).unwrap();
    let page = doc.load_page(68).unwrap();
    let (hlines, vlines) = extract_lines(&page, raw_page.width, raw_page.height).unwrap();
    let regions = detect_table_regions(
        &hlines,
        &vlines,
        &raw_page.words,
        raw_page.width,
        raw_page.height,
    );

    assert!(
        !regions.is_empty(),
        "DNV page 69 should produce at least one table region, got 0"
    );
}
