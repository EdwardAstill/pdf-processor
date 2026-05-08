# Figure Snapshot Extraction Plan

status: completed
date: 2026-05-05
shape: research-plan-execute-review

## Task

Improve PDF-to-Markdown image output so `pdfp` can emit complete visual figure snapshots, not only raw embedded image objects.

## Research Inputs

- Research report: `.warden/research/figure-snapshot-extraction/REPORT.md`
- Evidence ledger: `.warden/research/figure-snapshot-extraction/evidence.jsonl`
- Contradictions: `.warden/research/figure-snapshot-extraction/contradictions.md`
- Existing PDF internals note: `docs/pdf-internals.md`
- Existing quality baseline: `.warden/research/local-ocr-and-quality-plan/baseline/current-quality/report.json`

## Assumptions

- id: A1
  statement: Current embedded image extraction should remain available because users may still want raw raster objects.
  type: behavior
  source: local code and research report.
  check: `pdfp convert --help` still exposes a mode equivalent to current image extraction.
  if false: Treat this as a breaking behavior change and update docs/tests accordingly.
  owner: Task 1

- id: A2
  statement: Figure snapshots should be opt-in initially to avoid surprising runtime and output-size changes.
  type: product
  source: prior performance baseline and PyMuPDF4LLM DPI/runtime guidance.
  check: default conversion output remains image-object based.
  if false: Rebaseline all corpus outputs and document the default change.
  owner: Task 1

- id: A3
  statement: Pure-Rust MuPDF region rendering is likely viable, but must be prototyped before committing to the full feature.
  type: implementation
  source: PyMuPDF/MuPDF docs and local `mupdf` crate inspection.
  check: prototype can render one page bbox to a valid PNG without Poppler or `mutool`.
  if false: switch Task 3 to a renderer sidecar design and update install docs.
  owner: Task 3

- id: A4
  statement: The existing PDFs are enough for first-pass quality and performance checks, but not enough for a permanent golden suite.
  type: test
  source: existing baseline over `example/pdf`.
  check: selected fixtures include at least one composite/raster figure and one vector-heavy figure.
  if false: add a small purpose-built fixture before marking the feature complete.
  owner: Task 5

## Behavior Contract

Default conversion remains compatible with today:

```bash
pdfp convert input.pdf -o out
```

Figure snapshot extraction is selectable:

```bash
pdfp convert input.pdf -o out --figures embedded
pdfp convert input.pdf -o out --figures snapshot
pdfp convert input.pdf -o out --figures both
pdfp convert input.pdf -o out --figures none
```

Additional controls:

```bash
pdfp convert input.pdf -o out --figures snapshot --figure-dpi 200 --figure-padding 8
pdfp convert input.pdf -o out --figures snapshot --debug-figures
```

Snapshot mode writes complete visual regions as `images/pageN_figM.png`. Embedded mode writes current raw image objects as `images/pageN_imgM.png`.

## Task 1: Define CLI And Data Model

Block: execute
Skill: rust
Status: completed

Instruction:

Add a `FigureMode` enum and CLI flags:

- `--figures <embedded|snapshot|both|none>` default `embedded`
- `--figure-dpi <N>` default `200`
- `--figure-padding <POINTS>` default `8`
- `--debug-figures`

Add structured types for figure candidates and rendered figure assets. Keep these separate from `ImageRef`, because `ImageRef` means embedded image object and a figure snapshot means rendered page region.

Acceptance:

- `cargo test --test cli_help` verifies the flags appear in help.
- Default conversion behavior remains equivalent to current embedded image extraction.
- `FigureMode::Embedded` and `--no-images` precedence is explicit in tests.

## Task 2: Detect Figure Candidates

Block: execute
Skill: rust / test-driven-development
Status: completed

Instruction:

Add a figure candidate detector that uses current layout data:

- embedded image bboxes as visual seeds,
- caption text block detection for `Figure`, `Fig.`, `Table`, `Exhibit`, and `Plate`,
- proximity/overlap grouping,
- multi-panel merge heuristics,
- tiny/decorative image suppression,
- page-bound clamping with padding.

Emit debug JSON when `--debug-figures` is set.

Acceptance:

- Unit tests cover caption below, caption above, multi-image panel merge, tiny decorative rejection, and page-bound clamping.
- Debug JSON includes page number, bbox, caption bbox, seed block IDs, confidence, and reason.

## Task 3: Prototype And Add Region Rendering

Block: execute
Skill: rust / systematic-debugging
Status: completed

Instruction:

Prototype clipped page-region rendering through the existing Rust MuPDF binding:

1. Render a known bbox from one existing PDF page to PNG.
2. Prefer display-list plus clip-aware draw-device rendering.
3. If the binding blocks that path, document the blocker and switch to the smallest viable fallback.

Then integrate it into conversion as `images/pageN_figM.png`.

Acceptance:

- Integration test renders at least one snapshot PNG from an existing fixture.
- PNG outputs are non-empty and have PNG magic bytes.
- Snapshot filenames are stable.
- No Poppler/`mutool` dependency is introduced unless the pure-Rust prototype fails and the plan is updated.

## Task 4: Integrate Markdown Output

Block: execute
Skill: rust
Status: completed

Instruction:

Thread rendered figure assets through the document pipeline and Markdown renderer:

- `embedded`: current `pageN_imgM.png` links only.
- `snapshot`: `pageN_figM.png` links only.
- `both`: include snapshot assets and preserve embedded images for debug/inspection.
- `none`: no image links or image writes.

Keep captions as Markdown text when the caption is extractable. Avoid duplicating the same caption as both text and image content by default.

Acceptance:

- Snapshot mode Markdown references `images/pageN_figM.png`.
- Snapshot-only mode does not emit raw `pageN_imgM.png` links.
- Existing figure/caption Markdown tests stay green or are updated with intentional behavior.

## Task 5: Benchmark Against Existing PDFs

Block: execute
Skill: performance-profiling
Status: completed

Instruction:

Run the existing quality/performance harness for:

- default embedded mode,
- `--figures snapshot`,
- `--figures both`,
- `--figures none`.

Use at least:

- `attention.pdf`,
- `clip.pdf`,
- `resnet.pdf`,
- the PDF/UA magazine fixture if present.

Compare runtime, output bytes, image count, Markdown image references, and selected visual correctness.

Acceptance:

- A report is written under `.warden/research/figure-snapshot-extraction/baseline/`.
- The report identifies at least one fixture where snapshot mode is better than embedded mode.
- The report identifies any unacceptable runtime/output-size regression before default behavior changes.

## Task 6: Update Documentation

Block: execute
Skill: writing
Status: completed

Instruction:

Update user docs and internals docs:

- `README.md`
- `docs/CLI.md`
- `docs/TESTING.md`
- `docs/pdf-internals.md`

Explain embedded images vs figure snapshots plainly, including what each mode can and cannot do.

Acceptance:

- Docs show example commands for every figure mode.
- Docs explain performance tradeoffs for snapshot DPI.
- Docs explain that perfect semantic figure detection is not guaranteed.

## Task 7: Verification Gate

Block: review
Skill: verification-before-completion
Status: completed

Instruction:

Run the normal repo checks plus targeted figure checks.

Acceptance:

- `cargo fmt --check`
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- targeted fixture conversions for embedded, snapshot, both, and none
- quality/performance report attached to the research directory

## Execution Notes

Start with Task 3's renderer prototype before spending too much time polishing Task 2 heuristics. If clipped rendering is blocked in the Rust binding, the CLI and candidate model remain useful, but the implementation plan needs a sidecar-renderer update before full execution.
