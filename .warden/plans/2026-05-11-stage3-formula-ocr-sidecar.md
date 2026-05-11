# Stage 3: Formula OCR Sidecar (Python Path)

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task when tasks are independent. For same-session manual execution, follow this plan directly. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire a pluggable formula OCR sidecar so that formula crops with sufficient confidence receive LaTeX reconstruction and the renderer emits `$$ ... $$` blocks instead of `<!-- formula-review -->` comments.

**Architecture:** New `src/formula/ocr.rs` defines a `FormulaSidecar` trait with a single `recognize(&Path) -> Option<String>` method. `SubprocessSidecar` implements it by shelling out to a command (default: `rapid-latex-ocr`). The pipeline calls the sidecar after crops are written, populates `FormulaCandidate.latex`, and the renderer already handles `Some(latex)` at `src/render/markdown.rs:239`. A new `--formula-sidecar <cmd>` CLI flag enables the feature; it is off by default.

**Tech Stack:** Rust, std::process::Command, rapid-latex-ocr (Python, optional), cargo test

**Recommended Skills:** test-driven-development, git

**Recommended MCPs:** none

**Status:** draft
**Refinement passes:** 0

## Assumptions

- `A1` — `FormulaCandidate.latex: Option<String>` exists and the renderer emits `$$ latex $$` when `Some`.
  Type: repo-state
  Source: `src/formula/detect.rs:18`, `src/render/markdown.rs:239` — confirmed in session
  Check: `grep -n "pub latex" src/formula/detect.rs && grep -n "Formula { latex" src/render/markdown.rs`
  If false: add `latex` field and renderer arm before proceeding.
  Owner: Task 1

- `A2` — `rapid-latex-ocr` accepts a PNG path as a positional argument and prints LaTeX to stdout.
  Type: external
  Source: github.com/RapidAI/RapidLaTeXOCR README
  Check: `pip install rapid-latex-ocr && rapid-latex-ocr --help`
  If false: adapt `SubprocessSidecar` command construction to the actual CLI interface.
  Owner: Task 2

- `A3` — Formula crops are written to disk (as PNG files) before the sidecar is called.
  Type: repo-state
  Source: `next.md` — "debug/formulas/ crops written after detection"
  Check: `grep -n "crop_path\|write_formula" src/pipeline.rs src/formula/detect.rs`
  If false: ensure crop writing precedes sidecar call in the pipeline.
  Owner: Task 3

- `A4` — Only `LocalCandidate` status candidates with confidence ≥ 70 should be sent to the sidecar to avoid wasting time on low-quality crops.
  Type: design
  Source: `next.md` — "1090 local-candidate vs 2533 needs-review"
  Check: review `FormulaStatus` enum in `src/formula/detect.rs`
  If false: adjust the confidence threshold in Task 3.
  Owner: Task 3

---

## File Map

| File | Change |
|------|--------|
| `src/formula/ocr.rs` (new) | `FormulaSidecar` trait + `SubprocessSidecar` impl |
| `src/formula/mod.rs` | Add `pub mod ocr;` |
| `src/pipeline.rs` | Call sidecar after crop writing; set `candidate.latex` |
| `src/cli.rs` | Add `--formula-sidecar <cmd>` flag |
| `tests/formula_ocr.rs` (new) | Unit tests for trait + subprocess sidecar |

---

### Task 1: Verify renderer handles Some(latex) correctly

**Files:**
- Read: `src/render/markdown.rs`
- Modify if needed: `src/render/markdown.rs`

**Ownership:**
- In scope: `BlockKind::Formula` rendering arm
- Out of scope: FormulaCandidate pipeline

**Assumption refs:** `A1`

- [ ] **Step 1: Write a test for formula block rendering**

Add to existing renderer tests in `src/render/markdown.rs`:

