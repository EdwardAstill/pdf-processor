---
title: "CJK and RTL Text Handling"
kind: "knowledge"
category: "wiki"
summary: "How Chinese, Japanese, Korean (CJK) and right-to-left (Arabic, Hebrew) text is encoded in PDFs: CID fonts, CMaps, ToUnicode, the Bidi Algorithm, and extraction failure modes with fallback strategies."
entities: [CID-fonts, CMap, ToUnicode, Bidi-Algorithm, CJK, RTL, MuPDF, pdfium]
---

# CJK and RTL Text Handling

CJK and RTL text in PDFs has unique failure modes that generic text extraction does not cover. MuPDF handles the common cases well, but edge cases — CID fonts without ToUnicode maps, Arabic ligatures, mixed-direction text — need explicit detection and fallback.

---

## CJK Text in PDFs

### Font Architecture

CJK text uses **CID-keyed fonts** — composite fonts (Type 0) where glyphs come from a CIDFont (CIDFontType0 for Type 1 outlines, CIDFontType2 for TrueType outlines). This is different from simple fonts like Type1 or standard TrueType.

### The Double Mapping Problem

To get Unicode text from a CID font, two mappings are needed:

1. **CMap**: character code → CID (glyph index). Multiple CMaps exist for different encodings:
   - Adobe-GB1 (Simplified Chinese)
   - Adobe-CNS1 (Traditional Chinese)
   - Adobe-Japan1 (Japanese)
   - Adobe-Korea1 (Korean)
   - Identity-H / Identity-V (direct CID mapping, horizontal/vertical)

2. **ToUnicode CMap**: CID → Unicode code point. This is a PDF stream resource. When present, extraction works. When missing or incomplete, extraction produces garbage.

### Common Failure Modes

**Missing ToUnicode CMap**: The most common failure. Text extraction returns `(cid:xxx)` marker text or U+FFFD (replacement character) for every glyph. The PDF looks fine on screen (the font has correct glyph outlines), but the Unicode mapping is absent.

**Incomplete CMap packs**: pdfminer.six requires pre-built CMap resource files for CJK extraction. If the CMap pack for the encoding is missing, CJK text extraction fails entirely.

**MuPDF** is the best-in-class for CJK because it ships with built-in CMap databases for all major CJK encodings. Most CJK PDFs extract correctly through MuPDF without configuration.

### Detection and OCR Fallback

The OpenDataLoader project (PR #291) added a practical detection heuristic:
- Count U+FFFD characters in extracted text
- If ≥30% of characters are replacement characters → flag page as extraction failure
- Route flagged pages to OCR fallback

This same heuristic should work for `pdfp`. If a page has unusually high U+FFFD density, the CID font mapping has failed and OCR is the only recovery path.

---

## RTL Text (Arabic, Hebrew)

### The Core Challenge

PDF stores text in **visual (display) order**, not logical order. When extracting Arabic or Hebrew:
- Characters are extracted in reverse order
- Ligatures (lam-alef in Arabic) may not decompose correctly
- Digits and punctuation embedded in RTL text break the extraction order further

The fix is to apply the **Unicode Bidirectional Algorithm (UAX #9)** as a post-processing step. This converts visual-order text to logical order for correct storage and rendering.

### Specific Failure Patterns

**Reverse word order**: Arabic words appear backwards in extraction.
**Ligature decomposition**: Lam-alef (ﻻ) does not decompose into lam (ﻝ) + alef (ﺍ).
**Mixed RTL+digits**: Numbers embedded in Arabic text break extraction order — digits extracted at wrong positions relative to surrounding Arabic text.

### Known Library Status

| Library | RTL support |
|---|---|
| MuPDF | Extracts in visual order; no automatic Bidi correction |
| pdfminer.six | No RTL support (open issue #515 since 2019) |
| pypdf | Partial: form fields support RTL; `extract_text()` needs post-processing |
| pdfplumber | Same as pdfminer.six — visual order |
| pdfium | Can extract in logical order via struct-tree role information |

### Post-Extraction Fix

After extracting text from an RTL page:
1. Detect RTL script (Unicode script property for Arabic/Hebrew blocks)
2. Apply the Unicode Bidi Algorithm to reorder characters into logical order
3. Handle ligature recomposition

In Rust, the `unicode-bidi` crate provides Bidi Algorithm implementation. In Python, `python-bidi` does the same.

---

## Relevance to pdfp

### CJK
- **Today**: MuPDF handles most CJK PDFs correctly. CJK is not `pdfp`'s most urgent gap.
- **Detection**: Add U+FFFD density check during extraction. When density exceeds threshold, emit warning and suggest `--ocr` fallback.
- **Stage 3**: Explicit CJK CMap validation for known-failure encodings.

### RTL
- **Today**: No RTL post-processing. Extracted Arabic/Hebrew is in visual order.
- **Quick win**: Add Bidi reordering as a post-extraction step. `unicode-bidi` crate is ~500 LOC, no system dependencies.
- **Integration point**: Add `text_cleanup` pass in `src/pdf/text_cleanup.rs` that detects RTL pages and applies Bidi reordering to extracted blocks.

---

## Current State in pdfp

| Capability | Status |
|---|---|
| CJK extraction (happy path) | Works via MuPDF CMap database |
| CJK failure detection (U+FFFD check) | Not implemented |
| OCR fallback for CJK | Via `--ocr` command, no auto-routing |
| RTL Bidi reordering | Not implemented |
| Mixed-direction text handling | Not implemented |

## Related Pages

- [Text Extraction](text-extraction.md) — general extraction challenges
- [Scans and OCR](scans-and-ocr.md) — OCR fallback strategy
- [PDF Engines](../tools/pdf-engines.md) — MuPDF vs pdfium CJK capabilities
