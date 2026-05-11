# Stage 4: Standards Table Detection

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task when tasks are independent. For same-session manual execution, follow this plan directly. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Achieve meaningful table detection recall on engineering standards PDFs (DNV/ISO/IEC) by extracting drawing-operation line geometry from MuPDF and using it for explicit-line table detection, supplemented by whitespace column inference for borderless tables.

**Architecture:** New `src/layout/drawing_ops.rs` extracts horizontal and vertical line segments from MuPDF page paths. New `src/layout/table_detector.rs` takes line segments plus text word positions and produces candidate table regions (bounding boxes). Classifier in `src/layout/classifier.rs` calls the table detector per page before block assembly. Detected table bboxes flow back into Stage 1/2 formula exclusion. No ML models — purely geometric.

**Tech Stack:** Rust, mupdf crate (path/drawing API), cargo test

**Recommended Skills:** test-driven-development, git

**Recommended MCPs:** none

**Status:** draft
**Refinement passes:** 0

## Assumptions

- `A1` — MuPDF exposes path drawing operations (lines and rectangles) from the Rust `mupdf` crate, or they can be accessed via `Page::to_display_list()` / device callbacks.
  Type: external
  Source: mupdf crate 0.6 docs; `grep -rn "DisplayList\|PathEvent\|draw_device" ~/.cargo/registry`
  Check: `grep -rn "Path\|DisplayList\|stroke\|fill" ~/.cargo/registry/src/**/mupdf*/ 2>/dev/null | grep "pub fn" | head -10`
  If false: render page at medium DPI (150 DPI) and use OpenCV-style morphological line detection via the `imageproc` crate. Update Task 1 accordingly.
  Owner: Task 1

- `A2` — DNV-ST-N001 specification tables have at least one horizontal rule at the top of the table (column header separator), even when they lack vertical grid lines.
  Type: design
  Source: DNV assessment 2026-05-11; wiki/topics/technical-standards-documents.md
  Check: visual inspection of pages 69 and 389 in source renders under `target/quick-assess-dnv/`
  If false: rely entirely on whitespace column inference (Task 3) and skip explicit-line detection for borderless tables.
  Owner: Task 2

- `A3` — `RawPage` is available alongside classified `Page` blocks in the pipeline (needed to access geometry for both drawing ops and word positions).
  Type: repo-state
  Source: `src/pipeline.rs` — raw pages and classified pages co-exist during processing
  Check: `grep -n "raw_page\|RawPage\|Page {" src/pipeline.rs | head -15`
  If false: carry `RawPage` alongside `Page` through to the table detector call site.
  Owner: Task 3

---

## File Map

| File | Change |
|------|--------|
| `src/layout/drawing_ops.rs` (new) | Extract H/V line segments from MuPDF page paths |
| `src/layout/table_detector.rs` (new) | Line-based + whitespace table region detection |
| `src/layout/mod.rs` | Add `pub mod drawing_ops; pub mod table_detector;` |
| `src/layout/classifier.rs` | Call table detector; annotate blocks with table context |
| `src/pipeline.rs` | Pass detected table bboxes to formula exclusion |
| `tests/table_detection.rs` (new) | Unit and DNV regression tests |

---

### Task 1: Drawing operations extractor

**Files:**
- Create: `src/layout/drawing_ops.rs`
- Modify: `src/layout/mod.rs`

**Ownership:**
- In scope: `extract_lines(page: &mupdf::Page) -> (Vec<HLine>, Vec<VLine>)`
- Out of scope: table detection logic, classifier

**Assumption refs:** `A1`

**Invoke skill:** `test-driven-development` before starting this task.

- [ ] **Step 1: Investigate MuPDF Rust API for path/line access**

```bash
grep -rn "pub fn\|DisplayList\|Annot\|Path\|Stroke" \
  ~/.cargo/registry/src/**/mupdf*/src/ 2>/dev/null | grep -i "line\|path\|draw" | head -20
```

Also check: `cat ~/.cargo/registry/src/**/mupdf*/src/page.rs 2>/dev/null | grep "pub fn" | head -30`

If MuPDF Rust wrapper does not expose path ops:
- Fall back to rasterising the page at 150 DPI via `page.to_pixmap()`
- Use `imageproc` crate morphological operations to extract H/V lines from the greyscale image
- Adjust the implementation in Step 3 accordingly

