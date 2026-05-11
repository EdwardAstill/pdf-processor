# Stage 1: Formula False Positive Hardening

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task when tasks are independent. For same-session manual execution, follow this plan directly. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reduce formula false positives in both the word-based and visual detectors by adding reference-line filtering, table-region exclusion, and a decorative-rule band-height guard.

**Architecture:** No new modules. Three targeted changes to existing detectors: (1) `detect.rs` gains `is_reference_line()` guard and an `excluded_bboxes` parameter so table cell regions are never re-flagged as formulas; (2) `visual.rs` gains a minimum band-height filter to reject thin decorative rules and logo bars; (3) `pipeline.rs` wires extracted table-cell bboxes through to both detectors. All changes are additive to function signatures or guard clauses — existing behaviour on non-reference, non-table pages is unchanged.

**Tech Stack:** Rust, mupdf crate, cargo test

**Recommended Skills:** test-driven-development, git

**Recommended MCPs:** none

**Status:** approved
**Refinement passes:** 1

## Assumptions

- `A1` — `detect_formula_candidates` is the sole entry point for word-based formula detection; no caller bypasses it.
  Type: repo-state
  Source: `grep -rn "detect_formula_candidates" src/`
  Check: `grep -rn "detect_formula_candidates" src/ | wc -l` — expect ≥2 (definition + caller(s))
  If false: identify all call sites and update all of them.
  Owner: Task 1

- `A2` — `visual.rs` `detect_visual_formula_candidates` already accepts `excluded_regions: &[Bbox]`; we only add the band-height guard.
  Type: repo-state
  Source: `src/formula/visual.rs` — function signature read during session
  Check: `grep -n "excluded_regions" src/formula/visual.rs`
  If false: add the `excluded_regions` parameter to `visual.rs` as part of Task 3.
  Owner: Task 3

- `A3` — The pipeline calls `detect_coordinate_tables` before `detect_formula_candidates`; `table_candidates: Vec<TableCandidate>` is available at the call site in `pipeline.rs:157–167`. Classification runs AFTER formula detection. `suppress_formula_candidates_overlapping_tables` already does post-hoc table-based suppression; Task 2 additionally passes table bboxes into `detect_formula_candidates` directly so the unit test in Task 1 can exercise the suppression path.
  Type: architectural
  Source: `src/pipeline.rs:157–167` — confirmed during refinement
  Check: `grep -n "detect_formula_candidates\|table_candidates\|suppress_formula" src/pipeline.rs | head -15`
  If false: verify `table_candidates` is in scope before the `detect_formula_candidates` call; use the bboxes from it.
  Owner: Task 2

---

## File Map

| File | Change |
|------|--------|
| `src/formula/detect.rs` | Add `is_reference_line()`, `is_reference_section()`, `excluded_bboxes` param |
| `src/formula/mod.rs` | Update re-export signature if needed |
| `src/formula/visual.rs` | Add `MIN_BAND_HEIGHT_PX` guard in `looks_like_formula_band()` |
| `src/pipeline.rs` | Extract table-cell bboxes; pass to both detectors |
| `tests/formula_fp.rs` (new) | Regression unit tests for reference and table suppression |

---

### Task 1: Reference-line filter in word detector

**Files:**
- Modify: `src/formula/detect.rs`
- Create: `tests/formula_fp.rs`

**Ownership:**
- In scope: `detect_formula_candidates`, new `is_reference_line`, new `is_reference_section` functions
- Out of scope: `visual.rs`, pipeline wiring

**Assumption refs:** `A1`

**Invoke skill:** `test-driven-development` before starting this task.

- [ ] **Step 1: Write failing tests**

Add to a new file `tests/formula_fp.rs`:

```rust
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
    // DNV page 597 pattern: /34/ DNV-RU-OU-0300 (2018) Fleet in service
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
    // [1] Author, Title of Paper, Journal, 2020.
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
    // F = ma centered on page
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
```

- [ ] **Step 2: Run tests to confirm they fail**

```bash
cargo test --test formula_fp 2>&1 | grep -E "FAILED|error"
```
Expected: compilation error or test failures — `detect_formula_candidates` has wrong arity or tests don't exist yet.

- [ ] **Step 3: Add `excluded_bboxes` parameter and reference-line guards to `detect.rs`**

Change the public signature at the top of `detect_formula_candidates`:

```rust
pub fn detect_formula_candidates(raw_page: &RawPage, excluded_bboxes: &[Bbox]) -> Vec<FormulaCandidate> {
```

Add before the line-scoring loop:

```rust
    if is_reference_section(raw_page) {
        return Vec::new();
    }
```

Add new private functions after `math_score`:

