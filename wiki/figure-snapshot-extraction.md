---
title: "Figure Snapshot Extraction"
kind: "knowledge"
category: "wiki"
summary: "Explains why embedded PDF image extraction is not enough for complete figures, and why rendered page-region snapshots are the right local-first path."
entities: [pdfp, MuPDF, PyMuPDF, Docling, PyMuPDF4LLM, figure extraction]
updated: "2026-05-05"
---

# Figure Snapshot Extraction

Complete visual figures in PDFs are not the same thing as embedded image objects. A figure can be assembled from raster images, vector paths, text labels, legends, axes, masks, and panel letters. A converter that only extracts image XObjects can produce useful assets, but it cannot guarantee that the Markdown image link represents the figure the reader sees on the page.

## Core distinction

There are two different user intents:

- **Embedded image extraction**: recover raster image objects stored in the PDF. This is fast and useful for inspection, but it sees only the image object.
- **Figure snapshot extraction**: render the complete visual page region for a detected figure. This can include vector drawings, labels, legends, axes, and multiple panels.

`pdfp` should keep both concepts separate. In CLI terms, `--figures embedded` preserves raw image-object behavior, while `--figures snapshot` writes rendered regions such as `images/page3_fig1.png`.

## Why object extraction fails for figures

PDF pages are paint programs. A visible figure may be built from several operations:

- `/Image` XObjects for raster content
- path drawing operations for vector lines and shapes
- text drawing operations for labels, legends, axis ticks, and panel markers
- clipping masks and transforms
- multiple image panels arranged under one caption

The NLM biomedical figure-extraction paper documents the practical failure modes: common PDF image extractors split multi-panel figures, drop labels or legends, miss annotation marks, and lose overlaid graphics.

## Best local-first implementation shape

Use a layered approach:

1. Detect candidate regions from caption blocks and significant visual bboxes.
2. Merge nearby panels into one candidate.
3. Pad and clamp the candidate bbox to the page.
4. Render that page region through the local PDF renderer.
5. Keep debug JSON with bbox, caption relation, seed blocks, confidence, and reason.

The renderer should come from the existing PDF stack first. PyMuPDF documents clipped page rendering through `Page.get_pixmap(..., clip=...)`, and MuPDF exposes pixmap/draw-device primitives. That makes a MuPDF-backed Rust implementation preferable to adding a Poppler or `mutool` sidecar unless the binding blocks clipped rendering.

## Heuristic limits

Snapshot extraction is not full semantic figure understanding. It can still miss figures when:

- captions are absent or far from the visual region
- the page has dense magazine/brochure layout with many decorative images
- a vector-only figure lacks a recognizable caption
- the estimated region is blank or mostly whitespace

For this reason, snapshot mode should expose debug artifacts and avoid becoming the default until corpus benchmarks show stable quality and acceptable output size.

## Practical implications for `pdfp`

- Keep `--figures embedded` as the compatibility default.
- Add `--figures snapshot` for complete visual region capture.
- Add `--figures both` for debugging and quality comparison.
- Treat `--figures none` and `--no-images` as complete media suppression.
- Benchmark runtime, image count, and output bytes separately for embedded and snapshot modes.
- Use existing examples like `attention.pdf`, `clip.pdf`, `resnet.pdf`, and magazine/brochure fixtures to tune false positives.

## Sources

- PyMuPDF clipped rendering: https://pymupdf.readthedocs.io/en/latest/page.html#Page.get_pixmap
- PyMuPDF devices: https://pymupdf.readthedocs.io/en/latest/device.html
- PyMuPDF4LLM image output controls: https://pymupdf.readthedocs.io/en/latest/pymupdf4llm/api.html
- Docling figure export: https://docling-project.github.io/docling/examples/export_figures/
- NLM figure extraction paper: https://lhncbc.nlm.nih.gov/LHC-publications/PDF/pub7055.pdf
- Local research report: `.warden/research/figure-snapshot-extraction/REPORT.md`
