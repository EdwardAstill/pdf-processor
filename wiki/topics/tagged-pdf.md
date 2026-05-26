---
title: "Tagged PDF and Structure Tree"
kind: "knowledge"
category: "wiki"
summary: "How Tagged PDF works: structure trees, marked content, standard structure types, PDF 2.0 extensions, and how pdfp can use struct-tree data for heading recovery, reading order, and accessibility."
entities: [StructTreeRoot, pdfium, MuPDF, marked-content, Tagged-PDF, PDF-UA]
---

# Tagged PDF and Structure Tree

Tagged PDF is the mechanism by which PDFs carry semantic structure alongside visual content. A tagged PDF contains a **structure tree** — a hierarchy of elements (headings, paragraphs, lists, tables, figures) that describe what the content *is*, not just where it sits on the page.

For `pdfp`, the structure tree is the most reliable path to correct heading detection and reading order. When present, it beats every heuristic.

---

## How the Structure Tree Works

A tagged PDF's document catalog contains a `/StructTreeRoot` dictionary. This is the root of a tree of **structure elements**, each with:

- **`/S` (Type)**: The standard structure type — `Document`, `Part`, `Sect`, `H`, `H1`–`H6`, `P`, `L`, `LI`, `Lbl`, `LBody`, `Table`, `TR`, `TH`, `TD`, `Figure`, `Formula`, `Caption`, `TOC`, `TOCI`, `Art`, `Aside`, `BlockQuote`, `Note`, `Reference`, `BibEntry`, `Code`
- **`/K` (Children)**: Child elements or marked-content references
- **`/A` (Attributes)**: Layout, list-numbering, table-scope, and other attributes
- **`/Pg` (Page)**: Owning page for block-level elements

### Marked Content

The connection between the structure tree and page content happens through **marked content** — `BMC`/`BDC`/`EMC` operators in the content stream that tag spans of drawing operators. A structure element's `/K` can reference these marked-content IDs directly.

### Example

```
/StructTreeRoot
  /K [
    << /S /Document
       /K [
         << /S /H1 /K [ << /Type /MCR /Pg 3 0 R /MCID 0 >> ] >>
         << /S /P  /K [ << /Type /MCR /Pg 3 0 R /MCID 1 >> ] >>
         << /S /Table /K [ ... ] >>
       ]
    >>
  ]
```

---

## PDF 2.0 Extensions (ISO 32000-2)

PDF 2.0 adds to the tagging model:

- **MathML**: Formula structure elements can contain MathML for accessible math
- **Namespaces**: Structure elements can declare their namespace (`/NS`), enabling mixed-namespace documents
- **Associated Files (AF)**: Structure elements can reference associated files (images, data) via `/AF`
- **New structure types**: `Aside`, `BlockQuote`, `Caption`, `Formula`, `Note`, `Reference`, `Title`

The **Well-Tagged PDF (WTPDF)** specification from the PDF Association defines best practices for PDF 2.0 tagging. It categorizes tagging requirements as "shall", "should", or "may" for each element type.

---

## API Access

### pdfium

Exposes a dedicated struct-tree API via `fpdf_structtree.h`:

```
FPDF_StructTree_GetForPage(page)  → tree handle
FPDF_StructElement_GetType(elem)   → standard type string
FPDF_StructElement_GetTitle(elem)  → alternate/expanded title
FPDF_StructElement_GetMarkedContentID(elem) → MCID
FPDF_StructElement_CountChildren(elem)
FPDF_StructElement_GetChildAtIndex(elem, i)
```

`pdfp` can access this through the `pdfium-render` Rust crate (`--features pdfium-metadata`). This is the same feature gate that provides font-weight and italic flags.

### MuPDF

MuPDF's `fz_stext_page` does layout analysis but does NOT expose the structure tree directly. The Rust `mupdf` crate (0.6) has no struct-tree API. This is a known limitation — MuPDF can *use* the structure tree internally for text extraction ordering, but doesn't expose it.

### pdfplumber (Python)

`page.structure_tree` returns a tree of `StructureElement` objects with `.type`, `.tag`, `.properties`, and `.children`. This is the most accessible exploration API. The `pdfplumber` structure docs are an excellent reference.

---

## Relevance to pdfp

### Heading detection
Today `pdfp` uses font-size tiering and bold detection for headings. When a structure tree exists, `H1`–`H6` elements are authoritative — they tell you exactly what the document author intended. Font-size heuristics should only be a fallback.

### Reading order
The structure tree's depth-first traversal defines the logical reading order. This is more reliable than XY-Cut++ for complex layouts (multi-column with figures, sidebars).

### Outlines/Bookmarks (Stage 2)
PDF outline trees (bookmarks) are a separate hierarchy pointing to page destinations. Combined with the structure tree, outlines can be converted to a Markdown Table of Contents with correct heading cross-references.

### Link → heading conversion (Stage 2)
Internal PDF links (`/GoTo` actions) point to specific pages/positions. With the structure tree, `pdfp` can map these destinations to the nearest heading and emit Markdown heading references (`[text](#heading-slug)`) instead of page numbers.

### PDF/UA generation (Stage 4)
Generating accessible PDFs requires emitting a full structure tree with correct types, alt-text, and language tags. This is a separate capability from extraction — it means `pdfp` could *produce* tagged PDFs.

---

## PDF/UA Standards

Two ISO standards define accessible PDF requirements:

| Standard | Based on | Key additions |
|---|---|---|
| PDF/UA-1 (ISO 14289-1, 2014) | PDF 1.7 | Tagged PDF required; alt-text on images; logical reading order; heading hierarchy |
| PDF/UA-2 (ISO 14289-2, 2024) | PDF 2.0 | MathML support; namespaces; updated structure types; form-field accessibility |

ISO 14289 parts 1 and 2 are now freely available — no cost to access as of 2024.

### Validation tools

- **veraPDF** (open source, Java) — industry-standard PDF/A and PDF/UA validator
- **PAC (PDF Accessibility Checker)** — free Windows desktop tool
- **Adobe Acrobat Pro** — built-in accessibility checker

---

## Current State in pdfp

| Capability | Status |
|---|---|
| Structure tree reading | Available via `pdfium-render` feature gate |
| Heading detection from struct-tree | Not implemented |
| Reading order from struct-tree | Not implemented |
| Outline/bookmark extraction | Not implemented (Stage 2) |
| Link → heading conversion | Not implemented (Stage 2) |
| PDF/UA generation | Not implemented (Stage 4) |

## Related Pages

- [Heading Classification](../algorithms/heading-classification.md) — current font-size-based approach
- [Reading Order and Layout](../algorithms/reading-order.md) — XY-Cut++ approach
- [Pipeline Overview](../topics/pipeline-overview.md) — where struct-tree fits in the pipeline
- [PDF Engines](../tools/pdf-engines.md) — MuPDF vs pdfium capabilities