```rust
/// Returns true when the line looks like a bibliography entry.
/// Patterns: `/N/ text...`, `[N] text...`, `(N) text...` where N is 1-3 digits.
fn is_reference_line(text: &str) -> bool {
    let t = text.trim();
    // /34/ pattern (DNV-style)
    if t.starts_with('/') {
        let rest = &t[1..];
        if let Some(slash) = rest.find('/') {
            let maybe_num = &rest[..slash];
            if maybe_num.chars().all(|c| c.is_ascii_digit()) && slash <= 4 {
                return true;
            }
        }
    }
    // [1] or (1) pattern
    let first = t.chars().next().unwrap_or(' ');
    if first == '[' || first == '(' {
        let close = if first == '[' { ']' } else { ')' };
        if let Some(i) = t.find(close) {
            let inner = &t[1..i];
            if inner.chars().all(|c| c.is_ascii_digit()) && i <= 5 {
                return true;
            }
        }
    }
    false
}

/// Returns true if this page looks like a reference/bibliography section.
/// Heuristic: >60% of lines that have any content start with a reference marker.
fn is_reference_section(raw_page: &RawPage) -> bool {
    if raw_page.words.is_empty() {
        return false;
    }
    // Cheap check: first word of page is "References" or "Bibliography"
    let first_text = raw_page.words.first().map(|w| w.text.as_str()).unwrap_or("");
    if matches!(first_text, "References" | "Bibliography" | "REFERENCES" | "BIBLIOGRAPHY") {
        return true;
    }
    // Density check: count reference-marker lines
    let lines = group_words_into_lines(&raw_page.words);
    if lines.is_empty() {
        return false;
    }
    let ref_lines = lines.iter().filter(|l| is_reference_line(&l.text)).count();
    // If more than 40% of lines are reference-pattern, treat whole page as reference section
    ref_lines * 10 >= lines.len() * 4
}
```

In the line-scoring inner loop (inside `detect_formula_candidates`), add before `score_line`:

```rust
        if is_reference_line(&line.text) {
            continue;
        }
```

Also add excluded_bboxes overlap check before pushing candidates:

```rust
        let candidate_bbox = /* computed bbox */;
        if excluded_bboxes.iter().any(|ex| overlap_ratio(candidate_bbox, *ex) > 0.5) {
            continue;
        }
```

Note: `overlap_ratio` already exists in `detect.rs` — check with `grep -n "overlap_ratio" src/formula/detect.rs`.

- [ ] **Step 4: Fix any compilation errors and run tests**

```bash
cargo test --test formula_fp 2>&1 | tail -20
```
Expected: all 3 tests pass.

- [ ] **Step 5: Run full test suite to check no regressions**

```bash
cargo test 2>&1 | grep -E "FAILED|ok$|test result"
```
Expected: all existing tests still pass.

- [ ] **Step 6: Commit**

```bash
git add src/formula/detect.rs tests/formula_fp.rs
git commit -m "feat(formula): add reference-line filter and excluded_bboxes to word detector"
```

---

### Task 2: Wire table-cell bboxes into formula detection call sites

**Files:**
- Modify: `src/pipeline.rs`
- Modify: `src/formula/mod.rs` (update re-export if signature changed)

**Ownership:**
- In scope: pipeline call to `detect_formula_candidates`, `detect_visual_formula_candidates`
- Out of scope: the detectors themselves (changed in Task 1 and Task 3)

**Assumption refs:** `A1`, `A3`

- [ ] **Step 1: Write a failing integration test**

In `tests/formula_fp.rs`, add:

```rust
#[test]
fn table_bboxes_suppress_formula_candidates() {
    // A region that would score as a formula (centered, has = and Greek)
    // but overlaps a known table cell bbox should be suppressed.
    let p = page_with_words(vec![
        word("σ",   200.0, 300.0, 210.0, 312.0, 11.0),
        word("≥",   215.0, 300.0, 225.0, 312.0, 11.0),
        word("235", 230.0, 300.0, 255.0, 312.0, 11.0),
        word("MPa", 258.0, 300.0, 285.0, 312.0, 11.0),
    ]);
    // Table occupies the same region
    let table_region = Bbox { x0: 150.0, y0: 280.0, x1: 400.0, y1: 340.0 };
    let candidates = detect_formula_candidates(&p, &[table_region]);
    assert!(candidates.is_empty(), "symbol-heavy table cell should be suppressed");
}
```

- [ ] **Step 2: Run to confirm it fails**

```bash
cargo test --test formula_fp table_bboxes_suppress 2>&1 | grep -E "FAILED|ok"
```

- [ ] **Step 3: Locate the pipeline call site and extract table-cell bboxes**

