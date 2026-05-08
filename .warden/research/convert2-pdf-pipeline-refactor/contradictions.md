# Contradictions and Gaps

## True Contradictions

No hard contradiction found. The external sources and local wiki agree on a deterministic-first, hybrid-when-needed architecture.

## Different Use Cases

- OpenDataLoader and `pdf-inspector` emphasize deterministic local parsing and routing signals.
- Docling, Marker, MinerU-style approaches emphasize model/OCR/table recovery for hard pages.
- PyMuPDF4LLM sits close to `cnv`'s MuPDF-backed fast path, but its API surface is Python-oriented.

These are different operating points, not conflicting advice.

## Unsupported or Risky Claims

- Some benchmark numbers in project READMEs are project-reported. Treat them as directional until independently reproduced.
- Tagged-PDF and OCR improvements are feature work and are outside this refactor.

## Refactor Decision

The evidence supports extracting `main.rs`'s PDF processing body into a named pipeline module. That is narrow, behavior-preserving, and makes future tagged-PDF, OCR, page-routing, and table passes easier to introduce without growing `main.rs` or the renderer.