- [ ] **Step 2: Write failing tests**

Create `tests/table_detection.rs`:

```rust
use pdf_processor::layout::drawing_ops::{extract_lines, HLine, VLine};
use pdf_processor::document::types::Bbox;

#[test]
fn hline_struct_has_expected_fields() {
    let h = HLine { x0: 50.0, x1: 400.0, y: 200.0, thickness: 1.0 };
    assert!(h.length() > 300.0);
    assert!(h.is_significant()); // non-trivial length
}

#[test]
fn vline_struct_has_expected_fields() {
    let v = VLine { x: 100.0, y0: 150.0, y1: 400.0, thickness: 1.0 };
    assert!(v.length() > 200.0);
}

#[test]
#[ignore = "requires a test PDF with known lines"]
fn extracts_table_lines_from_pdf() {
    use std::path::Path;
    let pdf = Path::new("tests/fixtures/table_with_borders.pdf");
    let doc = mupdf::Document::open(pdf).unwrap();
    let page = doc.load_page(0).unwrap();
    let (hlines, vlines) = extract_lines(&page).unwrap();
    assert!(!hlines.is_empty(), "should detect horizontal rules");
    assert!(!vlines.is_empty(), "should detect vertical lines");
}
```

- [ ] **Step 3: Implement `src/layout/drawing_ops.rs`**

```rust
//! Extract line-segment geometry from PDF drawing operations.
//!
//! Two strategies depending on MuPDF API availability:
//!   Strategy A: Walk the page's path objects directly (preferred).
//!   Strategy B: Render page at 150 DPI; morphological line detection.
//!
//! Strategy B is used here as it is guaranteed to work with mupdf 0.6.

use anyhow::Result;
use mupdf::{Colorspace, Matrix, Page, Pixmap};

/// A horizontal line segment detected on the page (PDF coordinate space).
#[derive(Clone, Debug)]
pub struct HLine {
    pub x0: f32,
    pub x1: f32,
    pub y: f32,
    pub thickness: f32,
}

impl HLine {
    pub fn length(&self) -> f32 { (self.x1 - self.x0).abs() }
    /// A significant line spans at least 20% of typical page width.
    pub fn is_significant(&self) -> bool { self.length() > 100.0 }
}

/// A vertical line segment detected on the page (PDF coordinate space).
#[derive(Clone, Debug)]
pub struct VLine {
    pub x: f32,
    pub y0: f32,
    pub y1: f32,
    pub thickness: f32,
}

impl VLine {
    pub fn length(&self) -> f32 { (self.y1 - self.y0).abs() }
    pub fn is_significant(&self) -> bool { self.length() > 20.0 }
}

const DETECT_DPI: f32 = 150.0;
const SCALE: f32 = DETECT_DPI / 72.0;
/// Minimum dark-pixel run to count as a line (at 150 DPI).
const MIN_H_RUN_PX: usize = 50;
const MIN_V_RUN_PX: usize = 15;
/// Pixel brightness below which a pixel counts as "dark" (0–255).
const DARK_THRESH: u8 = 80;

/// Extract horizontal and vertical line segments from a rendered page.
pub fn extract_lines(page: &Page, page_width: f32, page_height: f32) -> Result<(Vec<HLine>, Vec<VLine>)> {
    let matrix = Matrix::new_scale(SCALE, SCALE);
    let pixmap = page
        .to_pixmap(&matrix, &Colorspace::device_gray(), 1.0, false)
        .map_err(|e| anyhow::anyhow!("pixmap render failed: {e}"))?;

    let w = pixmap.width() as usize;
    let h = pixmap.height() as usize;
    let samples = pixmap.samples();

    let mut hlines = Vec::new();
    let mut vlines = Vec::new();

    // Horizontal scan: for each row, find contiguous dark runs.
    for row in 0..h {
        let mut run_start: Option<usize> = None;
        for col in 0..w {
            let px = samples[row * w + col];
            if px < DARK_THRESH {
                if run_start.is_none() { run_start = Some(col); }
            } else if let Some(start) = run_start.take() {
                let run_len = col - start;
                if run_len >= MIN_H_RUN_PX {
                    let y_pdf = row as f32 / SCALE;
                    let x0_pdf = start as f32 / SCALE;
                    let x1_pdf = col as f32 / SCALE;
                    hlines.push(HLine { x0: x0_pdf, x1: x1_pdf, y: y_pdf, thickness: 1.0 });
                }
            }
        }
        if let Some(start) = run_start {
            let run_len = w - start;
            if run_len >= MIN_H_RUN_PX {
                hlines.push(HLine {
                    x0: start as f32 / SCALE,
                    x1: w as f32 / SCALE,
                    y: row as f32 / SCALE,
                    thickness: 1.0,
                });
            }
        }
    }

    // Vertical scan: for each column, find contiguous dark runs.
    for col in 0..w {
        let mut run_start: Option<usize> = None;
        for row in 0..h {
            let px = samples[row * w + col];
            if px < DARK_THRESH {
                if run_start.is_none() { run_start = Some(row); }
            } else if let Some(start) = run_start.take() {
                let run_len = row - start;
                if run_len >= MIN_V_RUN_PX {
                    vlines.push(VLine {
                        x: col as f32 / SCALE,
                        y0: start as f32 / SCALE,
                        y1: row as f32 / SCALE,
                        thickness: 1.0,
                    });
                }
            }
        }
    }

    // Merge adjacent near-identical lines (within 2px).
    let hlines = merge_hlines(hlines, page_width);
    let vlines = merge_vlines(vlines, page_height);

    Ok((hlines, vlines))
}

fn merge_hlines(mut lines: Vec<HLine>, _page_width: f32) -> Vec<HLine> {
    lines.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
    let mut merged: Vec<HLine> = Vec::new();
    for line in lines {
        if let Some(last) = merged.last_mut() {
            if (last.y - line.y).abs() < 2.0 && (last.x0 - line.x0).abs() < 5.0 {
                last.x1 = last.x1.max(line.x1);
                continue;
            }
        }
        merged.push(line);
    }
    merged
}

fn merge_vlines(mut lines: Vec<VLine>, _page_height: f32) -> Vec<VLine> {
    lines.sort_by(|a, b| a.x.partial_cmp(&b.x).unwrap());
    let mut merged: Vec<VLine> = Vec::new();
    for line in lines {
        if let Some(last) = merged.last_mut() {
            if (last.x - line.x).abs() < 2.0 && (last.y0 - line.y0).abs() < 5.0 {
                last.y1 = last.y1.max(line.y1);
                continue;
            }
        }
        merged.push(line);
    }
    merged
}
```

