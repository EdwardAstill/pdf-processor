# Code Reference

Each document analyses one module's implementation: source files, key types, key functions, CLI flags, and cross-references.

| Module | Source directory | Reference |
|---|---|---|
| **PDF format** | — | [pdf-format.md](pdf-format.md) — PDF file format primer (content streams, fonts, coordinate systems) |
| **PDF extraction** | `src/pdf/` | [pdf-extraction.md](pdf-extraction.md) — MuPDF bindings, text extraction, font metadata, text cleanup |
| **Layout analysis** | `src/layout/` | [layout-analysis.md](layout-analysis.md) — XY-Cut++ reading order, block classification, all table detectors, furniture suppression |
| **Formula detection** | `src/formula/` | [formula-detection.md](formula-detection.md) — word-geometry candidates, geometric LaTeX, visual band scan, OCR sidecar |
| **Pipeline** | `src/pipeline/` | [pipeline.md](pipeline.md) — page building, block merging, candidate routing, flow diagram |
| **Output** | `src/render/`, `src/figure/` | [markdown-rendering.md](markdown-rendering.md) — MarkdownRenderer, table/formula rendering, clean style, scholarly front matter, figure detection and snapshots |
| **External integration** | `src/ocr/`, `src/hybrid/` | [external-integrations.md](external-integrations.md) — OCRmyPDF preprocessing, Docling hybrid enrichment |
| **CLI and processors** | `src/cli.rs`, `src/processor/` | [cli-and-processors.md](cli-and-processors.md) — CLI argument types, command dispatch, all page operations |
| **Eval framework** | `src/eval/` | [eval-framework.md](eval-framework.md) — fixture loading, precision/recall metrics, eval runner |

## Related documentation

- [architecture.md](../architecture.md) — high-level module map and design decisions
- [CLI.md](../CLI.md) — full CLI reference with examples
- [TESTING.md](../TESTING.md) — test matrix, eval fixtures, quality improvement loop
- [TOOL_COMPARISON.md](../TOOL_COMPARISON.md) — comparison against other tools
- [wiki/](../../wiki/README.md) — deep knowledge base per pipeline stage
