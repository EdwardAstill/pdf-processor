---
title: "PDF Metadata and XMP"
kind: "knowledge"
category: "wiki"
summary: "How PDF stores document metadata: the legacy Info dictionary, modern XMP metadata streams, Dublin Core, PDF/A conformance data, custom namespaces, and Rust/Python APIs for reading and writing both."
entities: [XMP, Dublin-Core, PDF-A, xmpkit, xmp_writer, lopdf, pypdf]
---

# PDF Metadata and XMP

PDF has two overlapping metadata systems. Both can exist in the same file. They can contain conflicting information. Different tools read from different locations. Understanding both matters for reliable metadata extraction.

---

## Document Information Dictionary (Legacy)

The **Info dictionary** has been in PDF since version 1.0. It is referenced from the trailer via the `/Info` key.

### Standard entries

| Key | Type | Description |
|---|---|---|
| `/Title` | text string | Document title |
| `/Author` | text string | Author name |
| `/Subject` | text string | Document subject |
| `/Keywords` | text string | Comma-separated keywords |
| `/Creator` | text string | Application that created the document |
| `/Producer` | text string | Application that produced the PDF |
| `/CreationDate` | date string | When the document was created |
| `/ModDate` | date string | When the document was last modified |

### Limitations
- Only supports PDFDocEncoding or UTF-16BE text strings
- No structured metadata (no hierarchical data, no custom schemas)
- No way to express relationships between metadata fields
- Cannot describe metadata for sub-document objects (pages, images)

---

## XMP Metadata (Modern, ISO 16684)

XMP (Extensible Metadata Platform) stores metadata as an **XML stream** in the document catalog (`/Metadata` entry). It is serialized in RDF/XML and supports multiple namespaces.

### Namespaces

| Namespace | URI | Typical use |
|---|---|---|
| Dublin Core | `http://purl.org/dc/elements/1.1/` | Title, creator, subject, description, date |
| XMP Core | `http://ns.adobe.com/xap/1.0/` | CreateDate, ModifyDate, MetadataDate, CreatorTool |
| XMP Rights | `http://ns.adobe.com/xap/1.0/rights/` | Copyright, usage terms |
| PDF | `http://ns.adobe.com/pdf/1.3/` | Keywords, PDFVersion, Producer |
| PDF/A ID | `http://www.aiim.org/pdfa/ns/id/` | `pdfaid:part`, `pdfaid:conformance` |
| Custom | Any URI | Application-specific properties |

### PDF/A conformance in XMP

PDF/A-conforming documents embed conformance information in XMP:

```xml
<pdfaid:part>2</pdfaid:part>
<pdfaid:conformance>B</pdfaid:conformance>
```

This tells you the document is PDF/A-2B. This is the most reliable way to detect PDF/A compliance without running a full validator.

---

## Info Dict vs XMP: The Relationship

The relationship is **not symmetric**:

- **PDF/A-1** requires Info dict values to be *mirrored* in XMP — if `/Title` says "Report", XMP must also say "Report"
- XMP can contain *more* metadata than the Info dict (custom fields, structured data)
- When both exist and conflict: XMP is authoritative for PDF/A-conforming files, but no guarantee in other PDFs
- PDF 2.0 still supports both, but XMP is the canonical metadata store

### Migration pattern (TechNote 0003)

When migrating to PDF/A:
1. Read existing Info dict entries
2. Map to Dublin Core / XMP equivalents
3. Write XMP metadata stream
4. Keep Info dict for backward compatibility (mirror values)

---

## Rust Libraries

| Library | Read | Write | Notes |
|---|---|---|---|
| **lopdf** | Info dict only | Info dict only | `PdfMetadata` struct: title, author, subject, keywords, creator, producer, dates, page count, version. No XMP parsing. |
| **xmp_writer** | No | Yes | Step-by-step XMP construction. Output as byte vector. Simple API. |
| **xmpkit** | Yes | Yes | Pure-Rust. Compatible with Adobe XMP standard. Dublin Core + XMP Core namespaces. |
| **rpdfium_edit** | Info dict + XMP | Info dict + XMP | Full PDF editing via pdfium. |

### Accessing with lopdf
`lopdf` provides `PdfMetadata::read(doc)` which quickly extracts Info dict fields without loading the full document. This is useful for `pdfp inspect` or a future `--metadata` flag. XMP streams require separate XML parsing — `lopdf` can give you the raw stream bytes, but you need an XML/RDF parser to extract structured XMP data.

---

## Relevance to pdfp

### Metadata extraction (quick win)
`pdfp inspect --metadata` — extract and display both Info dict and XMP metadata. Info dict via MuPDF/lopdf; XMP via `quick-xml` (already a dependency).

### PDF/A detection
Read XMP → check `pdfaid:part` and `pdfaid:conformance`. Flag in output: `<!-- pdfa: 2B -->`.

### Inline formatting (wiki gap #5)
XMP may contain font embedding metadata that supplements missing font-weight information. When `pdfium-metadata` is unavailable and MuPDF doesn't expose font names, XMP font descriptors provide a fallback for bold/italic detection.

### Metadata preservation
When `pdfp` modifies and saves PDFs (Stage 2 encrypt, optimize, crop), preserve existing Info dict and XMP metadata. `lopdf` can carry Info dict through — XMP requires explicit stream preservation.

---

## Current State in pdfp

| Capability | Status |
|---|---|
| Info dict extraction | Available via MuPDF document metadata |
| XMP extraction | Not implemented |
| PDF/A detection | Not implemented |
| Metadata preservation on save | Not implemented |
| Font metadata from XMP | Not implemented |

## Related Pages

- [PDF Engines](../tools/pdf-engines.md) — MuPDF, pdfium, lopdf capabilities
- [Rust Crates](../tools/rust-crates.md) — quick-xml, lopdf
- [Inline Formatting](../structures/inline-formatting.md) — font metadata gap
