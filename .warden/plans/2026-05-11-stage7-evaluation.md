# Stage 7: Evaluation Infrastructure

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task when tasks are independent. For same-session manual execution, follow this plan directly. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `pdfp eval <fixtures-dir>` CLI command that converts fixture PDFs, compares output against per-page expected values, and reports precision/recall for formulas, tables, and headings.

**Architecture:** A new `src/eval/` module contains the fixture schema (`fixtures.rs`), metric calculations (`metrics.rs`), and the runner (`runner.rs`). Fixtures are JSON files alongside their PDFs. Each fixture lists per-page expectations (formula LaTeX snippets, heading text+level, table count). The runner must invoke the full local conversion pipeline in-process and collect the resulting `Document`; do not rebuild eval from only `PdfExtractor + Classifier`, because Stage 2 furniture suppression, Stage 3 formula sidecars, and Stage 4 geometry table detection/formula exclusion all live in `pipeline.rs`. A `pdfp eval` subcommand wires this up. No new external dependencies are required — serde_json is already used by the hybrid client.

**Tech Stack:** Rust, serde_json, cargo test, existing pipeline types (`Document`, `Page`, `Block`, `BlockKind`)

**Recommended Skills:** test-driven-development, git

**Recommended MCPs:** none

**Status:** draft
**Refinement passes:** 0

## Assumptions

- `A1` — `serde_json` is already a dependency (used by hybrid client).
  Type: repo-state
  Source: `src/hybrid/client.rs` — serde_json used for request/response parsing
  Check: `grep "serde_json" Cargo.toml`
  If false: add `serde_json = "1"` to `[dependencies]`.
  Owner: Task 1

- `A2` — The evaluation runner needs an in-process full-pipeline entry point that returns `Document`. If no public function exists yet, Stage 7 must first extract one from `pipeline.rs` without changing CLI behavior.
  Type: repo-state
  Source: Stage 4 implementation; geometry table detection is wired in `build_document_from_raw` / `build_page`
  Check: `grep -n "fn build_document_from_raw\|fn build_page\|pub fn process_pdf" src/pipeline.rs`
  If false: use the existing public full-pipeline API.
  Owner: Task 3

- `A3` — Formula recall is computed as `found / expected` where "found" means at least one `BlockKind::Formula` or `BlockKind::FormulaReview` block on the page regardless of LaTeX correctness, and "expected" is the `expected_formula_count` field in the fixture. LaTeX string matching is skipped for now (OCR quality is separate from detection recall).
  Type: design
  Source: pragmatic decision — LaTeX similarity scoring requires edit-distance or BLEU, added in future
  Check: N/A (design decision)
  If false: add edit-distance matching to metrics module.
  Owner: Task 2

- `A4` — Table recall is computed at the page level: a page "passes" if at least one `BlockKind::CoordinateTable` is found when `expected_tables > 0`. Page-level precision/recall is reported. For standards smoke fixtures, optionally also report debug `table_region` counts, but the primary metric should use the final `Document` blocks.
  Type: design
  Source: pragmatic decision — cell-level coverage requires a ground-truth table labeller
  Check: N/A (design decision)
  If false: refine to cell coverage in a future pass.
  Owner: Task 2

- `A5` — Heading accuracy is `exact_match_count / expected_count` where exact match requires both `text.trim() == expected.text.trim()` (case-insensitive) and `level == expected.level`.
  Type: design
  Source: heading text+level is the minimal verifiable signal from the classifier
  Check: N/A (design decision)
  If false: relax to text-only matching and report level separately.
  Owner: Task 2

- `A6` — The `pdfp eval` command does NOT need `--hybrid` or other advanced flags. It uses the default local pipeline only (no docling-serve connection required during eval).
  Type: policy
  Source: evaluation should be repeatable offline
  Check: N/A (design decision)
  If false: add optional `--hybrid` flag to `EvalArgs`.
  Owner: Task 4

---

## File Map

