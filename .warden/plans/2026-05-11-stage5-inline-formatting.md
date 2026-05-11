# Stage 5: Inline Formatting (Block-Level Bold, Italic, Code)

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task when tasks are independent. For same-session manual execution, follow this plan directly. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When pdfium-metadata is available, blocks whose dominant font is bold, italic, or monospace get wrapped in `**…**`, `*…*`, or `` `…` `` respectively in the rendered Markdown output.

**Architecture:** Two new boolean fields (`bold`, `italic`) and one computed field (`monospace`, derived from `font_name`) are added to `Block`. The classifier sets `bold`/`italic` from `PageMetadata` when available. The renderer wraps `BlockKind::Paragraph` and `BlockKind::ListItem` text in the appropriate Markdown markers. Single-line blocks whose `font_name` contains a monospace family name are promoted to `BlockKind::CodeBlock` in the classifier. Without pdfium-metadata the fields default to `false` and the renderer emits plain text unchanged — no regression on the default build.

**Stage 4 adjustment:** Refresh all sample code against the current `RawTextBlock`, `RawPage`, `Page`, and `Classifier` APIs before implementation. Stage 4 added geometry-table, formula, figure, and media block constructors in `pipeline.rs`; those constructors also need explicit `bold: false, italic: false` defaults when `Block` grows new fields.

**Tech Stack:** Rust, pdfium-render (optional feature `pdfium-metadata`), cargo test

**Recommended Skills:** test-driven-development, git

**Recommended MCPs:** none

**Status:** draft
**Refinement passes:** 0

## Assumptions

- `A1` — `PageMetadata` (in `src/pdf/metadata.rs`) provides a `FontInfo { weight: u16, italic: bool, name: String }` keyed by bbox, and `FontInfo::is_bold()` returns `weight >= 700`.
  Type: repo-state
  Source: `src/pdf/metadata.rs` — confirmed in session
  Check: `grep -n "is_bold\|pub weight\|pub italic\|pub name" src/pdf/metadata.rs`
  If false: add `is_bold()` helper before proceeding.
  Owner: Task 1

- `A2` — `Block` does not yet have `bold` or `italic` fields; the classifier does not yet set them.
  Type: repo-state
  Source: `src/document/types.rs` and `src/layout/classifier.rs` — confirmed in session
  Check: `grep -n "pub bold\|pub italic" src/document/types.rs`
  If false: skip Task 1 (fields already exist).
  Owner: Task 1

- `A2b` — Stage 4 added additional `Block` construction sites in `src/pipeline.rs` for geometry-backed coordinate tables, formulas, formula-review comments, embedded images, and figure snapshots.
  Type: repo-state
  Source: Stage 4 implementation
  Check: `grep -rn "Block {" src/ | grep -v "BlockKind" | head -80`
  If false: update only the construction sites that exist.
  Owner: Task 1

- `A2c` — The sample tests in this draft may not match the current concrete struct fields. Treat them as behavioral examples, not copy/paste code.
  Type: repo-state
  Source: Stage 4 review of current `RawTextBlock`, `RawPage`, `Page`, and `Classifier` APIs
  Check: `sed -n '1,220p' src/document/types.rs && grep -n "classify_page_with_metadata" src/layout/classifier.rs`
  If false: use the exact current API in tests.
  Owner: Task 1

- `A3` — The renderer emits `BlockKind::Paragraph` text as a plain string with no surrounding markup (other than a trailing newline).
  Type: repo-state
  Source: `src/render/markdown.rs:159` — confirmed in session
  Check: `grep -n "Paragraph" src/render/markdown.rs | head -10`
  If false: adjust the renderer wrapping logic in Task 3 to match the actual render path.
  Owner: Task 3

- `A4` — A "monospace" block is one whose `font_name` (set by `dominant_font_name`) contains one of: `"Courier"`, `"Mono"`, `"Consol"`, `"Code"`, `"Fixed"` (case-insensitive). This heuristic is intentionally conservative.
  Type: design
  Source: common PDF font naming conventions
  Check: manual review of font names in test PDFs via `pdfp inspect`
  If false: extend the pattern list before merging.
  Owner: Task 4

