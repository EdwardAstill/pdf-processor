# Context Placement

What content belongs in each area of the `pdf-processor` wiki.

## Area map

| Area | Purpose | What goes here |
|---|---|---|
| `algorithms/` | How it works | Detailed algorithm descriptions for each pipeline stage: table detection, reading order, OCR, formula handling, heading classification, page triage |
| `tools/` | What to use | Catalogues of libraries, frameworks, models, crates: PDF engines, extraction libs, layout models, equation OCR models, table models, Rust crates |
| `structures/` | Content types | How specific PDF elements (headings, tables, equations, lists, inline formatting) are encoded and how `pdfp` recovers them |
| `topics/` | Pipeline concepts | Cross-cutting concerns: pipeline design, text extraction, layout, scans, rendering, evaluation, information extraction, standards documents |
| `projects/` | External landscape | What other projects do, comparison matrices, reference implementations, improvement opportunities |
| `references/` | Wiki rules | This directory — conventions, placement, and wiki governance |

## Placement decisions

- If a page describes **how an algorithm works** in detail, it belongs in `algorithms/`
- If a page **catalogues tools/libraries/models**, it belongs in `tools/`
- If a page describes **how pdfp recovers a PDF structural element**, it belongs in `structures/`
- If a page covers **pipeline architecture, cross-cutting concerns, or evaluation**, it belongs in `topics/`
- If a page **compares or evaluates other projects**, it belongs in `projects/`

## Depth

Max depth: `<area>/<topic>.md`. No subdirectories below each area. `depth_adherence: loose` — the current wiki stays at this depth but could add `tools/models/<model>.md` in the future.