Add to `src/layout/mod.rs`:
```rust
pub mod drawing_ops;
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test table_detection hline_struct 2>&1 | tail -5
cargo test --test table_detection vline_struct 2>&1 | tail -5
cargo test 2>&1 | grep -E "FAILED|test result"
```

- [ ] **Step 5: Commit**

```bash
git add src/layout/drawing_ops.rs src/layout/mod.rs tests/table_detection.rs
git commit -m "feat(layout): add drawing-ops line extractor for table detection"
```

---

### Task 2: Line-based table region detector

**Files:**
- Create: `src/layout/table_detector.rs`
- Modify: `src/layout/mod.rs`

**Ownership:**
- In scope: `detect_table_regions(hlines, vlines, words, page_width, page_height) -> Vec<Bbox>`
- Out of scope: cell-level structure recovery, classifier integration

**Assumption refs:** `A2`

**Invoke skill:** `test-driven-development` before starting this task.

- [ ] **Step 1: Write failing tests**

Add to `tests/table_detection.rs`:

```rust
use pdf_processor::layout::table_detector::detect_table_regions;
use pdf_processor::layout::drawing_ops::{HLine, VLine};
use pdf_processor::document::types::{Bbox, RawWord};

fn word_at(x0: f32, y0: f32, x1: f32, y1: f32) -> RawWord {
    RawWord { text: "cell".into(), bbox: Bbox::new(x0, y0, x1, y1),
              font_size: 10.0, font_name: "Arial".into() }
}

#[test]
fn detects_table_between_two_hlines() {
    // Two horizontal rules bounding a text region = table
    let hlines = vec![
        HLine { x0: 50.0, x1: 450.0, y: 200.0, thickness: 1.0 },
        HLine { x0: 50.0, x1: 450.0, y: 240.0, thickness: 1.0 },
    ];
    let words = vec![
        word_at(60.0, 205.0, 120.0, 218.0),
        word_at(200.0, 205.0, 260.0, 218.0),
        word_at(350.0, 205.0, 420.0, 218.0),
    ];
    let regions = detect_table_regions(&hlines, &[], &words, 595.0, 842.0);
    assert!(!regions.is_empty(), "two hlines bounding text should produce a table region");
    let r = &regions[0];
    assert!(r.y0 <= 200.0 && r.y1 >= 240.0, "region should span the two hlines");
}

#[test]
fn no_table_when_hlines_are_isolated_rules() {
    // A single thin rule with no text between two adjacent rules
    let hlines = vec![
        HLine { x0: 50.0, x1: 450.0, y: 100.0, thickness: 0.5 },
    ];
    let regions = detect_table_regions(&hlines, &[], &[], 595.0, 842.0);
    assert!(regions.is_empty(), "single rule with no bounded text is not a table");
}

#[test]
fn whitespace_inference_detects_column_aligned_numbers() {
    // Three columns of right-aligned numbers with consistent x positions
    let words = vec![
        // Row 1
        word_at(50.0,  300.0, 90.0,  312.0),  // col 1
        word_at(180.0, 300.0, 220.0, 312.0),  // col 2
        word_at(320.0, 300.0, 360.0, 312.0),  // col 3
        // Row 2
        word_at(50.0,  316.0, 90.0,  328.0),
        word_at(180.0, 316.0, 220.0, 328.0),
        word_at(320.0, 316.0, 360.0, 328.0),
        // Row 3
        word_at(50.0,  332.0, 90.0,  344.0),
        word_at(180.0, 332.0, 220.0, 344.0),
        word_at(320.0, 332.0, 360.0, 344.0),
    ];
    let regions = detect_table_regions(&[], &[], &words, 595.0, 842.0);
    assert!(!regions.is_empty(), "3x3 column-aligned text should be detected as table");
}
```