| File | Change |
|------|--------|
| `src/eval/mod.rs` (new) | Module root: `pub mod fixtures; pub mod metrics; pub mod runner;` |
| `src/eval/fixtures.rs` (new) | `FixtureFile`, `PageExpectation` structs + serde; `load_fixtures()` |
| `src/eval/metrics.rs` (new) | `PageMetrics`, `DocMetrics`, `compute_page_metrics()`, `aggregate()`, `print_report()` |
| `src/eval/runner.rs` (new) | `run_eval(fixtures: &[FixtureFile]) -> Vec<DocResult>`; calls full local pipeline |
| `src/pipeline.rs` | Add or expose an in-process `Document`-returning local pipeline entry point for eval |
| `src/main.rs` | Add `mod eval;` and `Commands::Eval` dispatch |
| `src/cli.rs` | Add `Eval(EvalArgs)` variant to `Commands`; `EvalArgs { dir: PathBuf }` |
| `tests/eval_fixtures/` (new) | Example fixture JSON + minimal test PDF |
| `docs/TESTING.md` | Document `pdfp eval` command and fixture format |

---

### Task 1: Define fixture schema

**Files:**
- Create: `src/eval/mod.rs`
- Create: `src/eval/fixtures.rs`
- Test: `tests/eval_integration.rs`

**Ownership:**
- In scope: `FixtureFile`, `PageExpectation` data types, `load_fixtures()`
- Out of scope: metric computation, pipeline invocation

**Assumption refs:** `A1`

- [ ] **Step 1: Write a failing test for fixture loading**

Create `tests/eval_integration.rs`:

```rust
use std::io::Write;
use pdf_processor::eval::fixtures::{load_fixtures, FixtureFile};

fn write_fixture(json: &str) -> tempfile::NamedTempFile {
    let mut f = tempfile::NamedTempFile::with_suffix(".json").unwrap();
    write!(f, "{}", json).unwrap();
    f
}

#[test]
fn load_fixtures_parses_valid_json() {
    let json = r#"{
        "pdf": "sample.pdf",
        "pages": [
            {
                "page": 1,
                "expected_formula_count": 2,
                "expected_headings": [{"text": "Introduction", "level": 1}],
                "expected_tables": 0
            }
        ]
    }"#;
    let f = write_fixture(json);
    let fixtures = load_fixtures(f.path().parent().unwrap()).expect("load");
    assert_eq!(fixtures.len(), 1);
    assert_eq!(fixtures[0].pages[0].expected_formula_count, 2);
    assert_eq!(fixtures[0].pages[0].expected_headings[0].text, "Introduction");
    assert_eq!(fixtures[0].pages[0].expected_headings[0].level, 1);
}

#[test]
fn load_fixtures_skips_non_json_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "# readme").unwrap();
    let fixtures = load_fixtures(dir.path()).expect("load");
    assert_eq!(fixtures.len(), 0);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test eval_integration load_fixtures 2>&1 | tail -15`
Expected: compile error — `eval` module and `load_fixtures` not found.

- [ ] **Step 3: Implement `src/eval/fixtures.rs`**

Create `src/eval/mod.rs`:

```rust
pub mod fixtures;
pub mod metrics;
pub mod runner;
```

Create `src/eval/fixtures.rs`:

```rust
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Deserialize)]
pub struct ExpectedHeading {
    pub text: String,
    pub level: u8,
}

#[derive(Debug, Deserialize)]
pub struct PageExpectation {
    pub page: usize,
    #[serde(default)]
    pub expected_formula_count: usize,
    #[serde(default)]
    pub expected_headings: Vec<ExpectedHeading>,
    #[serde(default)]
    pub expected_tables: usize,
}

#[derive(Debug, Deserialize)]
pub struct FixtureFile {
    /// Path to PDF, relative to the fixture JSON file's directory.
    pub pdf: String,
    pub pages: Vec<PageExpectation>,
    /// Absolute path to the fixture JSON (set by loader, not in JSON).
    #[serde(skip)]
    pub fixture_dir: PathBuf,
}

impl FixtureFile {
    pub fn pdf_path(&self) -> PathBuf {
        self.fixture_dir.join(&self.pdf)
    }
}

/// Load all `.json` fixture files from `dir` (non-recursive).
pub fn load_fixtures(dir: &Path) -> anyhow::Result<Vec<FixtureFile>> {
    let mut fixtures = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        let json = std::fs::read_to_string(&path)?;
        let mut fixture: FixtureFile = serde_json::from_str(&json)
            .map_err(|e| anyhow::anyhow!("fixture parse error {}: {}", path.display(), e))?;
        fixture.fixture_dir = dir.to_path_buf();
        fixtures.push(fixture);
    }
    Ok(fixtures)
}
```

