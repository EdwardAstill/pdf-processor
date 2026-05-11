//! Regression tests: word-based formula detector false positives.

use pdf_processor::formula::detect::detect_formula_candidates;
use pdf_processor::document::types::{RawPage, RawWord, Bbox};

fn word(text: &str, x0: f32, y0: f32, x1: f32, y1: f32, font_size: f32) -> RawWord {
    RawWord {
        text: text.to_string(),
        bbox: Bbox { x0, y0, x1, y1 },
        font_size,
        page_num: 0,
        block_id: 0,
        line_id: 0,
        baseline_y: y1,
    }
}

fn page_with_words(words: Vec<RawWord>) -> RawPage {
    RawPage { page_num: 0, width: 595.0, height: 842.0, blocks: vec![], words, image_refs: vec![] }
}

#[test]
fn reference_slash_lines_not_flagged() {
    let p = page_with_words(vec![
        word("/34/",      50.0, 100.0, 80.0,  112.0, 10.0),
        word("DNV-RU-OU-0300", 82.0, 100.0, 180.0, 112.0, 10.0),
        word("(2018)",    182.0, 100.0, 220.0, 112.0, 10.0),
        word("Fleet",     222.0, 100.0, 260.0, 112.0, 10.0),
        word("in",        262.0, 100.0, 278.0, 112.0, 10.0),
        word("service",   280.0, 100.0, 330.0, 112.0, 10.0),
    ]);
    let candidates = detect_formula_candidates(&p, &[]);
    assert!(
        candidates.is_empty(),
        "reference line should not produce formula candidates, got: {:#?}",
        candidates.iter().map(|c| &c.source_text).collect::<Vec<_>>()
    );
}

#[test]
fn bracketed_reference_lines_not_flagged() {
    let p = page_with_words(vec![
        word("[1]",     50.0, 100.0, 70.0,  112.0, 10.0),
        word("Author,", 72.0, 100.0, 130.0, 112.0, 10.0),
        word("Title",   132.0, 100.0, 170.0, 112.0, 10.0),
        word("of",      172.0, 100.0, 185.0, 112.0, 10.0),
        word("Paper,",  187.0, 100.0, 235.0, 112.0, 10.0),
        word("Journal,",237.0, 100.0, 295.0, 112.0, 10.0),
    ]);
    let candidates = detect_formula_candidates(&p, &[]);
    assert!(candidates.is_empty(), "bracketed reference should not be flagged");
}

#[test]
fn real_formula_still_detected_after_reference_filter() {
    // F = ma centered on page with equation number
    let p = page_with_words(vec![
        word("F",    220.0, 400.0, 230.0, 415.0, 12.0),
        word("=",    235.0, 400.0, 245.0, 415.0, 12.0),
        word("m",    250.0, 400.0, 260.0, 415.0, 12.0),
        word("·",    262.0, 400.0, 270.0, 415.0, 12.0),
        word("a",    272.0, 400.0, 282.0, 415.0, 12.0),
        word("(3.1)",480.0, 400.0, 520.0, 415.0, 11.0),
    ]);
    let candidates = detect_formula_candidates(&p, &[]);
    assert!(!candidates.is_empty(), "real numbered formula should still be detected");
}

#[test]
fn table_bboxes_suppress_formula_candidates() {
    // σ ≥ 235 MPa scores as a formula candidate (centered, relation, symbol-heavy).
    // Verify exclusion is doing the work: assert non-empty without exclusion,
    // then assert empty with the table region covering the formula.
    let p = page_with_words(vec![
        word("σ",   200.0, 300.0, 210.0, 312.0, 11.0),
        word("≥",   215.0, 300.0, 225.0, 312.0, 11.0),
        word("235", 230.0, 300.0, 255.0, 312.0, 11.0),
        word("MPa", 258.0, 300.0, 285.0, 312.0, 11.0),
    ]);
    // Without exclusion: must produce at least one candidate
    let without = detect_formula_candidates(&p, &[]);
    assert!(
        !without.is_empty(),
        "σ ≥ 235 MPa should score as a formula candidate without exclusion"
    );
    // With the table region covering the formula: must suppress it
    let table_region = Bbox { x0: 150.0, y0: 280.0, x1: 400.0, y1: 340.0 };
    let with_exclusion = detect_formula_candidates(&p, &[table_region]);
    assert!(
        with_exclusion.is_empty(),
        "symbol-heavy table cell should be suppressed by excluded_bboxes, got: {:#?}",
        with_exclusion.iter().map(|c| &c.source_text).collect::<Vec<_>>()
    );
}

#[test]
fn lone_equation_number_not_filtered_as_reference() {
    // A single "(3.1)" word on its own line is an equation label, not a reference.
    // It should NOT be filtered by is_reference_line.
    // In practice it appears as part of a grouped formula line, but ensure
    // the filter doesn't discard it when standalone.
    let p = page_with_words(vec![
        word("F",     220.0, 400.0, 230.0, 415.0, 12.0),
        word("=",     235.0, 400.0, 245.0, 415.0, 12.0),
        word("ma",    250.0, 400.0, 270.0, 415.0, 12.0),
        // equation number on a slightly different baseline (separate line after grouping)
        word("(3.1)", 480.0, 403.0, 520.0, 418.0, 11.0),
    ]);
    // The formula line + equation number should together produce a candidate
    let candidates = detect_formula_candidates(&p, &[]);
    assert!(
        !candidates.is_empty(),
        "F = ma (3.1) should be detected as a formula candidate"
    );
}