- [ ] **Step 2: Confirm tests fail**

```bash
cargo test --test table_detection detects_table_between 2>&1 | grep -E "FAILED|error\["
```

- [ ] **Step 3: Implement `src/layout/table_detector.rs`**

```rust
//! Detect table candidate regions from line geometry and word positions.
//!
//! Two strategies:
//!  1. Line-based: pairs of significant H lines bounding text words → table.
//!  2. Whitespace-based: ≥3 words aligned in ≥3 consistent columns across ≥2 rows → table.

use crate::document::types::{Bbox, RawWord};
use crate::layout::drawing_ops::{HLine, VLine};

const MIN_TABLE_WIDTH_FRACTION: f32 = 0.3; // table must span at least 30% of page width
const MIN_COLUMN_COUNT: usize = 2;
const MIN_ROW_COUNT: usize = 2;
/// Maximum vertical gap between two hlines to be considered a table band.
const MAX_TABLE_BAND_HEIGHT: f32 = 300.0;
const MIN_TABLE_BAND_HEIGHT: f32 = 8.0;

/// Returns bounding boxes of candidate table regions.
pub fn detect_table_regions(
    hlines: &[HLine],
    _vlines: &[VLine],
    words: &[RawWord],
    page_width: f32,
    _page_height: f32,
) -> Vec<Bbox> {
    let mut regions = Vec::new();

    // Strategy 1: significant hline pairs with words between them.
    let sig_hlines: Vec<&HLine> = hlines
        .iter()
        .filter(|h| h.is_significant() && h.length() > page_width * MIN_TABLE_WIDTH_FRACTION)
        .collect();

    for i in 0..sig_hlines.len() {
        for j in (i + 1)..sig_hlines.len() {
            let top = sig_hlines[i];
            let bot = sig_hlines[j];
            let band_h = bot.y - top.y;
            if band_h < MIN_TABLE_BAND_HEIGHT || band_h > MAX_TABLE_BAND_HEIGHT {
                continue;
            }
            // Count words between the two lines.
            let words_in_band = words
                .iter()
                .filter(|w| w.bbox.y0 >= top.y && w.bbox.y1 <= bot.y)
                .count();
            if words_in_band < MIN_COLUMN_COUNT {
                continue;
            }
            let x0 = top.x0.min(bot.x0);
            let x1 = top.x1.max(bot.x1);
            regions.push(Bbox::new(x0, top.y, x1, bot.y));
        }
    }

    // Strategy 2: whitespace-based column alignment.
    if let Some(ws_region) = whitespace_table_region(words, page_width) {
        // Only add if not already covered by a line-based region.
        if !regions.iter().any(|r| overlap_ratio(*r, ws_region) > 0.5) {
            regions.push(ws_region);
        }
    }

    regions
}

/// Simple whitespace column inference: cluster word x-centres into N columns,
/// then check if ≥MIN_ROW_COUNT rows have words in ≥MIN_COLUMN_COUNT columns.
fn whitespace_table_region(words: &[RawWord], page_width: f32) -> Option<Bbox> {
    if words.len() < MIN_COLUMN_COUNT * MIN_ROW_COUNT {
        return None;
    }

    // Collect x-centres.
    let mut x_centres: Vec<f32> = words.iter().map(|w| w.bbox.center_x()).collect();
    x_centres.sort_by(|a, b| a.partial_cmp(b).unwrap());

    // Find column boundaries via large gaps.
    let col_gap_threshold = page_width * 0.08;
    let mut column_xs: Vec<f32> = vec![x_centres[0]];
    for &x in &x_centres[1..] {
        if x - column_xs.last().copied().unwrap_or(0.0) > col_gap_threshold {
            column_xs.push(x);
        } else {
            // Update running centre.
            *column_xs.last_mut().unwrap() = x;
        }
    }
    if column_xs.len() < MIN_COLUMN_COUNT {
        return None;
    }

    // Group words into rows by Y proximity (within 15pt).
    let mut rows: Vec<(f32, Vec<&RawWord>)> = Vec::new();
    for word in words {
        let y = word.bbox.center_y();
        if let Some(row) = rows.iter_mut().find(|(ry, _)| (ry - y).abs() < 15.0) {
            row.1.push(word);
        } else {
            rows.push((y, vec![word]));
        }
    }

    // Count rows that have words in at least MIN_COLUMN_COUNT distinct columns.
    let qualifying_rows: Vec<_> = rows
        .iter()
        .filter(|(_, row_words)| {
            let cols_hit: std::collections::HashSet<usize> = row_words
                .iter()
                .filter_map(|w| {
                    let cx = w.bbox.center_x();
                    column_xs
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| {
                            (cx - **a).abs().partial_cmp(&(cx - **b).abs()).unwrap()
                        })
                        .map(|(i, _)| i)
                })
                .collect();
            cols_hit.len() >= MIN_COLUMN_COUNT
        })
        .collect();

    if qualifying_rows.len() < MIN_ROW_COUNT {
        return None;
    }

    // Bounding box over qualifying rows.
    let all_words: Vec<&RawWord> = qualifying_rows
        .iter()
        .flat_map(|(_, ws)| ws.iter().copied())
        .collect();
    let x0 = all_words.iter().map(|w| w.bbox.x0).fold(f32::MAX, f32::min);
    let y0 = all_words.iter().map(|w| w.bbox.y0).fold(f32::MAX, f32::min);
    let x1 = all_words.iter().map(|w| w.bbox.x1).fold(f32::MIN, f32::max);
    let y1 = all_words.iter().map(|w| w.bbox.y1).fold(f32::MIN, f32::max);
    Some(Bbox::new(x0, y0, x1, y1))
}

fn overlap_ratio(a: Bbox, b: Bbox) -> f32 {
    let ix0 = a.x0.max(b.x0);
    let iy0 = a.y0.max(b.y0);
    let ix1 = a.x1.min(b.x1);
    let iy1 = a.y1.min(b.y1);
    if ix1 <= ix0 || iy1 <= iy0 { return 0.0; }
    let inter = (ix1 - ix0) * (iy1 - iy0);
    let area_a = (a.x1 - a.x0) * (a.y1 - a.y0);
    inter / area_a.max(1.0)
}
```