Add `mod eval;` to `src/main.rs` (before the CLI match block).

Add to `src/lib.rs` or the appropriate re-export if tests import from `pdf_processor::eval`. If the crate has no `lib.rs`, create one with:

```rust
pub mod eval;
```

And ensure `src/main.rs` does `#[path = "eval/mod.rs"] mod eval_internal;` only for binary — tests import from the lib. Check whether a `lib.rs` already exists:

```bash
ls src/lib.rs 2>/dev/null && echo "exists" || echo "not found"
```

If `lib.rs` does not exist, create it:

```rust
// Public API surface for integration tests.
pub mod document;
pub mod eval;
pub mod layout;
pub mod formula;
pub mod render;
pub mod pdf;
```

Add the same modules that are already `mod X` in `main.rs`.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test eval_integration load_fixtures`
Expected: PASS.

Run: `cargo test 2>&1 | tail -5`
Expected: no regressions.

- [ ] **Step 5: Commit**

```bash
git add src/eval/ src/main.rs src/lib.rs tests/eval_integration.rs
git commit -m "feat(eval): fixture schema and loader"
```

---

### Task 2: Implement metrics computation

**Files:**
- Create: `src/eval/metrics.rs`
- Test: `tests/eval_integration.rs`

**Ownership:**
- In scope: `PageMetrics`, `DocMetrics`, `compute_page_metrics()`, `aggregate()`, `print_report()`
- Out of scope: pipeline invocation, fixture loading

**Assumption refs:** `A3`, `A4`, `A5`

- [ ] **Step 1: Write failing tests for metric computation**

In `tests/eval_integration.rs`, add:

```rust
use pdf_processor::eval::metrics::{compute_page_metrics, PageMetrics};
use pdf_processor::eval::fixtures::PageExpectation;
use pdf_processor::document::types::{Bbox, Block, BlockKind, Page};

fn bbox() -> Bbox { Bbox { x0: 0.0, y0: 0.0, x1: 100.0, y1: 20.0 } }
fn make_block(kind: BlockKind, text: &str) -> Block {
    Block {
        id: 0, bbox: bbox(), text: text.to_string(),
        kind, font_size: 12.0, font_name: "Times".to_string(),
        page_num: 0, reading_order: 0,
        bold: false, italic: false,
    }
}

#[test]
fn formula_recall_counts_formula_blocks() {
    let page = Page {
        page_num: 0,
        blocks: vec![
            make_block(BlockKind::Formula { latex: None, display: true }, ""),
            make_block(BlockKind::Paragraph, "Normal text"),
        ],
        images: vec![], formulas: vec![], tables: vec![],
    };
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 2,
        expected_headings: vec![],
        expected_tables: 0,
    };
    let m = compute_page_metrics(&page, &expectation);
    // found 1 formula block, expected 2 → recall = 0.5
    assert!((m.formula_recall - 0.5).abs() < 0.01, "formula recall = {}", m.formula_recall);
}

#[test]
fn heading_accuracy_counts_exact_matches() {
    use pdf_processor::eval::fixtures::ExpectedHeading;
    let page = Page {
        page_num: 0,
        blocks: vec![
            make_block(BlockKind::Heading { level: 1 }, "Introduction"),
            make_block(BlockKind::Heading { level: 2 }, "Background"),
            make_block(BlockKind::Heading { level: 1 }, "Wrong level"),
        ],
        images: vec![], formulas: vec![], tables: vec![],
    };
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 0,
        expected_headings: vec![
            ExpectedHeading { text: "Introduction".to_string(), level: 1 },
            ExpectedHeading { text: "Background".to_string(), level: 2 },
        ],
        expected_tables: 0,
    };
    let m = compute_page_metrics(&page, &expectation);
    // Both match exactly → accuracy = 1.0
    assert!((m.heading_accuracy - 1.0).abs() < 0.01, "heading accuracy = {}", m.heading_accuracy);
}

