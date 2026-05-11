---
title: "Headings"
kind: "reference"
category: "wiki"
summary: "How headings are encoded in PDFs, how pdfp detects and classifies them, and how they map to Markdown heading levels."
virtual_path: "wiki/structures/headings"
entities: [font-size-tiering, struct-tree, TOC, bold-detection, pdfium-metadata]
---

# Headings

Headings are the most important structural element for downstream usability. Getting the hierarchy wrong makes documents hard to navigate and breaks LLM context.

## How PDFs Encode Headings

PDFs have no native heading concept at the text-stream level. A heading is visually distinct text — usually larger, bolder, or styled differently — but the file format encodes it the same way as any other text span.

Three sources of heading evidence, in confidence order:

1. **Struct-tree roles** (highest): Tagged PDFs contain a `/StructTreeRoot` with explicit H1–H6 and Title role assignments. Authoritative — never override these.
2. **TOC/outline** (high): The PDF's `/Outlines` (bookmark tree) lists known heading texts and page numbers. Match extracted text to outline entries.
3. **Visual signals** (default): Font size, font weight, line length, and punctuation density.

## Visual Signal Heuristics

### Font-size tiering

`pdfp` uses ratio of block font size to page body-mode font size:

| Ratio | Level |
|-------|-------|
| ≥ 2.0 | H1 |
| ≥ 1.6 | H2 |
| ≥ 1.35 | H3 |
| ≥ 1.15 | H4 |
| < 1.15 | H5 / body |

Implemented in `src/layout/classifier.rs`.

### Bold at body size

When font weight ≥ 700 and the line is short (not a full paragraph) and does not end with sentence-terminating punctuation → classify as H4 even at body size.

Available only with `--features pdfium-metadata` (requires libpdfium). Falls back silently to size-only when the library is absent.

### Supporting signals

- **Short line** (< ~60 chars): Headings rarely wrap.
- **Low punctuation density**: Headings rarely contain commas, semicolons, or full stops mid-line.
- **Preceding vertical gap**: Headings typically have more whitespace above them than below.
- **All-caps**: Common in standards and formal documents for section headings.

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| Numbered section labels (1.2.3) become H1 | Large bold numbering at section start | TOC-match or struct-tree override |
| Body-size bold headings missed | No weight signal without pdfium | Enable `pdfium-metadata` feature |
| Running headers classified as document headings | Repeated header text treated as content | Furniture suppression before classification |
| Figure captions (bold, short) promoted to headings | Bold + short = heading heuristic fires | Check context: preceded by figure block |
| All caps body text promoted | All-caps signal over-triggers | Weight all-caps signal lower; require size OR bold as well |

## Markdown Rendering

`pdfp` emits `#`–`#####` Markdown headings. The renderer (`src/render/markdown.rs`) maps `BlockKind::Heading { level }` directly:

```
H1 → #
H2 → ##
H3 → ###
H4 → ####
H5 → #####
```

No heading is emitted as bold text — heading structure is always preferred over inline emphasis.

## Standards-Specific Notes

Engineering standards (DNV, ISO, IEC) typically use:
- Large bold numbering for main clause headings (matches H1/H2 by size)
- Same-size bold for sub-clause headings (needs weight signal → pdfium-metadata)
- All-caps for special sections (FOREWORD, SCOPE, NORMATIVE REFERENCES)
- A running header with the standard number and clause title — this must be suppressed as furniture before heading detection runs

## See Also

- [algorithms/heading-classification.md](../algorithms/heading-classification.md) — detailed algorithm descriptions
- [topics/layout-and-reading-order.md](../topics/layout-and-reading-order.md) — heading attachment to body blocks
