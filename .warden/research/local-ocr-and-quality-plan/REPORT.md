# Local OCR and PDF Quality Viability Report

Date checked: 2026-05-05

## Conclusion

The viable next move is an OCRmyPDF-backed local OCR sidecar, not a second converter and not direct Tesseract integration. The sidecar should create a searchable derivative PDF, cache it, then rerun the existing MuPDF/XY-Cut/classifier/render pipeline. This keeps the system simple and preserves the deterministic local path for born-digital PDFs.

The plan should also make the baseline corpus first-class before changing behavior: current `cnv` converts all in-repo example PDFs successfully, but scan-only and numeric-table cases expose the quality gaps.

## Why OCRmyPDF First

OCRmyPDF already solves the parts `cnv` should not own yet: PDF validation, preserving image content, adding invisible text layers, rotation, deskew, language packs, multiprocessing, and Tesseract orchestration. Its docs and README explicitly describe this searchable-PDF workflow.

Tesseract direct output is real and useful for images, including searchable PDF and hOCR output, but using it directly would force `cnv` to own PDF rasterization, page image extraction, page assembly, and text-layer placement. That is more complexity than the repo needs for a first local OCR path.

## Routing Shape

The evidence supports these defaults:

- `--ocr off` remains the default unless explicitly changed later.
- `--ocr auto` runs OCR only when scan triage says the local text path is poor.
- `--ocr force` exists for bad embedded text layers, but is documented as slower and riskier.
- OCR cache is on when a cache dir is provided; cache keys include file metadata and OCR options.
- Hybrid Docling remains separate: it is for table/formula/layout recovery, not only OCR.

## Baseline Findings

The baseline command processed `example/pdf` recursively:

- 44 total PDFs
- 44 passed
- 0 failed

The top-level examples:

- 22 total PDFs
- 22 passed
- 0 failed

Important observed gaps:

- `golden__chinese_scan.pdf` is image-only output with a scan-heavy warning.
- `golden__issue-336-conto-economico-bialetti.pdf` has useful table regions but still has glued numeric rows.
- `Magazine-danish.pdf` extracts 387 images, showing decorative-media pressure.
- `resnet.pdf` has 219 heading markers, a likely signal of heading over-classification.

## Complexity Assessment

Mostly healthy:

- `main.rs` is now thin after the pipeline refactor.
- `pipeline.rs` is the right orchestration home.
- `hybrid/client.rs` is defensive because Docling response schemas move; that is acceptable.

Needs simplification:

- `render/markdown.rs` contains too much structural recovery. Future table, form, furniture, and front-matter logic should move upstream into explicit structure passes.
- `hybrid/page_extract.rs` temp-file extraction is acceptable but should use collision-resistant temp paths and be wrapped behind a page-extraction abstraction.
- Table repair should not be implemented as more renderer heuristics.

## Implementation Recommendation

1. Improve the quality harness so the baseline is reproducible and top-level/non-duplicate summaries are first-class.
2. Add `src/ocr/` around an `OcrProvider` trait and `OcrMyPdfProvider`.
3. Add CLI flags for OCR mode, language, cache dir, timeout, and provider command.
4. Add dependency detection and clear missing-tool errors.
5. Integrate OCR as preprocessing before `PdfExtractor::extract`.
6. Use baseline fixtures for acceptance, especially `golden__chinese_scan.pdf`.
7. Then move table/form/furniture recovery out of the renderer in bounded steps.