#[test]
fn table_found_when_coordinate_table_present() {
    use pdf_processor::document::types::DetectedTable;
    let page = Page {
        page_num: 0,
        blocks: vec![
            make_block(BlockKind::CoordinateTable {
                table: DetectedTable {
                    bbox: bbox(), rows: vec![],
                }
            }, ""),
        ],
        images: vec![], formulas: vec![], tables: vec![],
    };
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 0,
        expected_headings: vec![],
        expected_tables: 1,
    };
    let m = compute_page_metrics(&page, &expectation);
    assert!(m.table_found, "table must be found when CoordinateTable block present");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test eval_integration formula_recall heading_accuracy table_found 2>&1 | tail -20`
Expected: compile errors — `metrics` module and `compute_page_metrics` not found.

- [ ] **Step 3: Implement `src/eval/metrics.rs`**

```rust
use crate::document::types::{BlockKind, Page};
use crate::eval::fixtures::PageExpectation;

#[derive(Debug, Default)]
pub struct PageMetrics {
    pub formula_recall: f32,
    pub heading_accuracy: f32,
    /// Whether at least one table was found when expected_tables > 0.
    pub table_found: bool,
    pub table_expected: bool,
}

#[derive(Debug, Default)]
pub struct DocMetrics {
    pub pages: usize,
    pub formula_recall_sum: f32,
    pub heading_accuracy_sum: f32,
    pub table_pages_expected: usize,
    pub table_pages_found: usize,
}

pub fn compute_page_metrics(page: &Page, expected: &PageExpectation) -> PageMetrics {
    // Formula recall
    let formula_found = page.blocks.iter().filter(|b| {
        matches!(b.kind,
            BlockKind::Formula { .. } | BlockKind::FormulaReview { .. })
    }).count();
    let formula_recall = if expected.expected_formula_count == 0 {
        1.0 // no expectation → vacuously correct
    } else {
        (formula_found.min(expected.expected_formula_count) as f32)
            / (expected.expected_formula_count as f32)
    };

    // Heading accuracy (exact text+level match, case-insensitive trim)
    let heading_accuracy = if expected.expected_headings.is_empty() {
        1.0
    } else {
        let mut matches = 0usize;
        for exp in &expected.expected_headings {
            let found = page.blocks.iter().any(|b| {
                if let BlockKind::Heading { level } = b.kind {
                    level == exp.level
                        && b.text.trim().to_lowercase() == exp.text.trim().to_lowercase()
                } else {
                    false
                }
            });
            if found { matches += 1; }
        }
        matches as f32 / expected.expected_headings.len() as f32
    };

    // Table detection (page-level)
    let table_found = page.blocks.iter().any(|b| {
        matches!(b.kind, BlockKind::CoordinateTable { .. })
    });
    let table_expected = expected.expected_tables > 0;

    PageMetrics { formula_recall, heading_accuracy, table_found, table_expected }
}

pub fn aggregate(page_metrics: &[PageMetrics]) -> DocMetrics {
    let mut doc = DocMetrics { pages: page_metrics.len(), ..Default::default() };
    for m in page_metrics {
        doc.formula_recall_sum += m.formula_recall;
        doc.heading_accuracy_sum += m.heading_accuracy;
        if m.table_expected {
            doc.table_pages_expected += 1;
            if m.table_found { doc.table_pages_found += 1; }
        }
    }
    doc
}

