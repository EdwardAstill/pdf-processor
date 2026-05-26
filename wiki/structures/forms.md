---
title: "PDF Forms (AcroForm and XFA)"
kind: "knowledge"
category: "wiki"
summary: "How interactive forms work in PDF: AcroForm field types, widget annotations, appearance streams, XFA vs AcroForm, deprecation status, and Rust/Python libraries for reading and filling form fields."
entities: [AcroForm, XFA, pdfer-forms, acroform, lopdf, pypdf, pdfium]
---

# PDF Forms (AcroForm and XFA)

PDF supports two form technologies: **AcroForm** (the original, still supported) and **XFA** (XML-based, deprecated in PDF 2.0). Understanding both matters because real-world PDFs contain both, and only AcroForm has a future.

---

## AcroForm

AcroForm has been part of PDF since version 1.2. It defines forms as a collection of **field dictionaries** organized in a hierarchy, with **widget annotations** providing the visual representation on pages.

### Field Types

| Field type | `/FT` value | Description |
|---|---|---|
| Button | `Btn` | Pushbutton, checkbox, radio button |
| Text | `Tx` | Single-line or multi-line text input |
| Choice | `Ch` | Drop-down list or combo box |
| Signature | `Sig` | Digital signature placeholder |

### Field Hierarchy

The document catalog's `/AcroForm` entry points to the interactive form dictionary. Fields are organized in a tree:

```
/AcroForm
  /Fields [
    << /T (name) /FT /Tx /V (John) /Kids [widget_annot_ref] >>
    << /T (address) /FT /Tx /V (123 Main St) /Kids [widget_annot_ref] >>
  ]
```

A **terminal field** has widget annotation children — these are the fields that appear on pages. Non-terminal fields are organizational parents that can share properties with their children.

### Widget Annotations

Each visible field is backed by a widget annotation (subtype `/Widget`). Key entries:

| Entry | Description |
|---|---|
| `/Rect` | Bounding box on the page |
| `/F` | Flags (print, hidden, readonly, required) |
| `/AP` | Appearance streams — `/N` (normal), `/R` (rollover), `/D` (down) |
| `/MK` | Appearance characteristics (border, background colors, font) |
| `/DA` | Default appearance string (font, size, color for text fields) |
| `/Ff` | Field flags (multiline, password, comb, rich-text for text fields) |

### Reading and Filling

Reading field values: traverse `/AcroForm/Fields`, read `/V` (value) and `/DV` (default value) from terminal field dictionaries.

Filling: update `/V` in the field dictionary, regenerate or update the appearance stream in `/AP/N`, save incrementally or rewrite.

### Flattening

Form flattening: remove the AcroForm structure, convert widget annotations into static page content. Done by rendering appearance streams as permanent page content. `pdfer-forms` supports flattening directly.

---

## XFA (XML Forms Architecture)

### What it is
XFA was introduced in PDF 1.5 as an XML-based alternative to AcroForm. It uses an XML "template" defining form layout plus XML "data" filling the form. XFA forms handle dynamic layouts where the number of fields varies with data size.

### Three types
| Type | AcroForm fallback? | Readable without Adobe? |
|---|---|---|
| Static XFA | Yes — contains both XFA stream and AcroForm | Yes, via AcroForm fallback |
| Dynamic XFA | No — XFA-only | No — only Adobe Reader/Acrobat renders correctly |

### Deprecation
XFA was **deprecated in PDF 2.0** (ISO 32000-2). PDF 2.0 writers must not create XFA forms. PDF 2.0 readers may still process XFA for backward compatibility, but the standard expects migration to AcroForm.

### Impact on pdfp
`pdfp` should detect XFA forms and, for static XFA, extract the AcroForm fallback. For dynamic XFA, extraction is essentially impossible without rendering the form — these documents should be flagged as unextractable forms and routed to a rasterization/OCR fallback.

---

## Libraries

### Rust

| Library | Capabilities | Based on |
|---|---|---|
| **pdfer-forms** | Fill, inspect, flatten AcroForm fields. 23× faster than pypdf. | lopdf |
| **acroform** | Read and fill AcroForm fields. Typed values (text, bool, choice, int). | lopdf |
| **lopdf** | Low-level PDF object manipulation. Can read/write field dictionaries and widget annotations directly. | Pure Rust |
| **pdfium-render** | Can extract form field data via pdfium's form API (`FPDFDOC_InitFormFillEnvironment`). | pdfium |

### Python
- **pypdf**: `reader.get_fields()`, `writer.update_page_form_field_values()`, field flattening
- **pdfplumber**: Extracts form field rectangles and values from annotations

---

## Relevance to pdfp

### Stage 3: Form Reading
Extract form field names, types, and values. Emit as structured Markdown (field list or key-value section). Detect form field locations and avoid treating them as document text.

### Stage 3: Form Filling
`pdfp form fill --field name=value` — populate AcroForm fields with external data. `pdfer-forms` provides a Rust-native path.

### Stage 3: Form Flattening
`pdfp form flatten` — convert fillable forms to static PDFs. Required for archival and non-interactive workflows.

### Stage 2 (quick win)
Form field detection during extraction — recognize widget annotation rects and exclude them from text extraction, or emit them as `<!-- form-field: name="..." -->` markers in Markdown.

---

## Current State in pdfp

| Capability | Status |
|---|---|
| Form field detection | Not implemented |
| Form field value extraction | Not implemented |
| Form filling | Not implemented (Stage 3) |
| Form flattening | Not implemented (Stage 3) |
| XFA detection and routing | Not implemented |

## Related Pages

- [Tables, Forms, and Financial Documents](../structures/tables.md) — form/key-value extraction context
- [Information Extraction](information-extraction.md) — common form extraction targets
- [PDF Engines](../tools/pdf-engines.md) — pdfium provides form rendering