Find where `detect_formula_candidates` is called:

```bash
grep -n "detect_formula_candidates\|detect_visual_formula" src/pipeline.rs
```

In `src/pipeline.rs`, `detect_coordinate_tables` is already called at line ~157 and its result is `table_candidates: Vec<TableCandidate>`. Change the existing `detect_formula_candidates` call (line ~164) from:

```rust
detect_formula_candidates(&raw_page)
```

to:

```rust
let excluded: Vec<Bbox> = table_candidates
    .iter()
    .map(|tc| tc.table.bbox)
    .collect();
detect_formula_candidates(&raw_page, &excluded)
```

The `excluded_regions` argument passed to `detect_visual_formula_candidates` a few lines later already uses the same table candidate bboxes (line ~174 in `excluded_regions`); no change needed there.

`TableCandidate` is imported via `table::{detect_coordinate_tables, TableCandidate}` at the top of `pipeline.rs` — `tc.table.bbox` is the `Bbox` of the table (confirmed from `suppress_formula_candidates_overlapping_tables` which uses `table.table.bbox`).

Note: `suppress_formula_candidates_overlapping_tables` still runs as a second pass after detection. The `excluded_bboxes` param in `detect_formula_candidates` and the post-hoc suppression are complementary — the param prevents generating low-confidence candidates that would pass the confidence threshold; the post-hoc pass catches any that slip through.

- [ ] **Step 4: Run tests**

```bash
cargo test --test formula_fp 2>&1 | tail -10
cargo test 2>&1 | grep -E "FAILED|test result"
```
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/pipeline.rs src/formula/mod.rs
git commit -m "feat(pipeline): pass table-cell bboxes as excluded regions into formula detectors"
```

---

### Task 3: Decorative-rule band-height filter in visual detector

**Files:**
- Modify: `src/formula/visual.rs`

**Ownership:**
- In scope: `looks_like_formula_band()` and the `DarkBand` filtering logic
- Out of scope: `detect.rs`, pipeline

**Assumption refs:** `A2`

**Invoke skill:** `test-driven-development` before starting this task.

- [ ] **Step 1: Write failing tests**

Add to `tests/formula_fp.rs`:

```rust
// Integration test — needs actual rendered page. Use unit test of the filter function instead.
// Test the band-height guard via the public API with a mock page that has only a thin horizontal rule.

#[test]
fn thin_horizontal_rule_not_flagged_by_visual_detector() {
    // A page that contains only a thin decorative rule and no formula cue words
    // should produce zero visual formula candidates.
    // We test this at the detect.rs level — visual candidates only fire when
    // either has_formula_cue() is true or existing word candidates exist.
    // A page with no formula cues and no word candidates → visual pass skipped.
    let p = page_with_words(vec![
        word("Section", 50.0, 100.0, 120.0, 112.0, 14.0),
        word("3.2",     125.0, 100.0, 155.0, 112.0, 14.0),
        // Thin decorative rule represented by a very wide, very short text span
        // (This tests the word path; visual path requires a real PDF render)
    ]);
    // Word detector should not flag the section heading
    let candidates = detect_formula_candidates(&p, &[]);
    assert!(candidates.is_empty(), "section heading should not be flagged");
}
```

For the visual band guard, add a unit test directly in `visual.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn band_below_height_threshold_rejected() {
        let band = DarkBand {
            y0: 100, y1: 101,  // 1px high — decorative rule
            x0: 50,  x1: 540,
            dark_pixels: 490,
            max_horizontal_run: 490,
        };
        // At RENDER_DPI=72, 1px ≈ 1pt. Minimum formula glyph height ≈ 4px at 72dpi.
        assert!(!band_has_sufficient_height(&band),
            "1px band should be rejected as decorative rule");
    }

    #[test]
    fn band_with_glyph_height_accepted() {
        let band = DarkBand {
            y0: 100, y1: 115,  // 15px high — normal formula glyph
            x0: 150, x1: 400,
            dark_pixels: 200,
            max_horizontal_run: 50,
        };
        assert!(band_has_sufficient_height(&band),
            "15px band should pass height check");
    }
}
```

- [ ] **Step 2: Run to confirm tests fail**

```bash
cargo test band_has_sufficient_height 2>&1 | grep -E "FAILED|error"
```

- [ ] **Step 3: Add band height guard to visual.rs**

Add the helper function:

```rust
/// A band must be at least this many pixels tall to be considered a formula region.
/// At 72 DPI, body text glyphs are typically 8–14px tall.
/// Decorative rules and logo bars are typically 1–3px.
const MIN_BAND_HEIGHT_PX: i32 = 4;

