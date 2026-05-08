# Source Notes

date: 2026-05-05

## PyMuPDF / MuPDF

PyMuPDF's official page API documents `Page.get_pixmap()` with `dpi`, `colorspace`, `clip`, `alpha`, and annotation controls. The useful finding is not Python-specific: MuPDF can render a rectangular page region to a pixmap, which is exactly what a complete figure snapshot needs.

The PyMuPDF device API also documents clip-aware pixmap devices. Locally, the Rust `mupdf` crate has lower-level page, display-list, device, pixmap, and clip APIs. The plan should validate the pure-Rust route first so `pdfp` does not grow a renderer sidecar unnecessarily.

## Docling

Docling has an official figure export example that writes page images, table images, picture images, and Markdown with embedded or referenced images. This supports the design choice that "figure assets" should be first-class conversion outputs rather than incidental image-object dumps.

Docling's document model also exposes Markdown image modes and provenance/bounding boxes. That supports representing figure candidates as structured data with page number, bbox, caption relation, and confidence.

## PyMuPDF4LLM

PyMuPDF4LLM exposes image writing, image embedding, DPI, image-size limits, page chunks, image metadata, and graphics metadata. It also warns that higher OCR DPI increases processing time and memory roughly quadratically. For `pdfp`, figure snapshots should therefore be selectable by mode and bounded by DPI/size thresholds.

## Biomedical Figure Extraction Paper

The NLM paper is directly relevant to the user's issue. It documents that common PDF image extractors can miss complete figures, split multi-panel figures, drop labels/legends/text, and mishandle overlaid graphics. That is the precise failure mode observed in `pdfp`.