```rust
#[test]
fn renders_formula_with_latex_as_display_math() {
    use crate::document::types::{Block, BlockKind, Bbox, Page, Document};
    let block = Block {
        id: 0,
        bbox: Bbox::new(100.0, 300.0, 400.0, 320.0),
        text: "F = ma".into(),
        kind: BlockKind::Formula { latex: "F = ma".into(), display: true },
        font_size: 12.0,
        font_name: "Times".into(),
        page_num: 0,
        reading_order: 0,
    };
    // Build minimal document and render
    let page = Page { page_num: 0, width: 595.0, height: 842.0,
                      blocks: vec![block], override_markdown: None };
    let doc = Document { pages: vec![page], metadata: Default::default(),
                         source_path: Default::default() };
    let rendered = crate::render::markdown::render(&doc, &Default::default());
    assert!(rendered.markdown.contains("$$ F = ma $$"),
            "display formula should emit $$ ... $$, got: {}", rendered.markdown);
}
```

- [ ] **Step 2: Run test**

```bash
cargo test renders_formula_with_latex 2>&1 | tail -5
```
Expected: PASS. If FAIL, fix the renderer arm at `src/render/markdown.rs:237–242` to match.

- [ ] **Step 3: Commit (if renderer was changed)**

```bash
git add src/render/markdown.rs
git commit -m "test(render): verify formula latex rendering produces display math"
```

---

### Task 2: Implement FormulaSidecar trait and SubprocessSidecar

**Files:**
- Create: `src/formula/ocr.rs`
- Modify: `src/formula/mod.rs`

**Ownership:**
- In scope: trait definition, subprocess implementation, error handling
- Out of scope: pipeline wiring, CLI flag

**Assumption refs:** `A2`

**Invoke skill:** `test-driven-development` before starting this task.

- [ ] **Step 1: Write failing tests**

Create `tests/formula_ocr.rs`:

```rust
use pdf_processor::formula::ocr::{FormulaSidecar, SubprocessSidecar};
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn subprocess_sidecar_returns_none_when_command_fails() {
    // Use a command that exits non-zero or doesn't exist
    let sidecar = SubprocessSidecar::new("false".to_string()); // unix `false` always exits 1
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "dummy").unwrap();
    let result = sidecar.recognize(f.path());
    assert!(result.is_none(), "failed command should return None");
}

#[test]
fn subprocess_sidecar_captures_stdout_as_latex() {
    // Use `echo` to simulate a sidecar that returns LaTeX
    let sidecar = SubprocessSidecar::new("echo".to_string());
    let mut f = NamedTempFile::new().unwrap();
    writeln!(f, "dummy").unwrap();
    // `echo <path>` will print the path, not LaTeX — but we just verify Some is returned
    // A real sidecar would print LaTeX. Here we verify the mechanics.
    let result = sidecar.recognize(f.path());
    // `echo path` exits 0 and prints something, so we should get Some
    assert!(result.is_some(), "echo should return Some with stdout content");
}

#[test]
fn subprocess_sidecar_trims_whitespace_from_output() {
    // printf produces output with trailing newline — it should be trimmed
    let sidecar = SubprocessSidecar::new("printf".to_string());
    // This invocation will output "\\frac{a}{b}\n" if we pass it as an arg
    // We just verify trimming in isolation — test the normalise_latex helper
    assert_eq!(
        pdf_processor::formula::ocr::normalise_latex("  \\frac{a}{b}  \n"),
        "\\frac{a}{b}"
    );
}
```

- [ ] **Step 2: Confirm tests fail**

```bash
cargo test --test formula_ocr 2>&1 | grep -E "error|FAILED" | head -5
```

- [ ] **Step 3: Implement `src/formula/ocr.rs`**

