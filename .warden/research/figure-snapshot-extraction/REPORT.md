# Figure Snapshot Extraction Research Report

date: 2026-05-05
status: complete

## Recommendation

Add a separate "figure snapshot" path to `pdfp` instead of trying to make raw embedded-image extraction behave like full figure extraction.

The current `images/pageN_imgM.png` path should remain available because it extracts raster objects. The new path should render the visually complete page region for a detected figure and write assets like `images/page3_fig1.png`. This is the right fix for multi-panel figures, vector-heavy diagrams, panel labels, legends, and overlaid text.

## Why This Is The Right Fix

The current extractor sees `TextBlockType::Image`, decodes that embedded image object, and writes it as PNG. That means it can only recover what the PDF stores as a raster image object. It cannot recover a figure assembled from multiple image objects, vector paths, text labels, masks, or form XObjects unless those happen to be inside one image block.

External evidence matches this local limitation. A biomedical figure-extraction paper found that existing PDF image extractors often miss complete figures, split multi-panel figures, drop text labels/legends, or lose overlaid graphic elements. PyMuPDF and MuPDF document clipped page-region rendering, and Docling/PyMuPDF4LLM both expose image/figure output as a higher-level document-conversion feature.

## Proposed Behavior

Add a figure mode to Markdown conversion:

```bash
pdfp convert input.pdf -o out --figures embedded
pdfp convert input.pdf -o out --figures snapshot
pdfp convert input.pdf -o out --figures both
pdfp convert input.pdf -o out --figures none
```

Initial default should be `embedded`, matching today's behavior. Once snapshot quality is proven, the default can be revisited.

Add controls:

```bash
--figure-dpi 200
--figure-padding 8
--figure-min-width-ratio 0.10
--figure-min-height-ratio 0.06
--debug-figures
```

`--debug-figures` should emit candidate JSON with page number, bbox, caption bbox, seed blocks, confidence, and reason. That makes false positives and misses diagnosable without guessing from Markdown alone.

## Detection Strategy

Use a conservative rule-based detector first:

1. Seed candidates from embedded image bboxes already exposed by the extractor.
2. Seed candidates from vector/graphics bboxes if the MuPDF binding can expose them cleanly.
3. Detect caption text blocks using patterns like `Figure`, `Fig.`, `Table`, `Exhibit`, and `Plate`.
4. Group nearby image/vector regions and captions on the same page.
5. Merge horizontally aligned or vertically close image regions into one candidate when they look like a multi-panel figure.
6. Add padding, clamp to page bounds, and reject tiny decorative objects.

The snapshot should normally include the visual figure body, including labels, axes, legends, and panel letters. Captions should remain Markdown text where possible, rather than being baked into the snapshot by default.

## Rendering Strategy

Prototype pure-Rust clipped rendering first:

1. Build or reuse a page display list.
2. Convert the figure bbox from PDF points into a target pixel rectangle at `--figure-dpi`.
3. Render the clipped region into a pixmap with MuPDF draw-device clipping.
4. Write PNG bytes to `images/pageN_figM.png`.

The local Rust `mupdf` crate appears viable because it exposes pixmaps, display lists, draw devices, and clip-aware device construction. The main implementation risk is coordinate transforms, not the rendering concept.

Fallback only if needed: use a renderer sidecar such as Poppler or `mutool draw` for cropped page rasterization. That should stay a contingency because `pdfp` already depends on MuPDF and should avoid adding another runtime dependency unless the binding blocks the implementation.

## Testing And Performance

Use the existing corpus baseline before accepting the change:

- `attention.pdf`: should produce snapshots for early architecture/attention figures, and the snapshots should include labels/panels that raw embedded images may miss.
- `clip.pdf`: useful for image/text-heavy page rendering and output-size checks.
- `resnet.pdf`: useful as a vector-heavy diagram stress case.
- PDF/UA magazine fixture: useful for decorative image pressure and avoiding over-extraction.

Measure:

- conversion success/failure count,
- runtime for embedded vs snapshot vs both,
- output image count,
- output image bytes,
- Markdown image references,
- candidate false positives/misses on selected fixtures.

Acceptance should not require perfect figure detection. It should require that snapshot mode improves at least one known composite/vector/label-heavy case, keeps default behavior stable, and reports enough debug metadata to tune misses.

## Sources

- PyMuPDF page rendering docs: https://pymupdf.readthedocs.io/en/latest/page.html#Page.get_pixmap
- PyMuPDF device docs: https://pymupdf.readthedocs.io/en/latest/device.html
- MuPDF draw/display-list docs: https://mupdf.readthedocs.io/en/1.23.0/mupdf-js.html#drawdevice
- Docling figure export example: https://docling-project.github.io/docling/examples/export_figures/
- Docling document model docs: https://docling-project.github.io/docling/reference/docling_document/
- PyMuPDF4LLM API docs: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/api.html
- NLM figure extraction paper: https://lhncbc.nlm.nih.gov/LHC-publications/PDF/pub7055.pdf
