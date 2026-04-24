# PDF Quality Roadmap Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task. For same-session manual execution, route back through `executor`. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Improve `cnv` from a good born-digital PDF converter into a measured, regression-protected PDF-to-markdown CLI that handles academic papers, business tables, tagged PDFs, and scanned documents more reliably.

**Machine plan:** 2026-04-24-pdf-quality-roadmap.yaml

**Architecture:** Keep the Rust CLI and MuPDF-based local path as the default. Add quality in layers: first a reproducible corpus and scoring harness, then local text/layout fixes, then stronger table detection, then optional hybrid/OCR and tagged-PDF paths. Every improvement must land with a fixture or snapshot that proves the behavior and prevents silent regressions.

**Tech Stack:** Rust, MuPDF bindings, optional Docling backend, optional PDFium metadata/structure-tree feature, markdown snapshots, ignored `test-corpus/` fixtures.

---

## Research Summary

Sources checked:

- PyMuPDF/MuPDF TextPage docs: pages become blocks, lines, spans, chars; natural reading order is not guaranteed, and sorted/top-left order is a separate extraction choice. Dict/rawdict/XML forms expose font, bbox, and character-level detail that `cnv` should emulate where the Rust binding allows it. Source: https://pymupdf.readthedocs.io/en/latest/app1.html
- PyMuPDF4LLM docs: a production-grade Markdown path explicitly targets multi-column pages, tables, headings, images/vector graphics, page chunks, and automatic OCR for pages that need it. Source: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/
- Docling docs: Docling focuses on PDF layout, reading order, OCR, table structure, formulas, and a structured `DoclingDocument` with body/furniture separation and provenance. Sources: https://docling-project.github.io/docling/ and https://docling-project.github.io/docling/concepts/docling_document/
- Docling OCR examples: full-page OCR can be forced, with table structure and multiple OCR backends. This supports keeping OCR optional and page-selective. Source: https://docling-project.github.io/docling/examples/full_page_ocr/
- pdfplumber table docs: table extraction needs multiple strategies, including graphical lines, text alignment, and explicit boundaries. This is a better model than one heuristic. Source: https://github.com/jsvine/pdfplumber
- Tagged PDF research: tagged PDFs carry logical structure and reading order; when available, that should beat visual heuristics. PDFium exposes structure-tree APIs such as `FPDF_StructTree_GetForPage`. Sources: https://www.pdflib.com/pdf-knowledge-base/pdfua/tagged-pdf-basics/ and https://pdfium.googlesource.com/pdfium/+/main/public/fpdf_structtree.h

Local findings:

- `src/pdf/extractor.rs` already extracts MuPDF text blocks, char positions, dominant font size, and images.
- `src/layout/xycut.rs` is the right place for reading-order improvements.
- `src/layout/classifier.rs` currently relies heavily on size/regex heuristics; metadata and tagged structure should override those heuristics when available.
- `src/render/markdown.rs` already has many domain heuristics, but it has grown large. New render behavior should be tested in focused unit tests before expanding it.
- Current scan behavior is honest: it warns that local output is poor. The missing piece is a smoother optional OCR path and cached hybrid output.

## Success Criteria

- `cargo test` remains green.
- `git ls-files ':(glob)**/*.png' ':(glob)**/*.pdf' | wc -l` remains `0`.
- A local quality command can run the ignored fixture corpus and produce a Markdown/JSON report.
- Academic born-digital PDFs improve on known rough edges: heading levels, section spacing, paragraph reflow, hyphenation, and front matter.
- Tables improve on both bordered and borderless examples.
- Scanned PDFs either route to OCR/hybrid when enabled or produce a clear actionable warning.
- Tagged PDFs use struct-tree roles for headings, reading order, artifacts, and table/list semantics when available.

---

### Task 1: Reproducible Quality Harness

**Files:**
- Create: `scripts/quality-report.sh`
- Create: `tests/quality.rs`
- Modify: `docs/TESTING.md`
- Modify: `tests/golden.rs`

- [x] **Step 1: Write fixture-presence tests**

Add integration tests that assert missing `test-corpus/` fixtures are reported as skipped, not silently counted as real quality coverage.

Run:

```bash
cargo test --test golden -- --nocapture
```

Expected: PASS, with visible `SKIP` messages when fixtures are absent.

- [x] **Step 2: Add a quality report script**

Create `scripts/quality-report.sh` that runs `cnv` over known local fixtures and writes:

- `/tmp/cnv-quality/<case>/output.md`
- `/tmp/cnv-quality/report.json`
- summary counts: pages, warnings, extracted images, empty pages, table markers, heading counts

Run:

```bash
bash scripts/quality-report.sh
```

Expected: exits 0 if fixtures exist; exits 0 with clear skip summary if not.

- [x] **Step 3: Add baseline report docs**

Update `docs/TESTING.md` with the quality-report workflow and state that `test-corpus/` is ignored by design.

- [ ] **Step 4: Commit**

```bash
git add scripts/quality-report.sh tests/quality.rs tests/golden.rs docs/TESTING.md
git commit -m "test: add pdf quality reporting harness"
```

