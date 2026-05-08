# Wiki Findings

## Local `convert2` Wiki

Relevant artifacts:

- `wiki/opendataloader-ecosystem.md`
- `wiki/project-comparison-matrix.md`
- `wiki/improvement-opportunities.md`
- `wiki/markdown-rendering.md`

What they imply now:

- `cnv` should stay deterministic and local by default.
- Page and region routing should be explicit.
- Tagged PDF, OCR, table, formula, and debug paths should remain modular.
- Renderer code should format known structure; it should not become the home for layout recovery, routing, or document subtype logic.
- The highest-value future improvements are tagged-PDF structure, OCR preprocessing, numeric-heavy tables, geometry-aware table detection, and debug artifacts.

Conflict or gap:

- The wiki points at feature priorities, but this turn asked for a refactor. The safe translation is to improve module boundaries, not build the first feature in the priority list.

## Warden Refactoring Wiki

Relevant artifact:

- `/home/eastill/projects/warden/wiki/knowledge/refactoring-code-intelligence.md`

What it implies now:

- This is a structural refactor because code crosses module boundaries.
- The impact surface should be mapped with callers, tests, and imports before editing.
- The change should be one reversible green step, verified with tests.

Conflict or gap:

- No graph backend is needed for this repo-sized change; `rg`, source reads, and `cargo test` are sufficient evidence.
