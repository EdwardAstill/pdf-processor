---
title: "Improvement Opportunities for cnv"
kind: "roadmap"
category: "wiki"
summary: "Summarizes the main functional gaps still visible in cnv's PDF-to-Markdown output and turns the cross-project research into a prioritized list of concrete improvements."
virtual_path: "wiki/roadmap/improvement-opportunities"
entities: [cnv, Tagged PDF, OCR, XY-Cut++, financial tables, debug artifacts]
---

# Improvement Opportunities for cnv

`cnv` is already solid on many text-heavy PDFs, invoices, and simpler forms, but it is not functionally optimal yet. The current example corpus and the external project survey both point to specific missing capabilities that would materially improve Markdown quality.

## Why `cnv` Is Not Functionally Optimal Yet

The current visible gaps are:

- hard financial statements still lose row structure
- scan-heavy PDFs still need a local OCR path
- tagged PDFs are not fully exploited
- magazine and brochure grouping remains weak
- table detection is still too dependent on text recovery instead of geometry
- debugging difficult layouts is slower than it should be

## Priority Improvements That Would Most Improve PDF-to-Markdown Quality

| Priority | Improvement | Why it matters | External evidence | Best implementation direction for `cnv` |
| --- | --- | --- | --- | --- |
| P0 | Tagged PDF / structure-tree reader | Tagged PDFs already contain authoritative headings, lists, and tables; guessing leaves quality on the table | `opendataloader-pdf` uses structure tags directly | add an alternate metadata/structure reader and prefer tags when present |
| P1 | Local OCR preprocessing path | scan-heavy examples are still the worst local outputs | `OpenDataLoader PDF`, `PyMuPDF4LLM`, `OCRmyPDF`, and `docling` all treat OCR as a real first-class path | generate a searchable derivative PDF locally, then rerun the normal extractor |
| P2 | Numeric-heavy table engine | financial statements remain the biggest active quality gap | `pdf-inspector`, `OpenDataLoader PDF`, `marker`, and `MinerU` all separate hard table logic from generic prose rendering | detect numeric-heavy regions, infer column guides, then build rows/cells before Markdown emission |
| P3 | Rectangle and line based table detection | visually-defined tables cannot be recovered well from text alone | `pdf-inspector` and `pdfplumber` both rely on geometry; `OpenDataLoader PDF` uses border analysis | add access to drawing ops or equivalent line/rect data and use it in table detection |
| P4 | Cross-page table continuation | financial and report tables often span pages | `MinerU` and `marker` explicitly merge or repair continuation tables | add table-fragment continuity logic across adjacent pages |
| P5 | Font metadata and heading evidence beyond size | body-size bold headings and subtle section breaks are still missed | `pdf-inspector` uses font patterns; tagged-PDF tools use roles; `PyMuPDF4LLM` supports TOC-driven headers | keep size tiers, but add font family/weight/style and TOC/tag evidence |
| P6 | Broken-encoding and OCR fallback detection | some PDFs look digital but still have unusable text | `pdf-inspector` explicitly flags broken encodings; `PyMuPDF4LLM` OCRs garbled regions selectively | add encoding health checks and route only the broken pages or spans |
| P7 | Hidden text and off-page filtering | hidden layers reduce Markdown quality and create downstream safety risk | `OpenDataLoader PDF` ships prompt-injection filtering for hidden/off-page content | suppress invisible, zero-size, and off-page text during extraction |
| P8 | Debug artifact mode | current heuristic tuning is slower because Markdown hides the actual structural failure | `marker`, `pdfplumber`, `pdf-inspector`, and `table-transformer` all benefit from debug visuals | emit page overlays for blocks, columns, table guides, suppressed furniture, and final reading order |
| P9 | Better document subtype detection | papers, invoices, forms, financials, slides, brochures, and scans should not all share one parser path | every strong parser classifies page or document type before applying recovery logic | add document and page subtype signals early in the pipeline |
| P10 | Table-specific evaluation fixtures | snapshot diffs are good, but weak for measuring partial table progress | `table-transformer`, `marker`, and OpenDataLoader benchmarking separate table quality from prose quality | add structured row/column/cell fixtures for the hardest financial and invoice regions |

## Improvements That Are Valuable but Not the Immediate Bottleneck

These are useful, but they are not the first things to build:

- full broad multi-format support
- hosted API integrations
- VLM-first conversion
- LLM cleanup as a default path

These may help some users, but they do not address `cnv`'s current core quality bottlenecks as directly as table structure, OCR fallback, and tagged-PDF support.

## Improvements That Would Directly Help the Current Example Corpus

The standing example set suggests this practical mapping:

- `golden__issue-336-conto-economico-bialetti.pdf`
  needs numeric-heavy table parsing, geometry-aware table detection, and cross-page continuity
- `golden__chinese_scan.pdf`
  needs local OCR preprocessing and better scan routing
- `attention.pdf`, `bert.pdf`, `clip.pdf`, `gpt3.pdf`, `resnet.pdf`
  need richer heading evidence and better front-page author/affiliation handling
- `PDFUA-Ref-2-01_Magazine-danish.pdf` and `PDFUA-Ref-2-06_Brochure.pdf`
  need better page subtype detection, article clustering, and furniture suppression
- `PDFUA-Ref-2-02_Invoice.pdf`
  needs better business header and address normalization

## Current Status (2026-05)

Several items have moved since this roadmap was first written:

- **Debug artifact mode** (P8): implemented. `--debug-formulas` and
  `--debug-tables` emit crops and JSON manifests under `debug/`.
- **Visual formula detection**: implemented (2026-05-11). Catches visible
  equations the word-based detector misses (DNV page 670 style). Opt-in via
  `--debug-formulas`.
- **Formula false positive suppression**: partial. Table suppression in word
  path works; visual path still gets decorative-rule and logo-bar FPs.

Remaining active gaps for technical standards work:

1. Table detection for standards (borderless, symbol-heavy rows) — currently
   ~1% recall on DNV. See [Technical Standards Documents](technical-standards-documents.md).
2. Formula OCR sidecar — no LaTeX reconstruction yet, only review markers.
   See [Formula Detection and OCR](formula-detection-and-ocr.md).
3. Watermark/footer suppression ("Downloaded by…" noise in standards).
4. Formula false-positive suppression for decorative rules and logo bars.

## The Best Next Build Order

The original P0–P10 sequence remains valid. Updated for current state:

1. ~~debug artifact mode~~ ✓ done
2. ~~visual formula detection~~ ✓ done
3. **Formula false-positive suppression** — decorative-rule filter, furniture
   mask pre-pass, table-first suppression
4. **Geometry-aware table detection for standards** — explicit-line extraction,
   borderless table inference, furniture-bbox exclusion
5. Formula OCR sidecar — RapidLaTeX-OCR or UniMERNet via ONNX/Python
6. Tagged PDF / structure-tree reader (P0 from original)
7. Local OCR preprocessing path (P1)
8. Numeric-heavy / financial table engine (P2)
9. Cross-page table continuation (P4)
10. Encoding-health checks (P6)

## Sources for the Improvement Opportunities

- [Project comparison matrix](project-comparison-matrix.md)
- [OpenDataLoader ecosystem](opendataloader-ecosystem.md)
- [Reference implementations](reference-implementations.md)
- [Formula detection and OCR](formula-detection-and-ocr.md)
- [Technical standards documents](technical-standards-documents.md)