```rust
//! Formula OCR sidecar contract and implementations.
//!
//! The `FormulaSidecar` trait decouples the pipeline from any specific
//! OCR backend. `SubprocessSidecar` shells out to a CLI tool (default:
//! `rapid-latex-ocr`). Future: `OnnxSidecar` for native inference (Stage 6).

use std::path::Path;
use std::process::Command;

/// Converts a formula crop image to a LaTeX string.
pub trait FormulaSidecar: Send + Sync {
    /// Returns the LaTeX representation of the formula in the given image,
    /// or None if recognition failed or the crop is unusable.
    fn recognize(&self, crop_path: &Path) -> Option<String>;
}

/// Calls an external CLI tool to perform formula OCR.
///
/// The command receives the crop path as its first positional argument and
/// is expected to print the LaTeX to stdout on success (exit 0).
///
/// Example: `SubprocessSidecar::new("rapid-latex-ocr".to_string())`
pub struct SubprocessSidecar {
    command: String,
}

impl SubprocessSidecar {
    pub fn new(command: String) -> Self {
        Self { command }
    }
}

impl FormulaSidecar for SubprocessSidecar {
    fn recognize(&self, crop_path: &Path) -> Option<String> {
        let output = Command::new(&self.command)
            .arg(crop_path)
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let stdout = String::from_utf8(output.stdout).ok()?;
        let latex = normalise_latex(&stdout);
        if latex.is_empty() { None } else { Some(latex.to_string()) }
    }
}

/// Trim whitespace and normalise a raw sidecar output string.
pub fn normalise_latex(raw: &str) -> &str {
    raw.trim()
}
```

Add to `src/formula/mod.rs`:
```rust
pub mod ocr;
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test formula_ocr 2>&1 | tail -10
cargo test 2>&1 | grep -E "FAILED|test result"
```

- [ ] **Step 5: Commit**

```bash
git add src/formula/ocr.rs src/formula/mod.rs tests/formula_ocr.rs
git commit -m "feat(formula): add FormulaSidecar trait and SubprocessSidecar implementation"
```

---

### Task 3: Add --formula-sidecar CLI flag and wire into pipeline

**Files:**
- Modify: `src/cli.rs`
- Modify: `src/pipeline.rs`

**Ownership:**
- In scope: CLI flag definition, pipeline call to sidecar per high-confidence crop
- Out of scope: sidecar implementation (Task 2)

**Assumption refs:** `A3`, `A4`

- [ ] **Step 1: Write a CLI integration test**

Add to existing CLI tests or `tests/cli_flags.rs`:

```rust
#[test]
fn formula_sidecar_flag_accepted_by_cli() {
    use std::process::Command;
    let out = Command::new(env!("CARGO_BIN_EXE_pdfp"))
        .args(["--help"])
        .output()
        .unwrap();
    let help = String::from_utf8(out.stdout).unwrap();
    assert!(help.contains("formula-sidecar"),
        "--formula-sidecar flag must appear in help output");
}
```

- [ ] **Step 2: Confirm test fails**

```bash
cargo test formula_sidecar_flag_accepted 2>&1 | grep -E "FAILED|error"
```

- [ ] **Step 3: Add flag to CLI**

In `src/cli.rs`, find the `ConvertArgs` struct and add:

```rust
/// Path or command name for formula OCR sidecar.
/// The command receives a crop PNG path as its first argument and should
/// print LaTeX to stdout. Example: rapid-latex-ocr
#[arg(long, value_name = "CMD")]
pub formula_sidecar: Option<String>,
```

- [ ] **Step 4: Wire sidecar into pipeline**

In `src/pipeline.rs`, after formula crops are written to disk, add:

```rust
use crate::formula::ocr::{FormulaSidecar, SubprocessSidecar};

// Build sidecar once per document (not per page)
let sidecar: Option<Box<dyn FormulaSidecar>> = cli
    .formula_sidecar
    .as_deref()
    .map(|cmd| Box::new(SubprocessSidecar::new(cmd.to_string())) as Box<dyn FormulaSidecar>);

// After writing crops, for each high-confidence local candidate:
if let Some(ref sid) = sidecar {
    for candidate in formula_candidates.iter_mut() {
        if candidate.latex.is_some() { continue; } // already has LaTeX
        if candidate.confidence < 70 { continue; }  // only high-confidence crops
        if let Some(ref crop) = candidate.crop_path {
            candidate.latex = sid.recognize(Path::new(crop));
        }
    }
}
```