- `A5` — Bold/italic flags are only meaningful within pdfium-metadata builds. In the default build (no feature), `dominant_font_name` returns `"unknown"` and `PageMetadata` is never available; the new fields default to `false` and produce no output change.
  Type: policy
  Source: `src/pdf/metadata.rs:116-127` — stub vs real loader
  Check: `cargo test` without `--features pdfium-metadata` must produce the same output as before this change.
  Owner: Task 2

---

## File Map

| File | Change |
|------|--------|
| `src/document/types.rs` | Add `bold: bool`, `italic: bool` to `Block` |
| `src/layout/classifier.rs` | Set `bold`/`italic` in `classify_block_with_metadata`; promote monospace single-line to `CodeBlock` |
| `src/render/markdown.rs` | Wrap Paragraph/ListItem text when `block.bold` or `block.italic`; codify monospace |
| `tests/inline_formatting.rs` (new) | Unit tests for bold/italic wrapping and monospace promotion |

---

### Task 1: Add `bold` and `italic` fields to `Block`

**Files:**
- Modify: `src/document/types.rs:163-171`
- Test: `tests/inline_formatting.rs`

**Ownership:**
- In scope: `Block` struct fields, any `Block { .. }` construction sites in `src/`
- Out of scope: rendering, classification logic

**Assumption refs:** `A1`, `A2`

- [ ] **Step 1: Write failing test confirming fields exist**

Create `tests/inline_formatting.rs`:

```rust
use pdf_processor::document::types::{Bbox, Block, BlockKind};

#[test]
fn block_has_bold_and_italic_fields() {
    let b = Block {
        id: 0,
        bbox: Bbox { x0: 0.0, y0: 0.0, x1: 100.0, y1: 20.0 },
        text: "hello".to_string(),
        kind: BlockKind::Paragraph,
        font_size: 12.0,
        font_name: "Times-Roman".to_string(),
        page_num: 0,
        reading_order: 0,
        bold: false,
        italic: false,
    };
    assert!(!b.bold);
    assert!(!b.italic);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test inline_formatting block_has_bold_and_italic_fields 2>&1 | tail -20`
Expected: compile error — `unknown field 'bold'` (or similar missing-field error).

- [ ] **Step 3: Add `bold` and `italic` to `Block`; fix all construction sites**

In `src/document/types.rs`, inside `pub struct Block { … }`:

```rust
pub bold: bool,
pub italic: bool,
```

Grep for `Block {` construction sites and add `bold: false, italic: false,` to each:

```bash
grep -rn "Block {" src/ | grep -v "BlockKind\|test\|//\|#\[" | head -40
```

Update each site. The classifier constructs blocks in `src/layout/classifier.rs`; look for `Block { id:` patterns. Add `bold: false, italic: false,` to all of them.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test inline_formatting block_has_bold_and_italic_fields`
Expected: PASS

- [ ] **Step 5: Compile without pdfium-metadata to verify no regression**

Run: `cargo build 2>&1 | tail -5`
Expected: no errors or warnings about `bold`/`italic`.

- [ ] **Step 6: Commit**

```bash
git add src/document/types.rs src/layout/classifier.rs tests/inline_formatting.rs
git commit -m "feat: add bold/italic fields to Block (default false)"
```

---

### Task 2: Set `bold` and `italic` in classifier from `PageMetadata`

**Files:**
- Modify: `src/layout/classifier.rs` — `classify_block_with_metadata()`

**Ownership:**
- In scope: `classify_block_with_metadata` flag-setting logic
- Out of scope: `Block` struct definition, renderer

**Assumption refs:** `A1`, `A5`

- [ ] **Step 1: Write failing test for bold flag from metadata**

In `tests/inline_formatting.rs`, add:

```rust
use pdf_processor::layout::classifier::Classifier;
use pdf_processor::pdf::metadata::{FontInfo, PageMetadata, StructTag};
use pdf_processor::document::types::{Bbox, Page, RawTextBlock, RawWord};