Add to `src/layout/mod.rs`:
```rust
pub mod table_detector;
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test table_detection 2>&1 | tail -15
cargo test 2>&1 | grep -E "FAILED|test result"
```

- [ ] **Step 5: Commit**

```bash
git add src/layout/table_detector.rs src/layout/mod.rs tests/table_detection.rs
git commit -m "feat(layout): add line-based and whitespace table region detector for standards"
```

---

### Task 3: Integrate table detector into pipeline and classifier

**Files:**
- Modify: `src/pipeline.rs`
- Modify: `src/layout/classifier.rs`

**Ownership:**
- In scope: calling `extract_lines` + `detect_table_regions` per page, feeding results to formula exclusion
- Out of scope: table structure recovery (cell content, row/column assignment)

**Assumption refs:** `A1`, `A3`

- [ ] **Step 1: Write integration test**

Add to `tests/table_detection.rs`:

```rust
#[test]
#[ignore = "requires DNV PDF"]
fn dnv_page_69_produces_table_candidates() {
    use pdf_processor::pdf::extractor::extract_raw_pages;
    use pdf_processor::layout::{drawing_ops::extract_lines, table_detector::detect_table_regions};
    use std::path::Path;
    use mupdf::Document;

    let pdf_path = Path::new("/home/eastill/projects/literature/standards/pdfs/\
        marine-operations-lifting-transport/\
        DNV-ST-N001_2018 - Marine operations and marine warranty.pdf");
    let raw_pages = extract_raw_pages(pdf_path).unwrap();
    let raw_page = raw_pages.iter().find(|p| p.page_num == 68).unwrap(); // 0-indexed

    let doc = Document::open(pdf_path).unwrap();
    let page = doc.load_page(68).unwrap();
    let (hlines, vlines) = extract_lines(&page, raw_page.width, raw_page.height).unwrap();
    let regions = detect_table_regions(&hlines, &vlines, &raw_page.words, raw_page.width, raw_page.height);

    assert!(!regions.is_empty(),
        "DNV page 69 (alpha-factor table) should produce at least one table region, got 0");
}
```

