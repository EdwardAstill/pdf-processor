# Spec — Preserve spacing between subscript close and adjacent text

Date: 2026-04-24

## Goal

Stop `cnv` from emitting output like `A_{w}cross-sectional area of web` where a subscript group is glued directly to the following word. Insert a single space between a `_{...}` or `^{...}` group and any alphanumeric character that immediately follows it, when the source PDF had visible whitespace there.

## Why

Observed across every code/standards-style PDF converted with `cnv 0.1.0`. Examples from `DNV-RP-C202_2018 - Buckling of shells/DNV-RP-C202_2018 - Buckling of shells.md`:

```
A_{w}cross-sectional area of web, A_{w} = ht_{w}
f_{ak}reduced characteristic buckling strength
f_{akd}design local buckling strength
f_{E}elastic buckling strength
I_{po}polar moment of inertia
```

Expected:

```
A_{w} cross-sectional area of web, A_{w} = ht_{w}
f_{ak} reduced characteristic buckling strength
```

The symbol glossaries in engineering standards are dense tables of `<symbol>  <description>`. `cnv` already recovers the subscript structure as `_{...}`, but the text cleanup emits the subscript group without preserving the horizontal gap the PDF shows between the symbol and its definition. Downstream:

- Grep hits on symbol names collide with description words (`f_{ak}reduced` does not match `f_{ak}`).
- Markdown rendering produces unreadable runs of glued-together text.
- The issue is systemic — every symbol definition in every symbol-heavy doc is affected.

## Scope

In scope:
- Insert a space between a closing `}` of a `_{...}` / `^{...}` group and the next character, when (a) the next character is alphanumeric and (b) the PDF had measurable whitespace between the subscript group and the following glyph.
- Apply the fix in a single pass during the markdown render or text cleanup stage — whichever is cheaper and less invasive.

Out of scope:
- Refactoring how subscripts are detected or emitted.
- Switching to a different math delimiter (e.g. wrapping in `$...$`). That is a separate display-output concern.
- Fixing split subscripts like `M_{1,}_{Sd}` — related but a distinct bug (tracked separately).
- Handling the inverse case (missing space before `_{...}`); not observed in the corpus.

## Design

### Where the fix lives

Two candidate sites:

1. **`src/render/markdown.rs`** — where subscript/superscript spans are serialized. Most precise: the renderer knows the bounding-box gap to the next span and can emit a space only when the PDF had one.
2. **`src/pdf/text_cleanup.rs`** — post-processing pass on the already-serialized text. Simpler to implement as a regex substitution, but loses the ability to distinguish "glued in the PDF" from "visually separated in the PDF".

**Recommended:** fix in `src/render/markdown.rs` where span geometry is available. Fall back to a regex pass in `text_cleanup.rs` only if the renderer doesn't carry enough context.

### Geometry-aware rule (preferred)

When emitting a subscript/superscript group followed by another span:

- If the x-gap between the subscript group's right edge and the next span's left edge exceeds ~0.3× the span font size (a conservative "there is visible whitespace here" threshold), emit a single space after the closing `}`.
- Tuning value should be a named constant; make it easy to adjust during corpus testing.

### Regex fallback rule

If geometry is not reachable in the render pass, add a post-cleanup regex in `text_cleanup.rs`:

```regex
(_\{[^}]+\}|\^\{[^}]+\})([A-Za-z])
```

Replace with `$1 $2`. Apply once per line. Document the limitation: this inserts a space wherever a subscript is followed by a letter, even if the source PDF had no gap. The tradeoff is acceptable because in standards-style documents the gap is almost always present — the regex under-corrects rare symbol-concatenation cases (e.g. `x_{0}y_{0}` in formulas) but that is better than the current systemic failure.

If adopting the regex fallback, preserve the original spacing when the closing brace is already followed by whitespace, punctuation, or an opening brace (function composition).

### Pick one

Implementation should attempt geometry-aware first. If the renderer's per-span context makes that awkward, document the decision and use the regex fallback with the caveat above.

## Acceptance Criteria

1. Converting any PDF containing a symbol glossary with `<symbol>  <description>` pairs produces output where the symbol and description are separated by a single space.
2. Converting a formula with adjacent symbols (e.g. `x_{0}y_{0}`) does not gain spurious spaces when using the geometry-aware path. (If the regex fallback is used, document this as a known minor over-correction.)
3. Snapshot tests for representative fixtures (one symbol-glossary doc, one math-dense doc) update with the new spacing; review confirms the change is net-positive on every diff.
4. `cargo test` remains green.

## Test Plan

- **Unit:** in `src/render/markdown.rs` (or `src/pdf/text_cleanup.rs` if fallback), add tests covering:
  - Subscript immediately followed by alphabetic word → space inserted.
  - Subscript immediately followed by `=` or other punctuation → no change.
  - Subscript immediately followed by whitespace → no double-space.
  - Nested subscript patterns (`A_{i,j}`).
- **Fixture snapshot:** add a synthetic fixture PDF (or reuse a scan-light DNV doc) that contains a known symbol glossary. Compare rendered markdown before/after. Commit the "after" snapshot.
- **Corpus spot-check:** before merging, rerun on the Pyseas `extra/literature/pdfs/` corpus and grep for the characteristic glued pattern:
  ```
  rg '_\{[^}]+\}[A-Za-z]' markdown-cnv/
  ```
  Expect the count to drop from thousands to near-zero.

## Risks

- **Over-correction on legitimate adjacency:** formulas like `x_{i}y_{j}` or `B_{1}C_{2}` — symbols multiplied in sequence. The geometry-aware path handles this; the regex fallback mis-spaces them. Accept as a known tradeoff if using the fallback.
- **Per-doc snapshot churn:** every doc with a symbol glossary will show diffs. Expected; review one representative diff and batch-accept the rest.

## Open questions

- Does the renderer have span-geometry context at the point where subscript close is emitted, or has it already been flattened into a text buffer? Determines which implementation path is taken.
- Should the same rule apply before an opening `_{` when the preceding token is a bare letter (e.g. pattern `wordA_{i}` → `word A_{i}`)? Not observed in the corpus but logically symmetric. Defer unless a case arises.
