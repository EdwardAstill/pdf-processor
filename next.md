# Next: DNV Formula Extraction

## Status

The DNV-ST-N001 conversion did not get formulas fully correct. The current
formula path is useful as an audit and crop generator, but it is not yet a
reliable formula extractor or LaTeX reconstructor.

Current audit output:

- Source: `/home/eastill/projects/literature/standards/pdfs/marine-operations-lifting-transport/DNV-ST-N001_2018 - Marine operations and marine warranty.pdf`
- Markdown: `/tmp/pdfp-verify-2026-05-05/dnv-formulas/DNV-ST-N001_2018 - Marine operations and marine warranty/DNV-ST-N001_2018 - Marine operations and marine warranty.md`
- Formula debug directory: `/tmp/pdfp-verify-2026-05-05/dnv-formulas/DNV-ST-N001_2018 - Marine operations and marine warranty/debug/formulas/`
- Source page renders: `/tmp/pdfp-verify-2026-05-05/source-pages/`

Counts from the latest run:

- 699-page Markdown generated.
- 699 formula JSON ledger files generated.
- 3623 formula crop PNGs generated.
- No heuristic `$$` blocks injected in `--formulas auto`.
- Candidate split: 1090 `local-candidate`, 2533 `needs-review`.

## What Worked

Some real formula regions are detected and cropped cleanly.

Examples:

- Page 130 weight formulas are visually correct in the crops.
  - Source render: `/tmp/pdfp-verify-2026-05-05/source-pages/dnv-page130.png`
  - Crop example: `debug/formulas/page130_formula4.png`
  - Crop text: `W_Report, Factored <= W_ud / gamma_Weight`
- Page 675 padeye formula is detected and cropped cleanly.
  - Source render: `/tmp/pdfp-verify-2026-05-05/source-pages/dnv-page675.png`
  - Crop example: `debug/formulas/page675_formula12.png`
  - Formula visible in source: `R_pad = (R_pl * t_pl + 2 * R_ch * t_ch) / t`
- Page 389 has formula-like table expressions that are correctly cropped as
  visible math/table cells.
  - Source render: `/tmp/pdfp-verify-2026-05-05/source-pages/dnv-page389.png`
  - Crop example: `debug/formulas/page389_formula3.png`

## What Failed

### Tables Are Often Flagged As Formulas

Pages 69-71 contain alpha-factor tables. They are flagged heavily because the
table cells contain symbols such as `H_s`, `<`, `<=`, and `T_POP`.

This is useful for audit, but it is not formula extraction. These regions
should probably be routed to table extraction first, not formula extraction.

Example:

- Source render: `/tmp/pdfp-verify-2026-05-05/source-pages/dnv-page69.png`
- Crop example: `debug/formulas/page69_formula6.png`

### References Are False Positives

Pages 596-598 are mostly references. Many entries are falsely flagged because
slashes, numbers, hyphens, standards identifiers, and dates look math-like to
the current detector.

Example:

- Source render: `/tmp/pdfp-verify-2026-05-05/source-pages/dnv-page597.png`
- False-positive crop: `debug/formulas/page597_formula8.png`
- Text: `/34/ DNV-RU-OU-0300 (2018) Fleet in service`

### Some Real Formulas Are Missed Entirely

The important failure is missed displayed equations.

Page 670 has a visible equation in the rendered source page:

- Source render: `/tmp/pdfp-verify-2026-05-05/source-pages/dnv-page670.png`
- Current formula ledger for page 670 only captured the footer.
- `pdftotext -layout` also misses the equation.

This means the current word-based formula detector cannot rely on embedded PDF
text alone. Some formulas are visually present but not available as ordinary
words through the extraction path.

Markdown evidence:

- Around the 2-hook lift derivation, the Markdown says `Hence:` and then the
  formula is missing.
- The page 670 source render clearly shows the formula above the note.

## Interpretation

The current local formula feature is doing the right first-stage job:

- It finds many formula-like regions.
- It gives reviewable crops.
- It avoids polluting Markdown in `--formulas auto`.

But it is not enough for standards processing because:

- It has too many false positives from tables and references.
- It does not classify table-math versus standalone equations.
- It misses formulas that are not exposed as normal text words.
- It does not reconstruct LaTeX.

## Architecture Assessment

The architecture has a lot of moving parts, but it is not accidental mess. The
split is mostly domain-driven:

- `pdf/` extracts raw PDF text, geometry, images, and metadata.
- `layout/` turns raw geometry into reading order, tables, and structural
  blocks.
- `figure/` detects and renders visual figure regions.
- `formula/` audits formula-like regions and writes crops.
- `ocr/` prepares searchable PDFs when a scan has no usable text layer.
- `hybrid/` routes selected hard pages to an external backend.
- `render/` turns the document model into Markdown.
- `processor/` owns non-conversion PDF operations such as search, page editing,
  imposition, resize, inspect, and OCR command handling.

The weak point is not the module split. The weak point is that confidence and
review policy are still spread across separate features. The next refactor
should make uncertainty a shared contract: every ambiguous region should become
either a safe fallback, a debug artifact, or an explicit review marker.

`--conservative` is now the user-facing safe preset. It should remain the mode
for standards processing when wrong reconstruction is worse than missing or
flagged content. It resolves conversion to:

- embedded figures only
- layout table fallback
- formula audit mode
- no heuristic formula Markdown rendering

## Recommended Next Change

Build a second formula path that is image-based, not only word-based.

Suggested implementation sequence:

1. Add page-render based formula region detection.
   - Render pages to images.
   - Detect visually isolated equation bands.
   - Use text-block gaps, centered dense symbol bands, and surrounding prose
     cues such as `Hence:`, `formulae below`, `where:`, and equation captions.
   - This should catch page 670-style formulas that text extraction misses.

2. Suppress known false positives before counting formula candidates.
   - Ignore bibliography/reference pages or reference-style lines such as
     `/34/ DNV-RU-OU-0300...`.
   - Do not classify full table rows as formulas when table detection already
     owns the region.
   - Treat table cells with math symbols as table content, not standalone
     formulas.

3. Add a formula sidecar contract.
   - Input: crop PNG plus page/crop metadata.
   - Output: LaTeX, confidence, backend name, and failure reason.
   - First backend candidates: Docling for full-page enrichment; formula OCR
     sidecar for per-crop recognition.

4. Add review markers in Markdown for unresolved formulas.
   - For pages with detected/missed formula gaps, insert a visible comment such
     as:

     ```markdown
     <!-- formula-review: page=670 reason="visible equation not recovered" crop="..." -->
     ```

   - Do not silently omit formula-critical regions in standards output.

5. Add regression fixtures from DNV pages.
   - Page 130: should detect/crop displayed weight formulas.
   - Page 389: should classify formula-like table rows as table content.
   - Page 597: should not flag references as formulas.
   - Page 670: should detect the visible equation even though text extraction
     misses it.
   - Page 675: should detect/crop the padeye formula.

## Acceptance Criteria

- `pdfp convert DNV-ST-N001 ... --debug-formulas --formulas auto` still writes
  audit JSON and crops without injecting heuristic `$$` into Markdown.
- Page 670 has at least one true formula crop for the visible equation.
- Page 597 reference entries are no longer counted as formula candidates.
- Page 69 alpha-factor table rows are handled as tables, not standalone
  formulas.
- Markdown contains explicit review markers for formula regions that cannot be
  reconstructed.
- Existing formula tests and full `cargo test` pass.
