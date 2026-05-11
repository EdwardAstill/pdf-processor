---
title: "Inline Formatting (Bold, Italic, Code, Underline)"
kind: "reference"
category: "wiki"
summary: "How bold, italic, underline, and code spans are encoded in PDFs and how they map to Markdown inline markup."
virtual_path: "wiki/structures/inline-formatting"
entities: [bold, italic, underline, code-span, font-weight, font-style]
---

# Inline Formatting

Inline formatting — **bold**, *italic*, `code`, underline — adds emphasis and semantic signals within a paragraph. In technical documents it conveys defined terms, variables, warnings, and code references.

## How PDFs Encode Inline Formatting

PDF has no semantic markup layer analogous to HTML `<strong>` or `<em>`. All inline formatting is visual:

| Effect | PDF encoding |
|--------|-------------|
| **Bold** | Font family with "Bold" in name, or font weight ≥ 700 |
| *Italic* | Font family with "Italic" or "Oblique" in name, or font matrix skew |
| Bold italic | Both conditions above |
| Underline | Separate drawing operation (line below text baseline) |
| Strikethrough | Separate drawing operation (line through text midline) |
| `Code / monospace` | Monospace font family (Courier, Consolas, Source Code Pro, etc.) |

Tagged PDFs may carry `<Span>` elements with role attributes, but inline formatting roles (`Strong`, `Em`) are less commonly used than block-level roles.

## Extracting Inline Formatting in `pdfp`

`pdfp` uses mupdf 0.6 which does not expose font names in the Rust wrapper — only font size. This means:

- **Bold detection**: Not available in default build. Available via `--features pdfium-metadata` (pdfium-render exposes font weight).
- **Italic detection**: Not available in default build. Available via pdfium-render font style flags.
- **Monospace/code detection**: Partially available — can compare font size pattern but not family name without pdfium.
- **Underline**: Drawing operations are accessible via MuPDF path data; requires a separate pass over the page's graphic elements.

Without pdfium-metadata, `pdfp` does not emit `**bold**` or `*italic*` in output. This is a known gap.

## Markdown Mapping

| PDF signal | Markdown | Notes |
|-----------|----------|-------|
| Font weight ≥ 700 | `**text**` | Bold |
| Font style italic/oblique | `*text*` | Italic |
| Both bold and italic | `***text***` | Bold italic |
| Monospace font family | `` `text` `` | Code span |
| Underline | No standard Markdown | Drop or use HTML `<u>` |
| Strikethrough | `~~text~~` | GFM extension |

## Scope Decisions

Inline formatting detection should be span-level, not block-level:

- A fully-bold short line is likely a heading, not a paragraph with bold formatting (handle in heading classification)
- A partially-bold span within a sentence is inline bold
- The boundary: if >80% of a block's characters are bold/italic, treat as a structural element; otherwise treat as inline formatting

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| All body text comes out bold | Font weight threshold too low | Require weight > 600 and compare to paragraph norm |
| Variable names in equations lose italic | Italic detection absent (mupdf limitation) | pdfium-metadata feature; or heuristic: single letter surrounded by non-italic = likely variable |
| Code in a monospace paragraph not code-fenced | Monospace detection absent | Detect font family name via pdfium or heuristic on character set |
| Emphasis in standards lost | No inline formatting extraction at all | Known gap; needs pdfium-metadata or a post-extraction pass |
| Underlines rendered as `<u>` break LLM context | HTML in Markdown | Drop underlines unless semantic (hyperlinks); underline is rarely meaningful in technical text |

## Engineering Standards Notes

In technical standards, inline formatting carries specific meaning:

- **Bold** often marks defined terms on first use
- *Italic* often marks variable names and quantities (σ_y, F_Ed, L)
- `Monospace` marks code, command names, file paths
- Underline rarely appears in modern standards

When pdfium-metadata is not available, defined terms and variable names blend into surrounding prose. This is acceptable for plain extraction but degrades semantic quality for downstream NLP.

## See Also

- [algorithms/heading-classification.md](../algorithms/heading-classification.md) — how to distinguish "fully bold block" from "bold heading"
- [structures/equations.md](equations.md) — italic variable names inside display math
- [structures/headings.md](headings.md) — heading detection uses bold signal
