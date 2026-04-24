# Spec — Auto-suppress image extraction on scan-heavy PDFs

Date: 2026-04-24

## Goal

Stop `cnv` from dumping hundreds of megabytes of per-page PNGs when the input PDF is already known to be scan-heavy. Those images are duplicates of the source PDF pages, near-useless for downstream markdown consumption, and the extractor has already surfaced a warning that the document is scan-heavy.

## Why

Observed on a real-world corpus (12 engineering standards PDFs, ~306 MB total):

- `AS 4100 Design of Structural Connections` (scan-heavy) produced an 8 KB markdown file alongside a 241 MB `images/` directory containing 342 page-sized PNGs.
- `AISC 9th WSD` (scan-heavy) produced a 12 KB markdown file alongside a 171 MB `images/` directory containing 9,120 images.

Total image payload: ~412 MB of effectively useless output. `cnv` already detects scan-heavy conditions (`src/hybrid/triage.rs` exposes `is_image_only`, `is_low_density`, and emits a `"looks scan-heavy"` warning at `src/main.rs:138`). The detection fires but is purely advisory — image extraction still runs at full cost.

## Scope

In scope:
- Add logic to suppress image extraction when the per-document triage classifies the PDF as scan-heavy.
- Add a CLI flag to override the default (turn suppression off, or force it on).
- Update the scan-heavy warning message so users understand images were skipped.

Out of scope:
- Changing the scan-heavy detection thresholds (separate concern).
- Any OCR / hybrid routing behavior.
- Per-page image suppression on a mixed PDF — this is a whole-document decision.
- Retroactive cleanup of already-written image files on an aborted run.

## Design

### Trigger condition

Reuse the existing `ScanReport` in `src/hybrid/triage.rs`. A PDF is treated as scan-heavy for the purpose of image suppression when **either** of the following is true after triage:

- `image_only_pages == pages_total` (already the existing "entirely scanned" check).
- `pages_with_readable_text == 0 && low_density_pages > 0` (the existing `looks_scan_heavy` criterion).

These are the same conditions that already trigger the warning message, so users get consistent behavior.

### CLI surface

Add a new flag to `src/cli.rs`:

```rust
/// Control per-page image extraction when the PDF is scan-heavy (every page
/// looks like a scanned image). `auto` (default) skips image extraction on
/// scan-heavy PDFs — the extracted text is too poor to reference the images
/// meaningfully. `always` keeps the current behavior and extracts every page
/// as a PNG. `never` disables image extraction entirely (equivalent to the
/// existing --no-images).
#[arg(long, value_enum, default_value_t = ScanImagePolicy::Auto)]
pub scan_images: ScanImagePolicy,
```

`ScanImagePolicy` is a new enum with three variants: `Auto`, `Always`, `Never`. The existing `--no-images` flag continues to work; `--scan-images never` is a superset and takes precedence.

Resolution order (first match wins):
1. `--no-images` → images off.
2. `--scan-images never` → images off.
3. `--scan-images always` → images on, regardless of scan-heavy.
4. `--scan-images auto` (default) → images off when `ScanReport::looks_scan_heavy()` returns true, else on.

### Implementation

Small, local change:

1. Add `ScanImagePolicy` enum + field to `Cli` in `src/cli.rs`.
2. After triage runs in the main loop (around `src/main.rs:138`), compute an `extract_images: bool` from the policy + scan report.
3. Thread `extract_images` through to the image-writing step in the render/extraction pipeline. (Exact site to be confirmed during implementation — currently images are written during `PdfExtractor::extract_pdf`; may need a flag on the extractor or a post-extract filter.)
4. Update the existing scan-heavy warning to include a new line when images were auto-suppressed:

   ```
   warning: pdfs/X.pdf looks scan-heavy (...); local output may be poor.
            skipping image extraction (pass --scan-images always to force).
            try `--hybrid docling` for better text.
   ```

### Compatibility

- Default behavior changes: scan-heavy PDFs stop emitting images. This is an intentional quality-of-life improvement, not a regression for typical use.
- Users scripting against the old behavior can pass `--scan-images always`.
- `--no-images` semantics unchanged.

## Acceptance Criteria

1. Running `cnv pdfs/ -o out/` against a corpus containing a scan-heavy PDF produces no `images/` subdirectory for that PDF.
2. Running the same command with `--scan-images always` restores the prior behavior and writes the full `images/` dump.
3. `--no-images` continues to work as today.
4. The scan-heavy warning message explicitly mentions that images were skipped and how to re-enable them.
5. A non-scan-heavy PDF is unaffected — images are extracted as before.
6. `cargo test` remains green.

## Test Plan

- **Unit:** add tests in `src/hybrid/triage.rs` (or new test module) asserting `ScanReport::looks_scan_heavy()` returns the expected boolean on synthetic reports covering (a) all-image, (b) mixed-but-mostly-image, (c) mixed-but-mostly-text, (d) empty.
- **Integration:** add a fixture-driven test in `tests/` using one of the existing scan-heavy corpus entries. Assert:
  - Default run writes no `images/` subdir.
  - `--scan-images always` writes the expected number of images.
  - `--no-images` writes no images in both modes.
- **Snapshot:** update any existing markdown snapshots that referenced `images/pageN_imgM.png` for scan-heavy fixtures — those references become unreferenced.

## Risks

- **Markdown references stale images:** if the scan-heavy extractor still emits `![image](images/...)` markdown lines, suppressing the image writes will leave broken references. Mitigation: when `extract_images=false`, also skip emitting image-reference lines (or emit a placeholder comment noting the suppression).
- **False-negative on scan detection:** a mostly-scanned PDF with one OCR'd page could slip past `looks_scan_heavy`. Acceptable — the user can opt into `--scan-images never` globally.
- **User confusion over new default:** call it out prominently in the warning + changelog.

## Open questions

- Should the auto-suppression also apply when routing through `--hybrid docling`? Probably yes (docling returns its own structured output; local image dump adds nothing). Confirm during implementation.
- Should there be a dry-run mode that reports the would-be image count without writing? Out of scope for this spec; file follow-up if requested.