### Task 2: Text Cleanup And Paragraph Reflow

**Files:**
- Modify: `src/pdf/text_cleanup.rs`
- Modify: `src/pdf/extractor.rs`
- Modify: `src/render/markdown.rs`
- Test: module tests in the same files

- [x] **Step 1: Add failing tests for known text defects**

Cover:

- `3.1Encoder` -> `3.1 Encoder`
- `English-\nto-German` -> `English-to-German`
- repeated artificial blank lines inside one paragraph
- math-ish superscript/subscript output remains stable

Run:

```bash
cargo test text_cleanup render::markdown::tests::scholarly_front_matter -- --nocapture
```

Expected: at least one new test fails before implementation.

- [x] **Step 2: Implement minimal cleanup**

Implement conservative regex/string cleanup in `src/pdf/text_cleanup.rs` and render-time paragraph joining in `src/render/markdown.rs`. Avoid global markdown rewrites that could damage code blocks or tables.

- [x] **Step 3: Verify**

```bash
cargo test
cnv example/pdf/attention.pdf -o /tmp/cnv-quality-attention --no-images
rg "3\\.1Encoder|English-\\n\\nto-German" /tmp/cnv-quality-attention/attention/attention.md
```

Expected: tests pass; `rg` finds no broken spacing/hyphenation patterns.

- [ ] **Step 4: Commit**

```bash
git add src/pdf/text_cleanup.rs src/pdf/extractor.rs src/render/markdown.rs
git commit -m "fix: improve pdf text cleanup and paragraph reflow"
```

### Task 3: Heading And Section Classification

**Files:**
- Modify: `src/layout/classifier.rs`
- Modify: `src/render/markdown.rs`
- Test: `tests/golden.rs`
- Test: `tests/snapshots/attention_page_1.md`

- [x] **Step 1: Add failing heading-level tests**

Use synthetic `RawTextBlock`s and the `attention.pdf` first-page snapshot to prove:

- title is H1
- abstract is H2
- numbered top-level sections are H2, not H4
- subsection labels such as `3.1 Encoder and Decoder Stacks` have a space after the number and become a sane heading level

- [x] **Step 2: Implement section-number-aware heading logic**

In `classifier.rs`, add a numbered-section classifier before generic font-ratio mapping. Use page position, numeric pattern depth, font size, and line length.

- [x] **Step 3: Verify**

```bash
cargo test --test golden golden_snapshot_attention_page_1 -- --ignored --nocapture
cargo test layout::classifier render::markdown
```

Expected: snapshot either passes or produces a reviewable `.actual.md`; update snapshot only after manual inspection.

- [ ] **Step 4: Commit**

```bash
git add src/layout/classifier.rs src/render/markdown.rs tests/golden.rs tests/snapshots/attention_page_1.md
git commit -m "fix: improve heading classification for academic PDFs"
```

### Task 4: Table Detection Strategy Upgrade

**Files:**
- Create: `src/layout/table.rs`
- Modify: `src/layout/mod.rs`
- Modify: `src/layout/classifier.rs`
- Modify: `src/render/markdown.rs`
- Test: `tests/golden.rs`

- [x] **Step 1: Extract current table logic into `src/layout/table.rs`**

Move table-specific detection from `classifier.rs` without behavior changes.

Run:

```bash
cargo test layout::classifier render::markdown
```

Expected: PASS.

- [x] **Step 2: Add strategy enum**

Implement strategies inspired by `pdfplumber`:

- `LineGrid`: use vector/grid/line evidence when MuPDF exposes it.
- `TextAlignment`: use aligned words/blocks for borderless tables.
- `ExplicitRegion`: test-only helper for known fixture regions.

- [x] **Step 3: Add failing bordered and borderless table tests**

Use synthetic blocks first. Then add ignored fixture tests for invoice/form/financial statement PDFs.

Run:

```bash
cargo test layout::table render::markdown
cargo test --test golden -- --ignored golden_snapshot_invoice_and_form_structure
```

Expected: unit tests pass; ignored fixture test passes when local corpus exists.

- [ ] **Step 4: Commit**

```bash
git add src/layout/table.rs src/layout/mod.rs src/layout/classifier.rs src/render/markdown.rs tests/golden.rs
git commit -m "feat: add strategy-based table detection"
```

### Task 5: Selective OCR And Hybrid Cache

**Files:**
- Modify: `src/hybrid/triage.rs`
- Modify: `src/hybrid/client.rs`
- Modify: `src/cli.rs`
- Modify: `src/main.rs`
- Test: `tests/hybrid.rs`

- [x] **Step 1: Add tests for page-level routing decisions**

Cover:

- born-digital page does not route
- image-only/low-density page routes when hybrid is enabled
- math/table-heavy page routes
- cache hit skips backend call

- [x] **Step 2: Implement cache key**

Cache hybrid page results under `target/cnv-hybrid-cache/` for tests and a user cache dir for normal CLI runs. Key by file hash, page number, hybrid options, and backend URL.

- [x] **Step 3: Add CLI controls**

Add:

- `--hybrid-cache off|read-write`
- `--ocr-policy off|scan-only|all`
- keep defaults conservative: no OCR unless hybrid is enabled

- [x] **Step 4: Verify**

```bash
cargo test --test hybrid -- --nocapture
cnv example/pdf/golden__chinese_scan.pdf --hybrid docling --hybrid-policy auto -o /tmp/cnv-scan-test
```

Expected: tests pass; live command may skip/fail clearly if no Docling server is running.

- [ ] **Step 5: Commit**

```bash
git add src/hybrid src/cli.rs src/main.rs tests/hybrid.rs
git commit -m "feat: add selective hybrid OCR cache"
```

### Task 6: Tagged PDF Structure Tree Path

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/pdf/metadata.rs`
- Modify: `src/layout/classifier.rs`
- Modify: `src/main.rs`
- Test: `tests/golden.rs`

- [x] **Step 1: Add struct-tree feature tests behind `pdfium-metadata`**

Use PDF/UA fixtures when present. Tests should skip cleanly when `libpdfium` or fixtures are missing.

- [ ] **Step 2: Read structure roles**

Use the optional PDFium path to extract per-page structure elements where possible:

- role: H1-H6, P, L, LI, Table, TR, TH, TD, Figure, Artifact
- bounding boxes / MCID links if available
- alt/actual text where exposed

- [ ] **Step 3: Prefer semantic roles over visual heuristics**

Classifier should trust struct roles when overlap confidence is high. Renderer should suppress `Artifact` as furniture and render lists/tables from semantic roles when available.

- [ ] **Step 4: Verify**

```bash
cargo test --features pdfium-metadata
cargo test --test golden --features pdfium-metadata -- --ignored golden_presentation_suppresses_repeated_page_furniture
```

Expected: compile/test pass, skipped cleanly if system PDFium is absent.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/pdf/metadata.rs src/layout/classifier.rs src/main.rs tests/golden.rs
git commit -m "feat: use tagged pdf structure when available"
```

### Task 7: Image And Vector Handling

**Files:**
- Modify: `src/pdf/extractor.rs`
- Modify: `src/main.rs`
- Modify: `src/render/markdown.rs`
- Test: `tests/golden.rs`

- [ ] **Step 1: Add tests for decorative image suppression and figure retention**

Existing front-matter image tests should stay green. Add a vector-heavy fixture expectation that does not claim raster images when only vectors exist.

- [ ] **Step 2: Add image thresholds**

Add CLI/config thresholds for tiny decorative images:

- minimum width/height fraction
- edge/furniture repeated image suppression
- option to keep all images for debugging

- [ ] **Step 3: Investigate vector rendering**

Prototype page-region rendering for vector figures only if MuPDF exposes usable vector bounding boxes. If not, document the limitation and keep this as future work.

- [ ] **Step 4: Verify**

```bash
cargo test render::markdown::tests::scholarly_front_matter_drops_decorative_images_and_keeps_captioned_figure
cargo test --test golden -- --ignored golden_corpus_sweep
```

Expected: local corpus test passes when fixtures exist.

- [ ] **Step 5: Commit**

```bash
git add src/pdf/extractor.rs src/main.rs src/render/markdown.rs tests/golden.rs
git commit -m "feat: improve figure and decorative image handling"
```

### Task 8: Release Polish

**Files:**
- Modify: `README.md`
- Modify: `docs/TESTING.md`
- Modify: `Cargo.toml`
- Modify: `src/cli.rs`

- [ ] **Step 1: Document honest quality matrix**

README should say clearly:

- good: born-digital prose PDFs, many academic papers
- improving: tables, formulas, figures
- requires hybrid/OCR: scanned PDFs
- unsupported/limited: missing ToUnicode maps, complex vector-only diagrams, badly tagged PDFs

- [ ] **Step 2: Add CLI examples**

Include:

```bash
cnv paper.pdf -o out/
cnv report.pdf --no-images -o out/
cnv scanned.pdf --hybrid docling --hybrid-policy auto -o out/
```

- [ ] **Step 3: Final verification**

```bash
cargo fmt
cargo test
cargo clippy --all-targets -- -D warnings
git ls-files ':(glob)**/*.png' ':(glob)**/*.pdf' | wc -l
```

Expected: formatting clean, tests pass, clippy clean, tracked PDF/PNG count remains `0`.

- [ ] **Step 4: Commit**

```bash
git add README.md docs/TESTING.md Cargo.toml src/cli.rs
git commit -m "docs: document pdf conversion quality and limits"
```

## Recommended Order

Do Tasks 1-3 first. They improve the main local path and make every later change measurable. Do Task 4 before deeper Docling work because tables are currently a visible quality gap even for born-digital PDFs. Do Tasks 5-6 after that because they introduce optional heavier dependencies and need stable harnesses. Task 7 can run later unless image quality becomes the immediate product goal.

## Non-Goals

- Do not rewrite the CLI in TypeScript.
- Do not make OCR mandatory.
- Do not track PDF/image fixtures in git.
- Do not add a database, service, or UI until the CLI quality is stronger.
- Do not promise perfect PDF conversion; PDF extraction remains format- and source-quality-dependent.
