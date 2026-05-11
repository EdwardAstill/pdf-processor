# Stage 2: Furniture Suppression Pre-pass

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task when tasks are independent. For same-session manual execution, follow this plan directly. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Detect running headers, footers, and watermarks via cross-page text similarity and build a per-page furniture mask that suppresses those regions from formula detection, table extraction, and Markdown rendering.

**Architecture:** New `src/layout/furniture.rs` module. Takes `&[RawPage]` (the full document), computes a Y-band text-similarity pass across all pages, returns `HashMap<usize, Vec<Bbox>>` (furniture bboxes per page). Pipeline runs this once before classification; the mask is stored on the `Document` and consulted by the formula and table detectors. No ML required — pure geometry and string similarity.

**Tech Stack:** Rust, std::collections::HashMap, cargo test

**Recommended Skills:** test-driven-development, git

**Recommended MCPs:** none

**Status:** draft
**Refinement passes:** 0

## Assumptions

- `A1` — The full `Vec<RawPage>` is available in the pipeline before classification runs.
  Type: architectural
  Source: `src/pipeline.rs` — raw pages are extracted then classified
  Check: `grep -n "raw_pages\|RawPage\|classify" src/pipeline.rs | head -15`
  If false: pass pages to furniture detector lazily, or buffer them.
  Owner: Task 3

- `A2` — Watermark text ("Downloaded by", "No further distribution") appears consistently in the bottom ~8% of page height across most pages of a DNV document.
  Type: design
  Source: assessment output 2026-05-11; wiki/topics/technical-standards-documents.md
  Check: visually verify on a rendered DNV page with `pdfp inspect DNV.pdf --json`
  If false: widen the margin band or use full-page text scan with frequency counting.
  Owner: Task 1

- `A3` — `Document` struct can carry additional metadata without breaking existing serialisation.
  Type: repo-state
  Source: `src/document/types.rs`
  Check: `grep -n "pub struct Document" src/document/types.rs`
  If false: pass furniture mask through pipeline as a separate `&HashMap<usize, Vec<Bbox>>` parameter.
  Owner: Task 2

---

## File Map

| File | Change |
|------|--------|
| `src/layout/furniture.rs` (new) | Furniture detection: page-association + margin scanning |
| `src/layout/mod.rs` | Add `pub mod furniture;` |
| `src/document/types.rs` | Add `furniture_mask: HashMap<usize, Vec<Bbox>>` to `Document` or pipeline context |
| `src/pipeline.rs` | Run furniture pass; pass mask to formula/table detectors |
| `src/formula/detect.rs` | Accept `furniture_mask` and skip furniture regions |
| `tests/furniture.rs` (new) | Unit and integration tests |

---

### Task 1: Implement furniture detection module

**Files:**
- Create: `src/layout/furniture.rs`
- Modify: `src/layout/mod.rs`

**Ownership:**
- In scope: `detect_furniture_bboxes(pages: &[RawPage]) -> HashMap<usize, Vec<Bbox>>`
- Out of scope: pipeline wiring, formula integration

**Assumption refs:** `A2`

**Invoke skill:** `test-driven-development` before starting this task.

- [ ] **Step 1: Write failing tests**

Create `tests/furniture.rs`:

```rust
use pdf_processor::layout::furniture::detect_furniture_bboxes;
use pdf_processor::document::types::{RawPage, RawWord, Bbox};

fn word(text: &str, x0: f32, y0: f32, x1: f32, y1: f32) -> RawWord {
    RawWord { text: text.to_string(), bbox: Bbox::new(x0, y0, x1, y1),
              font_size: 9.0, font_name: "Arial".into() }
}

fn make_page(page_num: usize, footer_text: &[&str], body_text: &[&str], height: f32) -> RawPage {
    let mut words = vec![];
    // Body text near top
    for (i, t) in body_text.iter().enumerate() {
        words.push(word(t, 50.0, 50.0 + i as f32 * 15.0, 200.0, 62.0 + i as f32 * 15.0));
    }
    // Footer text near bottom
    let footer_y = height - 20.0;
    let mut x = 50.0;
    for t in footer_text {
        words.push(word(t, x, footer_y, x + 60.0, footer_y + 12.0));
        x += 65.0;
    }
    RawPage { page_num, width: 595.0, height, words, images: vec![] }
}

#[test]
fn repeated_footer_detected_as_furniture() {
    let footer = &["Downloaded", "by", "ACME", "Corp"];
    let pages: Vec<_> = (0..5).map(|i| {
        make_page(i, footer, &["Body", "text", "here"], 842.0)
    }).collect();
    let mask = detect_furniture_bboxes(&pages);
    // All 5 pages should have at least one furniture bbox in the footer region
    for i in 0..5usize {
        assert!(mask.contains_key(&i), "page {} should have furniture entries", i);
    }
}

#[test]
fn unique_page_text_not_marked_furniture() {
    // Body text differs on every page → not furniture
    let pages: Vec<_> = (0..5).map(|i| {
        make_page(i, &[], &[&format!("Unique content page {}", i)], 842.0)
    }).collect();
    let mask = detect_furniture_bboxes(&pages);
    // No footer text at all → mask may be empty or have no footer entries
    for i in 0..5usize {
        let bboxes = mask.get(&i).map(|v| v.len()).unwrap_or(0);
        assert_eq!(bboxes, 0, "unique body text should not be marked furniture on page {}", i);
    }
}

#[test]
fn single_page_document_produces_empty_mask() {
    let pages = vec![make_page(0, &["footer", "text"], &["body"], 842.0)];
    let mask = detect_furniture_bboxes(&pages);
    // Cannot detect furniture with only one page — need cross-page comparison
    assert!(mask.get(&0).map(|v| v.is_empty()).unwrap_or(true));
}
```

- [ ] **Step 2: Run to confirm tests fail**

```bash
cargo test --test furniture 2>&1 | grep -E "FAILED|error\[" | head -10
```

- [ ] **Step 3: Implement `src/layout/furniture.rs`**

```rust
//! Furniture detection via cross-page text similarity.
//!
//! Running headers, footers, and watermarks appear at consistent Y positions
//! with near-identical text across most pages. This pass identifies them
//! before classification so they can be excluded from formula/table detection.

use std::collections::HashMap;
use crate::document::types::{Bbox, RawPage, RawWord};

/// Fraction of page height to scan for headers (top) and footers (bottom).
const MARGIN_FRACTION: f32 = 0.08;

/// Minimum fraction of pages on which a text band must repeat to be furniture.
const REPEAT_THRESHOLD: f32 = 0.5;

/// Returns a map of page_num → list of furniture bboxes.
pub fn detect_furniture_bboxes(pages: &[RawPage]) -> HashMap<usize, Vec<Bbox>> {
    if pages.len() < 2 {
        return HashMap::new();
    }

    let mut result: HashMap<usize, Vec<Bbox>> = HashMap::new();

    // For each page, collect margin-band text signatures and their bboxes.
    let margin_data: Vec<Vec<(String, Bbox)>> = pages
        .iter()
        .map(|p| margin_words(p))
        .collect();

    // Build a frequency map: normalised_text → count across pages.
    let mut freq: HashMap<String, usize> = HashMap::new();
    for page_data in &margin_data {
        // Dedupe within a page — only count each text once per page.
        let mut seen = std::collections::HashSet::new();
        for (text, _) in page_data {
            let norm = normalise(text);
            if !norm.is_empty() && seen.insert(norm.clone()) {
                *freq.entry(norm).or_insert(0) += 1;
            }
        }
    }

    let min_count = ((pages.len() as f32 * REPEAT_THRESHOLD).ceil() as usize).max(2);

    // Mark bboxes on pages where furniture text appears.
    for (page_idx, page_data) in margin_data.iter().enumerate() {
        for (text, bbox) in page_data {
            let norm = normalise(text);
            if freq.get(&norm).copied().unwrap_or(0) >= min_count {
                result.entry(page_idx).or_default().push(*bbox);
            }
        }
    }

    result
}

/// Collect words in the top and bottom margin bands.
fn margin_words(page: &RawPage) -> Vec<(String, Bbox)> {
    let top_limit = page.height * MARGIN_FRACTION;
    let bottom_limit = page.height * (1.0 - MARGIN_FRACTION);
    page.words
        .iter()
        .filter(|w| w.bbox.y0 < top_limit || w.bbox.y1 > bottom_limit)
        .map(|w| (w.text.clone(), w.bbox))
        .collect()
}

/// Normalise text for comparison: lowercase, strip punctuation, trim.
fn normalise(text: &str) -> String {
    text.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}
```

Add to `src/layout/mod.rs`:
```rust
pub mod furniture;
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test furniture 2>&1 | tail -10
cargo test 2>&1 | grep -E "FAILED|test result"
```
Expected: all 3 furniture tests pass, no regressions.

