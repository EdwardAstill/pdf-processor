# Table System Review Research Plan

Status: recorded
Date: 2026-05-15 AWST
Branch: stage-8-heading-formula

## Question

Review the local table extraction code, compare it with established PDF table
systems, identify refactors, and adjust future benchmark stages so table quality
does not silently regress while image/vector work starts.

## Scope

- Local files:
  - `src/layout/table.rs`
  - `src/layout/table_detector.rs`
  - `src/layout/drawing_ops.rs`
  - `src/layout/classifier.rs`
  - `src/pipeline/mod.rs`
  - `src/pipeline/merge.rs`
  - `src/render/markdown.rs`
  - `src/eval/*`
- External primary sources:
  - pdfplumber official README
  - Camelot official documentation
  - Tabula Java official README
  - Microsoft Table Transformer / PubTables-1M repository and papers
  - Docling official model documentation

## Method

1. Re-run the current eval to anchor table precision.
2. Inspect local table detection, merge, render, and eval paths.
3. Run conversion with table debug output for the two engineering fixtures.
4. Compare the current architecture with external primary sources.
5. Record findings, recommended refactor order, and roadmap changes.

## Non-Goals

- Do not change table extraction code in this review pass.
- Do not vendor or adopt an external ML model until local eval has region-level
  table precision metrics.
- Do not replace the local Rust path with a Python/JVM dependency as the default
  fast path.