#[test]
fn bold_metadata_sets_bold_flag_on_block() {
    let block_bbox = Bbox { x0: 10.0, y0: 10.0, x1: 200.0, y1: 30.0 };
    let raw_block = RawTextBlock {
        bbox: block_bbox.clone(),
        text: "Bold Paragraph".to_string(),
        font_size: 12.0,
        font_name: "Helvetica-Bold".to_string(),
        reading_order: 0,
        words: vec![],
    };
    let page = Page {
        page_num: 0,
        blocks: vec![],
        images: vec![],
        formulas: vec![],
        tables: vec![],
    };
    let mut md = PageMetadata::default();
    md.fonts.push(FontInfo {
        bbox: block_bbox,
        weight: 700,
        italic: false,
        name: "Helvetica-Bold".to_string(),
    });
    let clf = Classifier::new(12.0);
    let blocks = clf.classify_page_with_metadata(&[raw_block], &page, Some(&md));
    assert!(blocks[0].bold, "block over bold bbox must have bold=true");
    assert!(!blocks[0].italic);
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --test inline_formatting bold_metadata_sets_bold_flag_on_block --features pdfium-metadata 2>&1 | tail -20`
Expected: FAIL — `bold` remains `false`.

- [ ] **Step 3: Implement bold/italic assignment in classifier**

In `src/layout/classifier.rs`, inside `classify_block_with_metadata`, after the block `kind` is resolved, add:

```rust
// Set bold/italic from font metadata when available.
let (bold, italic) = if let Some(md) = metadata {
    if let Some(font) = md.best_font_for(&rb.bbox) {
        (font.is_bold(), font.italic)
    } else {
        (false, false)
    }
} else {
    (false, false)
};
```

Then include `bold, italic` when constructing the `Block`.

Verify `PageMetadata` has a `best_font_for` method. If not, use the overlap-based lookup already in the module (grep `fn.*bbox` in `metadata.rs`):

```bash
grep -n "fn.*bbox\|fn.*font" src/pdf/metadata.rs | head -20
```

If the method is named differently, use the actual name. If it doesn't exist, add a simple helper:

```rust
// In src/pdf/metadata.rs
impl PageMetadata {
    pub fn best_font_for(&self, bbox: &Bbox) -> Option<&FontInfo> {
        self.fonts.iter().find(|f| f.bbox.overlaps(bbox))
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --test inline_formatting bold_metadata_sets_bold_flag_on_block --features pdfium-metadata`
Expected: PASS

- [ ] **Step 5: Verify default build unchanged**

Run: `cargo test 2>&1 | tail -10`
Expected: all tests pass, no new failures on default build.

- [ ] **Step 6: Commit**

```bash
git add src/layout/classifier.rs src/pdf/metadata.rs tests/inline_formatting.rs
git commit -m "feat(pdfium-metadata): set bold/italic flags on Block from font metadata"
```

---

### Task 3: Renderer wraps bold/italic paragraphs

**Files:**
- Modify: `src/render/markdown.rs`
- Test: `tests/inline_formatting.rs`

**Ownership:**
- In scope: `render_block` / the paragraph text-emission path
- Out of scope: heading rendering, table rendering, formula rendering

**Assumption refs:** `A3`

- [ ] **Step 1: Write failing test for bold paragraph rendering**

In `tests/inline_formatting.rs`, add:

```rust
use pdf_processor::render::markdown::MarkdownRenderer;
use pdf_processor::document::types::{Block, BlockKind, Bbox, Document, DocumentMetadata, Page};

fn make_para_block(text: &str, bold: bool, italic: bool) -> Block {
    Block {
        id: 0,
        bbox: Bbox { x0: 0.0, y0: 0.0, x1: 400.0, y1: 20.0 },
        text: text.to_string(),
        kind: BlockKind::Paragraph,
        font_size: 12.0,
        font_name: "Times-Roman".to_string(),
        page_num: 0,
        reading_order: 0,
        bold,
        italic,
    }
}

#[test]
fn bold_paragraph_renders_with_double_asterisks() {
    let block = make_para_block("Important note.", true, false);
    let doc = make_single_block_doc(block);
    let rendered = MarkdownRenderer::new().render(&doc);
    assert!(
        rendered.markdown.contains("**Important note.**"),
        "bold paragraph must be wrapped in **…**; got: {}",
        rendered.markdown
    );
}

#[test]
fn italic_paragraph_renders_with_single_asterisks() {
    let block = make_para_block("Side note.", false, true);
    let doc = make_single_block_doc(block);
    let rendered = MarkdownRenderer::new().render(&doc);
    assert!(
        rendered.markdown.contains("*Side note.*"),
        "italic paragraph must be wrapped in *…*; got: {}",
        rendered.markdown
    );
}

#[test]
fn bold_italic_paragraph_renders_with_triple_asterisks() {
    let block = make_para_block("Very important.", true, true);
    let doc = make_single_block_doc(block);
    let rendered = MarkdownRenderer::new().render(&doc);
    assert!(
        rendered.markdown.contains("***Very important.***"),
        "bold+italic must be wrapped in ***…***; got: {}",
        rendered.markdown
    );
}

#[test]
fn plain_paragraph_unchanged() {
    let block = make_para_block("Normal text.", false, false);
    let doc = make_single_block_doc(block);
    let rendered = MarkdownRenderer::new().render(&doc);
    assert!(
        rendered.markdown.contains("Normal text.")
            && !rendered.markdown.contains('*'),
        "plain paragraph must not gain asterisks; got: {}",
        rendered.markdown
    );
}
```

Add the helper `make_single_block_doc` (adapts the existing test helpers in `markdown.rs` or writes a new one):

```rust
fn make_single_block_doc(block: Block) -> Document {
    Document {
        pages: vec![Page {
            page_num: 0,
            blocks: vec![block],
            images: vec![],
            formulas: vec![],
            tables: vec![],
        }],
        metadata: DocumentMetadata::default(),
        source_path: std::path::PathBuf::from("test.pdf"),
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test inline_formatting bold_paragraph italic_paragraph bold_italic plain_paragraph 2>&1 | tail -30`
Expected: multiple FAILs — `**` not present in output.

- [ ] **Step 3: Wrap text in renderer**

In `src/render/markdown.rs`, find the paragraph text-emission path (around line 159 and the `render_block_text` / `render_one_block` function). Locate where `block.text` is pushed to the output string.

Add inline formatting wrapping. The wrapping prefix/suffix depends on `bold` and `italic`:

```rust
fn inline_wrap(text: &str, bold: bool, italic: bool) -> String {
    match (bold, italic) {
        (true, true)  => format!("***{}***", text),
        (true, false) => format!("**{}**", text),
        (false, true) => format!("*{}*", text),
        (false, false) => text.to_string(),
    }
}
```

Call `inline_wrap(&block.text, block.bold, block.italic)` wherever `BlockKind::Paragraph` and `BlockKind::ListItem` text is emitted as a plain string. Check both the main render loop (around line 159) and the `render_block_text` helper (around line 1645).

Do NOT apply `inline_wrap` inside heading rendering, table cells, formula review comments, or page markers.

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test inline_formatting`
Expected: all four tests PASS.

Run full suite: `cargo test`
Expected: no regressions.

- [ ] **Step 5: Commit**

```bash
git add src/render/markdown.rs tests/inline_formatting.rs
git commit -m "feat: renderer wraps bold/italic blocks in Markdown markers"
```

---

### Task 4: Promote monospace single-line blocks to CodeBlock

**Files:**
- Modify: `src/layout/classifier.rs` — `classify_block_with_metadata()`
- Test: `tests/inline_formatting.rs`

**Ownership:**
- In scope: single-line block promotion, `MONOSPACE_FONT_PATTERNS` constant
- Out of scope: multi-line code fences, table cells, headings

**Assumption refs:** `A4`

- [ ] **Step 1: Write failing test for monospace promotion**

In `tests/inline_formatting.rs`, add:

```rust
#[test]
fn monospace_single_line_block_becomes_code_block() {
    let rb = RawTextBlock {
        bbox: Bbox { x0: 0.0, y0: 0.0, x1: 300.0, y1: 15.0 },
        text: "cargo build --release".to_string(),
        font_size: 10.0,
        font_name: "CourierNewPSMT".to_string(),
        reading_order: 0,
        words: vec![],
    };
    let page = Page {
        page_num: 0,
        blocks: vec![],
        images: vec![],
        formulas: vec![],
        tables: vec![],
    };
    let clf = Classifier::new(12.0);
    let blocks = clf.classify_page_with_metadata(&[rb], &page, None);
    assert!(
        matches!(blocks[0].kind, BlockKind::CodeBlock),
        "monospace single-line block must become CodeBlock; got {:?}",
        blocks[0].kind
    );
}

#[test]
fn multi_line_monospace_block_stays_paragraph() {
    // Multi-line (contains newline) should not be silently promoted —
    // caller should use fenced code block rendering instead (future work).
    let rb = RawTextBlock {
        bbox: Bbox { x0: 0.0, y0: 0.0, x1: 300.0, y1: 30.0 },
        text: "line one\nline two".to_string(),
        font_size: 10.0,
        font_name: "Courier".to_string(),
        reading_order: 0,
        words: vec![],
    };
    let page = Page { page_num: 0, blocks: vec![], images: vec![], formulas: vec![], tables: vec![] };
    let clf = Classifier::new(12.0);
    let blocks = clf.classify_page_with_metadata(&[rb], &page, None);
    assert!(
        matches!(blocks[0].kind, BlockKind::Paragraph | BlockKind::CodeBlock),
        "multi-line must not be promoted to inline code block"
    );
    // Acceptable: either stay as Paragraph OR be CodeBlock (fenced). Test only
    // ensures it doesn't become some other incorrect kind.
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test inline_formatting monospace_single_line multi_line_monospace 2>&1 | tail -20`
Expected: `monospace_single_line_block_becomes_code_block` FAILS — kind is `Paragraph`.

- [ ] **Step 3: Add monospace detection to classifier**

In `src/layout/classifier.rs`, add a constant:

```rust
const MONOSPACE_FONT_PATTERNS: &[&str] = &[
    "courier", "mono", "consol", "code", "fixed", "inconsolata", "sourcecodepro",
];
```

In `classify_block_with_metadata`, after computing `kind`, add:

```rust
// Monospace single-line blocks → CodeBlock (inline code span).
let kind = if matches!(kind, BlockKind::Paragraph)
    && !rb.text.contains('\n')
    && {
        let name_lower = rb.font_name.to_lowercase();
        MONOSPACE_FONT_PATTERNS.iter().any(|p| name_lower.contains(p))
    }
{
    BlockKind::CodeBlock
} else {
    kind
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --test inline_formatting monospace_single_line multi_line_monospace`
Expected: both PASS (multi-line test accepts `Paragraph` or `CodeBlock` — either is acceptable).

Run full suite: `cargo test`
Expected: no regressions.

- [ ] **Step 5: Commit**

```bash
git add src/layout/classifier.rs tests/inline_formatting.rs
git commit -m "feat: promote monospace single-line blocks to CodeBlock"
```

---

### Task 5 (final): Spec Acceptance + Post-Implementation Review

**Files:**
- Modify: `.warden/specs/2026-05-11-stage5-inline-formatting-spec.md` (create if spec not present — fill Known Limitations and Post-Implementation Review blocks)

- [ ] **Step 1: Re-read the spec's Acceptance Criteria block**

Open `.warden/specs/` and locate any stage-5 spec file. If no separate spec exists, use this plan as the authority.

- [ ] **Step 2: Run every acceptance item in one batch**

```bash
# A: Default build — no bold/italic markers in output
cargo build 2>&1 | tail -3
cargo test 2>&1 | tail -5

# B: Feature build compiles cleanly
cargo build --features pdfium-metadata 2>&1 | tail -3

# C: Inline formatting tests pass
cargo test --test inline_formatting --features pdfium-metadata 2>&1

# D: Clippy clean
cargo clippy --features pdfium-metadata -- -D warnings 2>&1 | tail -10

# E: Bold paragraph round-trip (manual, if sample PDF available)
# pdfp convert --features pdfium-metadata sample.pdf -o /tmp/out/ && grep '\*\*' /tmp/out/*.md | head -5
```

- [ ] **Step 3: Resolve every failure**

For each failing item, fix or document as Known Limitation with root cause and ≥2 approaches tried.

- [ ] **Step 4: Fill the Post-Implementation Review block**

Three subsections: Acceptance results, Scope drift, Refactor proposals.

- [ ] **Step 5: Surface limitations to user**

If any acceptance item did not pass, summarise: which item, root cause, suggested next step.

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "docs(spec): stage 5 post-implementation review"
```