pub fn print_report(doc_name: &str, doc: &DocMetrics) {
    let pages = doc.pages.max(1) as f32;
    println!(
        "{doc_name}\n  formula recall:  {:.1}%\n  heading accuracy: {:.1}%\n  table recall:    {:.1}% ({}/{} pages)\n",
        100.0 * doc.formula_recall_sum / pages,
        100.0 * doc.heading_accuracy_sum / pages,
        if doc.table_pages_expected == 0 { 100.0 }
        else { 100.0 * doc.table_pages_found as f32 / doc.table_pages_expected as f32 },
        doc.table_pages_found,
        doc.table_pages_expected,
    );
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test eval_integration formula_recall heading_accuracy table_found`
Expected: all three PASS.

- [ ] **Step 5: Commit**

```bash
git add src/eval/metrics.rs tests/eval_integration.rs
git commit -m "feat(eval): page and doc metrics computation"
```

---

### Task 3: Implement the eval runner

**Stage 4 adjustment:** before computing metrics, add a public or crate-visible pipeline function that returns the fully built local `Document` for a PDF and options. Use that function from eval. Do not duplicate the extractor/classifier flow in the eval runner, because that would miss geometry table detection and formula exclusion.

**Files:**
- Create: `src/eval/runner.rs`
- Test: `tests/eval_integration.rs`

**Ownership:**
- In scope: `run_eval()` — in-process pipeline call + metric collection per fixture
- Out of scope: CLI wiring, metric printing

**Assumption refs:** `A2`, `A6`

- [ ] **Step 1: Write a failing test for runner (using a minimal fixture + real PDF)**

In `tests/eval_integration.rs`, add:

```rust
#[test]
fn runner_returns_metrics_for_fixture_dir() {
    use pdf_processor::eval::runner::run_eval;
    use pdf_processor::eval::fixtures::load_fixtures;

    // Use the bundled test fixtures directory.
    let fixtures_dir = std::path::Path::new("tests/eval_fixtures");
    if !fixtures_dir.exists() {
        // No fixtures available in CI — skip silently.
        return;
    }
    let fixtures = load_fixtures(fixtures_dir).expect("load fixtures");
    if fixtures.is_empty() { return; }

    let results = run_eval(&fixtures);
    assert!(!results.is_empty());
    // At minimum each result has a doc name and at least one page.
    for r in &results {
        assert!(!r.doc_name.is_empty());
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test eval_integration runner_returns_metrics 2>&1 | tail -10`
Expected: compile error — `runner` module not found.

- [ ] **Step 3: Implement `src/eval/runner.rs`**

```rust
use crate::eval::fixtures::FixtureFile;
use crate::eval::metrics::{aggregate, compute_page_metrics, DocMetrics, PageMetrics};
use crate::pipeline::process_pdf_to_document;

pub struct DocResult {
    pub doc_name: String,
    pub metrics: DocMetrics,
}

pub fn run_eval(fixtures: &[FixtureFile]) -> Vec<DocResult> {
    fixtures.iter().map(|f| run_one(f)).collect()
}

fn run_one(fixture: &FixtureFile) -> DocResult {
    let pdf_path = fixture.pdf_path();
    let doc_name = pdf_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown")
        .to_string();

    let raw_pages = match PdfExtractor::extract_pages(&pdf_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("eval: failed to extract {}: {e}", pdf_path.display());
            return DocResult { doc_name, metrics: DocMetrics::default() };
        }
    };

    let ordered = build_xycut_order(raw_pages);
    let mut page_metrics: Vec<PageMetrics> = Vec::new();

    for raw_page in &ordered {
        // Find the expectation for this page (1-indexed).
        let page_1indexed = raw_page.page_num + 1;
        let expectation = match fixture.pages.iter().find(|e| e.page == page_1indexed) {
            Some(e) => e,
            None => continue, // page not covered by fixture — skip
        };

        // Classify the page (no pdfium-metadata available in eval — default build).
        let body_font_size = body_font_size(&ordered);
        let clf = Classifier::new(body_font_size);
        let classified_page = clf.classify_page(&raw_page.blocks, raw_page);

        let m = compute_page_metrics(&classified_page, expectation);
        page_metrics.push(m);
    }

    DocResult {
        doc_name,
        metrics: aggregate(&page_metrics),
    }
}

fn body_font_size(pages: &[crate::document::types::RawPage]) -> f32 {
    // Mode of font sizes across all blocks — same heuristic as the main pipeline.
    let sizes: Vec<ordered_float::OrderedFloat<f32>> = pages.iter()
        .flat_map(|p| p.blocks.iter())
        .map(|b| ordered_float::OrderedFloat(b.font_size))
        .collect();
    if sizes.is_empty() { return 12.0; }
    let mut counts = std::collections::HashMap::new();
    for s in &sizes { *counts.entry(s).or_insert(0usize) += 1; }
    **counts.into_iter().max_by_key(|e| e.1).map(|e| e.0).unwrap_or(&ordered_float::OrderedFloat(12.0))
}
```

Note: check if `ordered_float` is already a dependency; if not, replace with a simpler sort-and-mode:

```bash
grep "ordered-float\|ordered_float" Cargo.toml
```

If not present, use this alternative body_font_size:

```rust
fn body_font_size(pages: &[crate::document::types::RawPage]) -> f32 {
    let mut freq: std::collections::HashMap<u32, usize> = std::collections::HashMap::new();
    for p in pages {
        for b in &p.blocks {
            *freq.entry((b.font_size * 10.0) as u32).or_insert(0) += 1;
        }
    }
    freq.into_iter()
        .max_by_key(|e| e.1)
        .map(|e| e.0 as f32 / 10.0)
        .unwrap_or(12.0)
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test eval_integration runner_returns_metrics`
Expected: PASS (skips gracefully if no fixtures present).

Run: `cargo test 2>&1 | tail -5`
Expected: no regressions.

- [ ] **Step 5: Commit**

```bash
git add src/eval/runner.rs tests/eval_integration.rs
git commit -m "feat(eval): in-process eval runner (extract + classify + metrics)"
```

---

### Task 4: Add `pdfp eval` CLI command

**Files:**
- Modify: `src/cli.rs` — add `Eval(EvalArgs)` to `Commands`
- Modify: `src/main.rs` — dispatch `Commands::Eval`

**Ownership:**
- In scope: `EvalArgs` struct, `Commands::Eval` variant, dispatch in `main.rs`
- Out of scope: runner logic, metric printing

**Assumption refs:** `A6`

- [ ] **Step 1: Write failing CLI smoke test**

In `tests/eval_integration.rs`, add:

```rust
#[test]
fn eval_command_errors_on_missing_dir() {
    use assert_cmd::Command;
    let mut cmd = Command::cargo_bin("pdfp").unwrap();
    let output = cmd
        .args(["eval", "/tmp/this_dir_does_not_exist_pdfp_eval_test"])
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "eval with missing dir must exit non-zero"
    );
}
```

Add `assert_cmd = "2"` to `[dev-dependencies]` in `Cargo.toml`.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test eval_integration eval_command_errors 2>&1 | tail -10`
Expected: error — `pdfp eval` subcommand not found (exits 2 with usage error).

- [ ] **Step 3: Add `EvalArgs` and `Commands::Eval` to cli.rs**

In `src/cli.rs`, add to the `Commands` enum:

```rust
/// Run quality evaluation against a directory of fixture JSON files.
Eval(EvalArgs),
```

Add the args struct:

```rust
#[derive(Args, Debug)]
pub struct EvalArgs {
    /// Directory containing fixture JSON files and their PDFs.
    pub dir: std::path::PathBuf,
}
```

- [ ] **Step 4: Add dispatch in main.rs**

In the `Commands` match in `main.rs`:

```rust
Commands::Eval(args) => {
    if !args.dir.exists() {
        eprintln!("error: fixture dir does not exist: {}", args.dir.display());
        std::process::exit(1);
    }
    let fixtures = eval::fixtures::load_fixtures(&args.dir)
        .map_err(|e| VtvError::Io(e.to_string().into()))?;
    if fixtures.is_empty() {
        println!("no fixture files found in {}", args.dir.display());
        return Ok(());
    }
    let results = eval::runner::run_eval(&fixtures);
    for r in &results {
        eval::metrics::print_report(&r.doc_name, &r.metrics);
    }
    println!("evaluated {} document(s)", results.len());
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test eval_integration eval_command_errors`
Expected: PASS.

Run: `cargo build && target/debug/pdfp eval --help`
Expected: shows `eval` subcommand help text with `<DIR>` argument.

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/main.rs tests/eval_integration.rs Cargo.toml
git commit -m "feat: add pdfp eval subcommand"
```

---

### Task 5: Add example fixtures and update TESTING.md

**Files:**
- Create: `tests/eval_fixtures/sample.json`
- Create: `tests/eval_fixtures/README.md` (instructions for adding new fixtures)
- Modify: `docs/TESTING.md`

**Ownership:**
- In scope: example fixture, documentation
- Out of scope: eval runner logic, metrics

**Assumption refs:** none

- [ ] **Step 1: Write a test that verifies the example fixture is valid JSON**

In `tests/eval_integration.rs`, add:

```rust
#[test]
fn example_fixture_is_valid_json() {
    let path = std::path::Path::new("tests/eval_fixtures/sample.json");
    if !path.exists() { return; } // skip if not yet created
    let json = std::fs::read_to_string(path).expect("read sample.json");
    let _: pdf_processor::eval::fixtures::FixtureFile =
        serde_json::from_str(&json).expect("sample.json must be valid FixtureFile JSON");
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test eval_integration example_fixture 2>&1 | tail -10`
Expected: SKIP (file not found) or FAIL (invalid JSON).

- [ ] **Step 3: Create example fixture**

Create `tests/eval_fixtures/sample.json`:

```json
{
  "pdf": "sample.pdf",
  "pages": [
    {
      "page": 1,
      "expected_formula_count": 0,
      "expected_headings": [
        { "text": "Abstract", "level": 1 }
      ],
      "expected_tables": 0
    }
  ]
}
```

Note: `sample.pdf` does not need to exist for the JSON validation test. The runner gracefully skips fixture files whose PDFs are missing (logs an error and returns empty metrics). Place real test PDFs alongside the JSON when available.

Create `tests/eval_fixtures/README.md`:

```markdown
# Evaluation Fixtures

Each `.json` file describes one PDF's expected content.

## Schema

```json
{
  "pdf": "relative/path/to/file.pdf",
  "pages": [
    {
      "page": 1,
      "expected_formula_count": 2,
      "expected_headings": [{"text": "Introduction", "level": 1}],
      "expected_tables": 1
    }
  ]
}
```

Field notes:
- `page`: 1-indexed.
- `expected_formula_count`: total formula/review blocks expected on this page.
- `expected_headings`: exact text (case-insensitive, trimmed) + heading level.
- `expected_tables`: 1 if at least one table is expected; 0 otherwise.

## Adding a new fixture

1. Place the PDF in this directory.
2. Run `pdfp inspect <pdf>` to identify page content.
3. Create a `.json` file with expectations for the pages you want to measure.
4. Run `pdfp eval tests/eval_fixtures/` to check recall/accuracy.
```

- [ ] **Step 4: Update `docs/TESTING.md`**

Add a `## Evaluation (pdfp eval)` section:

```markdown
## Evaluation (`pdfp eval`)

`pdfp eval <fixtures-dir>` runs the local pipeline against a set of fixture PDFs
and reports precision/recall for formulas, headings, and tables.

```bash
pdfp eval tests/eval_fixtures/
```

Output example:
```
paper.pdf
  formula recall:   72.5%
  heading accuracy: 90.0%
  table recall:     50.0% (1/2 pages)
```

Fixture format is described in `tests/eval_fixtures/README.md`.
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --test eval_integration example_fixture`
Expected: PASS.

Run: `cargo test 2>&1 | tail -5`
Expected: no regressions.

- [ ] **Step 6: Commit**

```bash
git add tests/eval_fixtures/ docs/TESTING.md tests/eval_integration.rs
git commit -m "docs(eval): example fixture and TESTING.md section"
```

---

### Task 6 (final): Spec Acceptance + Post-Implementation Review

**Files:**
- Modify: `docs/TESTING.md` or create `.warden/specs/2026-05-11-stage7-evaluation-spec.md` (fill Known Limitations and Post-Implementation Review)

- [ ] **Step 1: Re-read the acceptance criteria**

Criteria from the goal: (1) `pdfp eval <dir>` runs and prints a metrics report, (2) fixture loading handles missing PDFs gracefully, (3) all tests pass, (4) Clippy clean.

- [ ] **Step 2: Run every acceptance item in one batch**

```bash
# A: Build
cargo build 2>&1 | tail -3

# B: All tests pass
cargo test 2>&1 | tail -10

# C: Clippy clean
cargo clippy -- -D warnings 2>&1 | tail -5

# D: CLI help shows eval subcommand
target/debug/pdfp --help | grep eval

# E: Eval runs on example fixture dir (PDF missing → graceful skip)
target/debug/pdfp eval tests/eval_fixtures/ 2>&1

# F: Eval with missing dir exits non-zero
target/debug/pdfp eval /tmp/no_such_dir_pdfp 2>&1; echo "exit: $?"
```

- [ ] **Step 3: Resolve every failure**

Fix or document as Known Limitation with root cause and ≥2 approaches tried.

- [ ] **Step 4: Fill Post-Implementation Review**

Three subsections: Acceptance results, Scope drift, Refactor proposals.

- [ ] **Step 5: Surface limitations to user**

Summarise any acceptance items that did not pass.

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "docs: stage 7 post-implementation review"
```