- [ ] **Step 5: Run tests**

```bash
cargo test formula_sidecar_flag_accepted 2>&1 | tail -5
cargo build --release 2>&1 | grep -E "^error"
```
Expected: CLI test passes, binary builds cleanly.

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/pipeline.rs
git commit -m "feat(cli): add --formula-sidecar flag; wire SubprocessSidecar into pipeline"
```

---

### Task 4: End-to-end smoke test with rapid-latex-ocr

**Files:**
- Create: `tests/formula_sidecar_e2e.rs`

**Ownership:**
- In scope: `#[ignore]` test requiring Python + rapid-latex-ocr installed
- Out of scope: sidecar implementation

- [ ] **Step 1: Write end-to-end fixture test**

```rust
//! End-to-end: formula sidecar produces LaTeX from a real crop.
//! Requires: pip install rapid-latex-ocr

#[test]
#[ignore = "requires rapid-latex-ocr installed and test fixture"]
fn rapid_latex_ocr_produces_latex_for_simple_formula() {
    use pdf_processor::formula::ocr::{FormulaSidecar, SubprocessSidecar};
    use std::path::PathBuf;

    // Use a known test crop from the repo fixtures directory
    let crop = PathBuf::from("tests/fixtures/formula_crop_fma.png");
    if !crop.exists() {
        eprintln!("Skipping: fixture not found at {}", crop.display());
        return;
    }
    let sidecar = SubprocessSidecar::new("rapid-latex-ocr".to_string());
    let latex = sidecar.recognize(&crop);
    assert!(latex.is_some(), "rapid-latex-ocr should return Some for a valid crop");
    let s = latex.unwrap();
    assert!(!s.is_empty());
    println!("Got LaTeX: {}", s);
}
```

- [ ] **Step 2: Run to confirm ignored test compiles**

```bash
cargo test --test formula_sidecar_e2e 2>&1 | tail -5
```
Expected: 0 tests run, no compile errors.

- [ ] **Step 3: Commit**

```bash
git add tests/formula_sidecar_e2e.rs
git commit -m "test(formula): add ignored e2e test for rapid-latex-ocr sidecar"
```

---

### Task 5 (final): Acceptance + Review

- [ ] **Step 1: Full test suite**

```bash
cargo test 2>&1 | grep -E "test result|FAILED"
cargo clippy --all-targets -- -D warnings 2>&1 | grep error
```

- [ ] **Step 2: Manual smoke test**

```bash
# Install sidecar if available
pip install rapid-latex-ocr 2>/dev/null || true

# Run on a math-heavy test PDF with sidecar enabled
pdfp convert tests/fixtures/formulas.pdf \
    --formula-sidecar rapid-latex-ocr \
    --debug-formulas -o /tmp/stage3-verify/
grep -c '\$\$' /tmp/stage3-verify/**/*.md && echo "LaTeX blocks present"
```
Expected: `$$` blocks appear in Markdown where high-confidence crops exist.

- [ ] **Step 3: Verify conservative mode unchanged**

```bash
pdfp convert tests/fixtures/formulas.pdf --conservative -o /tmp/stage3-verify-conservative/
grep '$$' /tmp/stage3-verify-conservative/**/*.md && echo "UNEXPECTED" || echo "clean (no $$ in conservative)"
```
Expected: conservative mode still emits only `formula-review` comments, no `$$` blocks.

- [ ] **Step 4: Commit**

```bash
git add .
git commit -m "docs: stage 3 verification — formula OCR sidecar confirmed"
```