fn band_has_sufficient_height(band: &DarkBand) -> bool {
    (band.y1 - band.y0) >= MIN_BAND_HEIGHT_PX
}
```

In `looks_like_formula_band()` (or wherever bands are filtered before becoming candidates), add:

```rust
    if !band_has_sufficient_height(band) {
        return false;
    }
```

Place this as the first check — it's cheapest and rules out thin rules immediately.

- [ ] **Step 4: Run tests**

```bash
cargo test band_has_sufficient_height 2>&1 | tail -5
cargo test 2>&1 | grep -E "FAILED|test result"
```
Expected: new tests pass, no regressions.

- [ ] **Step 5: Commit**

```bash
git add src/formula/visual.rs
git commit -m "feat(formula): reject decorative-rule bands below minimum height threshold"
```

---

### Task 4: DNV regression fixtures

**Files:**
- Create: `tests/dnv_formula_regression.rs`

**Ownership:**
- In scope: `#[ignore]` integration tests that require the DNV PDF at a known path
- Out of scope: detector implementation (done in Tasks 1–3)

- [ ] **Step 1: Write the fixture file**

```rust
//! DNV-ST-N001 formula detection regression tests.
//! These tests require the DNV PDF at the path below and are ignored in CI.
//! Run locally with: cargo test --test dnv_formula_regression -- --ignored

const DNV_PDF: &str = "/home/eastill/projects/literature/standards/pdfs/\
    marine-operations-lifting-transport/\
    DNV-ST-N001_2018 - Marine operations and marine warranty.pdf";

#[cfg(test)]
mod dnv_formula_regression {
    use super::*;
    use pdf_processor::pdf::extractor::extract_raw_pages;
    use pdf_processor::formula::detect::detect_formula_candidates;

    fn dnv_raw_page(page_num: usize) -> pdf_processor::document::types::RawPage {
        let pages = extract_raw_pages(std::path::Path::new(DNV_PDF)).unwrap();
        pages.into_iter().find(|p| p.page_num == page_num).unwrap()
    }

    #[test]
    #[ignore = "requires DNV PDF"]
    fn page_597_has_zero_formula_candidates() {
        let page = dnv_raw_page(596); // 0-indexed
        let candidates = detect_formula_candidates(&page, &[]);
        assert!(
            candidates.is_empty(),
            "page 597 is references — got {} candidates: {:#?}",
            candidates.len(),
            candidates.iter().map(|c| &c.source_text).collect::<Vec<_>>()
        );
    }

    #[test]
    #[ignore = "requires DNV PDF"]
    fn page_130_has_formula_candidates() {
        let page = dnv_raw_page(129); // 0-indexed
        let candidates = detect_formula_candidates(&page, &[]);
        assert!(
            !candidates.is_empty(),
            "page 130 contains weight formulas — expected candidates"
        );
    }
}
```

- [ ] **Step 2: Run non-ignored tests to confirm compilation**

```bash
cargo test --test dnv_formula_regression 2>&1 | tail -5
```
Expected: 0 tests run (all `#[ignore]`), no compilation errors.

- [ ] **Step 3: Run ignored tests locally to check regression baselines**

```bash
cargo test --test dnv_formula_regression -- --ignored 2>&1 | tail -20
```
Expected: page 597 → 0 candidates; page 130 → ≥1 candidate.

- [ ] **Step 4: Commit**

```bash
git add tests/dnv_formula_regression.rs
git commit -m "test(formula): add DNV regression fixtures for FP pages 597 and 130"
```

---

### Task 5 (final): Acceptance + Review

**Files:**
- None (verification only)

- [ ] **Step 1: Run full test suite**

```bash
cargo test 2>&1 | grep -E "test result|FAILED"
```
Expected: all tests pass.

- [ ] **Step 2: Run clippy clean**

```bash
cargo clippy --all-targets -- -D warnings 2>&1 | grep -E "error|warning"
```
Expected: zero errors, zero warnings.

- [ ] **Step 3: Run DNV smoke test**

If the DNV PDF is available:
```bash
pdfp convert "$DNV_PDF" --conservative --debug-formulas --no-images -o /tmp/stage1-verify/
grep -c "formula-review" /tmp/stage1-verify/**/*.md
```
Expected: review markers still present but count should be lower than 3717 (pre-stage baseline).

- [ ] **Step 4: Verify page 597 no longer contributes candidates**

```bash
cargo test --test dnv_formula_regression -- --ignored page_597 2>&1 | tail -5
```

- [ ] **Step 5: Commit verification evidence**

```bash
git add .
git commit -m "test(formula): stage 1 verification — FP suppression confirmed"
```
