# Figure Snapshot Extraction Research Plan

date: 2026-05-05
status: completed

## Question

How should `pdfp` address the current image-extraction weakness where a visually single figure may be emitted as separate embedded images, or may lose vector graphics, labels, legends, and panel letters?

## Subquestions

1. Is raw embedded-image extraction enough for complete PDF figures?
2. Which renderer APIs support page-region snapshots?
3. How do comparable PDF-to-Markdown/document tools expose figures and images?
4. What should the first `pdfp` implementation do without turning conversion into a heavy ML layout system?
5. How should performance and quality be tested against the existing PDFs?

## Scope

In scope:

- Figure image output for Markdown conversion.
- Detection of likely figure regions from existing PDF layout data.
- Region rendering from the PDF page as a snapshot.
- CLI, docs, and regression/performance tests.

Out of scope for the first implementation:

- Full semantic figure understanding.
- ML-based figure/caption detection.
- Pixel-perfect extraction for every scientific publishing style.
- Replacing raw embedded-image extraction entirely.

## Local Context Checked

- `src/pdf/extractor.rs` uses MuPDF text-page image blocks and `image.to_pixmap()`.
- `src/pipeline.rs` writes those image objects as `images/pageN_imgM.png`.
- `docs/pdf-internals.md` already documents that vectors are skipped and image objects do not equal complete visual figures.
- Prior baseline output under `.warden/research/local-ocr-and-quality-plan/baseline/current-quality/` shows `attention.pdf` emits image references near figure captions, but it also has caption-only figure references later where no image was extracted.
