---
title: "Lists"
kind: "reference"
category: "wiki"
summary: "How lists are encoded in PDFs, how to detect ordered and unordered lists, nesting, and how they map to Markdown."
virtual_path: "wiki/structures/lists"
entities: [list-detection, bullet-normalisation, nesting, tagged-PDF]
---

# Lists

Lists are common in technical documents — specifications, requirements, procedures, standards clauses. They are easy to get right when the PDF is tagged and easy to get wrong when it is not.

## How PDFs Encode Lists

PDFs have no native list element at the content-stream level. A bulleted list is visually rendered text — each item is a separate text span or block, distinguished from body prose by:

- a leading bullet character (`•`, `–`, `◦`, `▪`, numbers, letters)
- consistent left indentation
- consistent inter-item vertical spacing (usually less than paragraph spacing)

Tagged PDFs carry `/L`, `/LI`, `/Lbl`, `/LBody` structure roles. When present these are authoritative.

## Detection Without Tags

Without struct-tree roles, list detection relies on these signals:

| Signal | Unordered list | Ordered list |
|--------|---------------|-------------|
| Leading character | `•`, `–`, `◦`, `▪`, `*`, `›` | `1.`, `a)`, `(i)`, `A.` |
| Left indent relative to body | > 0, consistent per list | > 0, consistent per list |
| First-line vs continuation indent | First line has bullet; continuation hangs | Same |
| Inter-item spacing | < paragraph spacing, consistent | Same |
| Line length | Usually shorter than body | Same |

### Bullet normalisation

PDFs frequently encode bullets as:
- Unicode bullet characters (U+2022 `•`, U+2013 `–`, U+25AA `▪`)
- Private Use Area glyphs (font-specific; appear as garbage without font lookup)
- Repeated en-dashes or hyphens

All should normalise to a single Markdown bullet `-`. The original glyph is not semantically meaningful.

### Ordered list marker detection

Common patterns to recognise:
- Arabic numerals: `1.`, `1)`, `(1)`
- Letters: `a.`, `a)`, `(a)`, `A.`
- Roman numerals: `i.`, `ii.`, `iii.`, `(i)`
- Section-style: `1.1`, `1.1.1` (these are headings, not list items — do not confuse)

### Hanging indent (continuation lines)

Multi-line list items have a "hanging indent" — the continuation lines are indented to align with the text start of the first line, not with the bullet. The bullet glyph appears only on the first line. Good detection checks for this pattern before concluding that the second line is a new item.

## Nesting

Nested lists are common in standards (requirements with sub-requirements). Detection:

1. Measure the indent level of each candidate item
2. Quantise to N levels (typically 2–3 in most documents)
3. Track level changes to open/close nested list blocks

Pitfalls:
- Inconsistent indentation in the PDF source means quantisation thresholds matter
- Some PDFs use visual indent without changing the actual X coordinate — use font size or explicit indent markers instead

## Common Failure Modes

| Symptom | Cause | Fix |
|---------|-------|-----|
| List items concatenated into a paragraph | Inter-item gap below paragraph-merge threshold | Tighten list-item gap detection |
| Bullet glyph appears in output as `?` or `□` | Private Use Area encoding | Map to `-` via font cmap lookup or heuristic |
| Continuation lines become new items | Hanging indent not detected | Check X of continuation vs bullet X |
| Numbered list items merged with body numbers | `1.` at start of body sentence triggers list detection | Require consistent repeating pattern, not single occurrence |
| Section headings (`1.2.3`) classified as list items | Ordered list marker regex over-triggers | Exclude items where the numbering depth exceeds 2 and font size suggests a heading |

## Markdown Rendering

```
- Item one
- Item two
  - Nested item
    - Double nested
- Item three

1. First
2. Second
   1. Sub-first
```

Markdown supports up to arbitrary nesting depth. `pdfp` emits standard GFM lists. Task lists (`- [ ]`) are used for checkbox-style form fields, not general lists.

## Tagged PDF

`/L` (List), `/LI` (List Item), `/Lbl` (Label/bullet), `/LBody` (item body). When struct-tree is available, the list hierarchy and item boundaries come directly from the tag tree with no heuristics needed.

## See Also

- [topics/text-extraction.md](../topics/text-extraction.md) — fragment merging that precedes list detection
- [structures/headings.md](headings.md) — avoiding confusion between numbered headings and ordered lists