- [ ] **Step 2: Confirm test fails (ignored)**

```bash
cargo test --test table_detection dnv_page_69 2>&1 | tail -5
```
Expected: 0 tests run (ignored), no compile errors.

- [ ] **Step 3: Wire into pipeline.rs**

In `src/pipeline.rs`, for each page during processing, before formula detection:

```rust
use crate::layout::drawing_ops::extract_lines;
use crate::layout::table_detector::detect_table_regions;
use mupdf::Document as MuDocument;

// Open MuPDF document once per document (reuse existing `mu_doc` if available)
let (hlines, vlines) = if let Ok(mu_page) = mu_doc.load_page(raw_page.page_num as i32) {
    extract_lines(&mu_page, raw_page.width, raw_page.height).unwrap_or_default()
} else {
    (vec![], vec![])
};

let geometry_table_bboxes = detect_table_regions(
    &hlines, &vlines, &raw_page.words, raw_page.width, raw_page.height
);

// Merge with existing table_bboxes (from classified TableCell blocks, Stage 1)
let mut excluded: Vec<Bbox> = table_bboxes.clone();
excluded.extend_from_slice(&geometry_table_bboxes);
if let Some(furniture) = furniture_mask.get(&raw_page.page_num) {
    excluded.extend_from_slice(furniture);
}
let formula_candidates = detect_formula_candidates(raw_page, &excluded);
```

- [ ] **Step 4: Run tests and smoke test**

```bash
cargo test 2>&1 | grep -E "FAILED|test result"
cargo build --release 2>&1 | grep "^error"
```

If DNV PDF available:
```bash
cargo test --test table_detection -- --ignored dnv_page_69 2>&1 | tail -10
```

- [ ] **Step 5: Commit**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): integrate table region detector; geometry-based table bboxes fed to formula exclusion"
```

---

### Task 4 (final): Acceptance + Review

- [ ] **Step 1: Full test suite**

```bash
cargo test 2>&1 | grep -E "test result|FAILED"
cargo clippy --all-targets -- -D warnings 2>&1 | grep error
```

- [ ] **Step 2: DNV table recall smoke test**

```bash
pdfp convert "$DNV_PDF" --conservative --debug-formulas --debug-tables --no-images \
    -o /tmp/stage4-verify/
# Count table candidates vs pre-stage baseline of 10
grep -c '"table_region"' /tmp/stage4-verify/**/debug/tables/*.json 2>/dev/null | tail -1
```
Expected: meaningfully more than 10 table candidates (pre-stage baseline).

- [ ] **Step 3: Verify formula FP reduction on table pages**

```bash
# Page 69 should now have formula candidates suppressed by table regions
cargo test --test dnv_formula_regression -- --ignored 2>&1 | tail -10
```

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "docs: stage 4 verification — standards table detection confirmed"
```
