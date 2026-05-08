# Formula Extraction Tooling Report

Checked: 2026-05-05

## Executive Finding

The proven direction is not generic OCR. Formula recovery needs a two-stage pipeline: formula-region detection followed by formula-image recognition into LaTeX. For `pdfp`, the best engineering path is:

1. Add formula blocks and formula coverage/audit to the Rust pipeline.
2. Use Docling formula enrichment as the first optional hybrid backend.
3. Add a lower-level local formula sidecar later, probably UniMERNet/PDF-Extract-Kit or PaddleOCR formula modules.
4. Keep Mathpix/Azure as explicit cloud backends for users who accept credentials, upload, and cost.

## Tool Shortlist

| Tool | What it gives | Fit for `pdfp` | Concern |
| --- | --- | --- | --- |
| Docling | Whole-document conversion with `do_formula_enrichment` / `docling --enrich-formula` | Best first integration because `pdfp` already has a Docling hybrid path | Python/model runtime, slower than local Rust path |
| MinerU | High-quality Markdown/JSON extraction with configurable table/formula recognition | Strong batch fallback for hard standards | Heavier, API or separate deployment shape |
| PDF-Extract-Kit + UniMERNet | Modular formula detection and formula-image-to-LaTeX recognition | Best future sidecar for patching only formula regions | Need model management and crop/coordinate integration |
| PaddleOCR PP-StructureV3 / PaddleOCR-VL | Local/service document parsing with formulas, tables, Markdown/JSON | Strong alternative backend, especially if Paddle runtime is acceptable | Heavier runtime; newer APIs need local testing |
| Marker/Surya/Texify | Markdown converter + LaTeX OCR path | Useful comparison backend | GPL/model restrictions make bundling awkward |
| Mathpix | High-quality hosted PDF math OCR with MD/MMD/LaTeX zip/line JSON outputs | Best optional cloud escape hatch | Paid API, external upload, credentials |
| Azure Document Intelligence | Enterprise cloud document layout with formula support | Enterprise option | Cloud, Azure coupling, formula behavior needs corpus testing |

## Recommendation

For DNV-ST-N001 formula gaps, implement a formula-aware quality gate before choosing a backend:

- Detect candidate formulas and equation numbers in `pdfp`.
- Emit a formula coverage ledger in debug output.
- If local extraction has missing/low-confidence formula text, route those pages to Docling formula enrichment.
- If Docling does not recover enough formulas, test UniMERNet/PDF-Extract-Kit and PaddleOCR-VL on the exact DNV pages.
- Do not mark standard-processing pages `active` unless the formula ledger is closed.

This preserves `pdfp`'s local-first speed while giving standards processing a reliable escalation path.

## Sources

- Docling enrichment docs: https://docling-project.github.io/docling/usage/enrichments/
- MinerU API docs: https://mineru.net/doc/docs/index_en/
- PDF-Extract-Kit: https://github.com/opendatalab/PDF-Extract-Kit
- PDF-Extract-Kit formula recognition docs: https://pdf-extract-kit.readthedocs.io/en/latest/algorithm/formula_recognition.html
- UniMERNet: https://github.com/opendatalab/unimernet
- Surya OCR: https://pypi.org/project/surya-ocr/
- Marker PDF: https://pypi.org/project/marker-pdf/1.7.3/
- Mathpix PDF API: https://docs.mathpix.com/reference/post-v3-pdf
- Azure Document Intelligence overview: https://learn.microsoft.com/en-us/azure/ai-services/document-intelligence/overview?view=doc-intel-3.0.0
- PaddleOCR PP-Structure quick start: https://www.paddleocr.ai/main/en/version2.x/ppstructure/quick_start.html
- PaddleOCR-VL usage docs: https://www.paddleocr.ai/latest/en/version3.x/pipeline_usage/PaddleOCR-VL.html
- PP-StructureV3 docs: https://www.paddleocr.ai/latest/en/version3.x/pipeline_usage/PP-StructureV3.html
