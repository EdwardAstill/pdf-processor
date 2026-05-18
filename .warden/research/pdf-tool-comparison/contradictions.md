# Contradictions And Caveats

- `pdfp` has measured fixture scores, but most third-party rows in the first
  table are sourced feature claims, not locally measured scores. The comparison
  must not state broad parity until those tools run on the same fixtures.
- Marker publishes favorable benchmark numbers, but they are vendor-provided
  and hardware/model dependent. Treat Marker as a useful benchmark oracle, not
  independent ground truth.
- Cloud APIs may outperform local tools on hard OCR/layout, but they require
  uploads, credentials, and paid quotas. That is a product tradeoff, not a pure
  quality win.
- Local ML systems such as Docling, Marker, and MinerU can be stronger on
  layout semantics but carry heavier runtime/model requirements than `pdfp`.