- [ ] **Step 5: Commit**

```bash
git add src/layout/furniture.rs src/layout/mod.rs tests/furniture.rs
git commit -m "feat(layout): add furniture detection via cross-page text similarity"
```

---

### Task 2: Wire furniture mask into pipeline and formula detection

**Files:**
- Modify: `src/pipeline.rs`
- Modify: `src/formula/detect.rs`

**Ownership:**
- In scope: pipeline stage ordering, passing mask into formula detector
- Out of scope: table detection integration (Stage 4)

**Assumption refs:** `A1`, `A3`

- [ ] **Step 1: Write a failing test showing watermark text not in formula candidates**

Add to `tests/furniture.rs`:

```rust
use pdf_processor::formula::detect::detect_formula_candidates;

#[test]
fn watermark_words_not_flagged_as_formulas() {
    // "Downloaded by ACME Corp on 2024-01-15" in the bottom margin
    // should not produce formula candidates even if it contains numbers.
    let height = 842.0_f32;
    let footer_y = height - 15.0;
    let p = RawPage {
        page_num: 0, width: 595.0, height,
        images: vec![],
        words: vec![
            word("Downloaded", 50.0, footer_y, 130.0, footer_y + 10.0),
            word("by",          132.0, footer_y, 150.0, footer_y + 10.0),
            word("ACME",        152.0, footer_y, 195.0, footer_y + 10.0),
            word("Corp",        197.0, footer_y, 230.0, footer_y + 10.0),
            word("on",          232.0, footer_y, 248.0, footer_y + 10.0),
            word("2024-01-15",  250.0, footer_y, 320.0, footer_y + 10.0),
        ],
    };
    let furniture_bboxes = vec![Bbox::new(0.0, footer_y - 5.0, 595.0, height)];
    let candidates = detect_formula_candidates(&p, &furniture_bboxes);
    assert!(candidates.is_empty(), "watermark line should be suppressed by furniture mask");
}
```

- [ ] **Step 2: Confirm test fails**

```bash
cargo test --test furniture watermark_words 2>&1 | grep -E "FAILED|ok"
```

- [ ] **Step 3: Update pipeline.rs to run furniture detection and merge with table bboxes**

In `src/pipeline.rs`, find where raw pages are extracted and classification begins. Add:

```rust
use crate::layout::furniture::detect_furniture_bboxes;

// After extracting raw_pages, before classification:
let furniture_mask = detect_furniture_bboxes(&raw_pages);
```

When calling `detect_formula_candidates` per page:

```rust
let mut excluded: Vec<Bbox> = table_bboxes.clone(); // from Stage 1
if let Some(furniture) = furniture_mask.get(&raw_page.page_num) {
    excluded.extend_from_slice(furniture);
}
let formula_candidates = detect_formula_candidates(raw_page, &excluded);
```

Apply the same exclusion when calling `detect_visual_formula_candidates`.

- [ ] **Step 4: Run tests**

```bash
cargo test --test furniture 2>&1 | tail -10
cargo test 2>&1 | grep -E "FAILED|test result"
```
Expected: all tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/pipeline.rs
git commit -m "feat(pipeline): apply furniture mask to formula detection"
```

---

### Task 3 (final): Acceptance + Review

**Files:**
- Verify only

- [ ] **Step 1: Run full test suite**

```bash
cargo test 2>&1 | grep -E "test result|FAILED"
cargo clippy --all-targets -- -D warnings 2>&1 | grep error
```

- [ ] **Step 2: DNV smoke test for watermark suppression**

```bash
pdfp convert "$DNV_PDF" --conservative --debug-formulas --no-images -o /tmp/stage2-verify/
# Check that "Downloaded" and "No further" text does not appear in formula crops
grep -ri "downloaded\|no further" /tmp/stage2-verify/**/debug/formulas/*.json || echo "clean"
```
Expected: no watermark text in formula candidate JSON.

- [ ] **Step 3: Check furniture detection on DNV**

```bash
# Inspect page 1 furniture mask via debug output (add --debug-furniture flag if available,
# or add a temporary log statement)
pdfp inspect "$DNV_PDF" --json | python3 -c "import json,sys; d=json.load(sys.stdin); print(len(d.get('pages', [])))"
```

- [ ] **Step 4: Commit and note limitations**

```bash
git add .
git commit -m "docs: stage 2 furniture suppression — verification complete"
```
