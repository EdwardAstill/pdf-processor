# Contradictions And Caveats

- The fixture-level table recall number is excellent (`4/4`) but it is not a
  sufficient table-quality claim. It coexists with broad false-positive table
  regions because current eval only measures whether a page has at least one
  table block.
- Camelot, pdfplumber, and Tabula are not drop-in proof that one algorithm is
  universally best. They support the architectural conclusion that strategies
  should be separated, scored, debugged, and region-scoped.
- Table Transformer and Docling are stronger evidence for optional sidecar or
  benchmark paths than for making ML mandatory. This project still needs a
  deterministic local path that works without heavy runtime dependencies.
- Debug conversion emits broad table candidates for more pages than the tracked
  eval currently scores. The recorded precision figures are therefore fixture
  precision, not whole-document precision.
