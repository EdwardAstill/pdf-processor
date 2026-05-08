# Contradictions And Evidence Gaps

Checked on 2026-05-08.

## Coordinate Algorithms Versus Visual/ML Algorithms

Coordinate algorithms such as pdfplumber, Camelot Stream/Network, and the current `src/layout/table.rs` path are the right default for born-digital PDFs because they use the existing text layer and can stay fast, local, deterministic, and MIT-compatible.

Visual and ML algorithms such as Camelot Lattice, Table Transformer, gmft, Surya, img2table, and PDF-Extract-Kit are stronger when the table structure is visual rather than text-coordinate-obvious: ruled tables, scanned tables, merged headers, and image-only tables. They add heavier dependencies, model downloads, and sometimes GPU/runtime friction.

Resolution: keep coordinate extraction as the default local path, and make visual/ML tools opt-in sidecars selected by quality gates.

## Full-Document Converters Versus `pdfp`'s Local Pipeline

Docling, MinerU, Marker, Unstructured, and similar tools can convert whole PDFs into Markdown/JSON. That can outperform local heuristics on complex documents, but replacing the local pipeline would lose the current inspect/search/page operations, deterministic debug files, and small Rust CLI behavior.

Resolution: use full-document converters as hybrid backends and benchmarks. Do not replace the local path unless repeated corpus runs show broad improvement and acceptable dependencies.

## Formula Detection Versus Formula Recognition

`src/formula/detect.rs` can find candidate regions, but it cannot recover semantic LaTeX from glyph geometry alone. Formula recognition needs a crop recognizer or a document backend with formula enrichment.

Resolution: keep local detection as an audit/routing layer. Send crops or pages to Docling/UniMERNet/PDF-Extract-Kit-style backends only when the user opts into recovery.

## Embedded Image Extraction Versus Figure Snapshot Rendering

Embedded image extraction is fast and precise for raw raster assets, but it misses vector charts, labels, axes, and multi-object figures. Snapshot rendering captures the painted page region but needs better candidate boxes.

Resolution: keep `embedded` for compatibility, keep `snapshot` for complete visual figures, and improve detection using PDFFigures2-style caption proposals and body-text exclusion.

## OCRmyPDF Versus Direct OCR Engines

OCRmyPDF is better as a first pass for scanned PDFs because it creates a searchable PDF that the rest of the extractor can consume. Direct OCR engines can help with specific image-table or LaTeX-OCR cases but fragment the pipeline.

Resolution: keep OCRmyPDF as default scan preprocessing. Add direct OCR/table/image engines only behind explicit sidecar experiments.

## License Fit Versus Technical Fit

Some technically strong projects are GPL or AGPL: Marker, Surya, and PDF-Extract-Kit. This repo is MIT licensed, so importing their code would be inappropriate without changing project licensing or receiving explicit approval.

Resolution: MIT/Apache sources can influence native code and direct dependencies. GPL/AGPL tools should be benchmark-only or user-installed subprocesses with clear license warnings.

## Benchmarks Versus Real Examples

PubTables-1M, FinTabNet, GriTS, and similar datasets provide structure-aware table evaluation, but the user's examples include standards, catalogues, scans, and mixed engineering documents. Public benchmarks do not fully represent those documents.

Resolution: use public metrics for table structure sanity and local examples for acceptance. The research loop should record both.
