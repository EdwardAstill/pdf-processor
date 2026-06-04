# Pipeline (`src/pipeline/`)

Glues all the modules together into the per-page build process. Owns the order of operations: extract → classify → detect tables → detect formulas → detect figures → merge → render.

## Source files

| File | Purpose |
|---|---|
| `mod.rs` | Main pipeline orchestration, page building, candidate-to-block conversion |
| `merge.rs` | Block merge helpers: formula suppression inside tables, text/table interleaving, formula/text suppression, media merge |

## Key types

| Type | Purpose |
|---|---|
| `PipelineContext` | Carries classifier, formula options, table options, figure options, output dir, page dimensions |
| `BuiltPage` | Output of `build_page()`: a `Page` struct with all blocks merged and ready for rendering |
| `FormulaRecord` | Audit record of each formula candidate: status, crop path, emission reason |
| `FormulaEmitMode` | `Conservative`, `Auto`, `All`, `None` — controls whether formula blocks are emitted |

## Key functions

### Page building (`mod.rs`)

| Function | Description |
|---|---|
| `build_page(raw_page, ctx) -> Result<BuiltPage>` | The main per-page build function. Order of operations: |
| | 1. Build XY-Cut reading order |
| | 2. Detect coordinate tables + geometry tables |
| | 3. Suppress overlapping table candidates |
| | 4. Filter emit-worthy tables |
| | 5. Detect formula candidates, suppressing those inside table regions |
| | 6. Optionally run visual band scanning (`--debug-formulas`) |
| | 7. Classify text blocks with metadata |
| | 8. Suppress text covered by furniture and tables |
| | 9. Convert candidates → blocks (table, formula) |
| | 10. Suppress text covered by formulas |
| | 11. Detect figure candidates |
| | 12. Save embedded images and render figure snapshots |
| | 13. Merge text + tables + formulas + media (images, figures) into final block list |

### Merge helpers (`merge.rs`)

| Function | Description |
|---|---|
| `suppress_formula_candidates_overlapping_tables(candidates, tables)` | Remove formula candidates whose bbox overlaps a high-confidence table region |
| `suppress_text_covered_by_tables(text_blocks, table_candidates)` | Remove classified text blocks that overlap detected table regions (prevent double-rendering) |
| `suppress_text_covered_by_formulas(text_blocks, formula_blocks)` | Same for formula blocks |
| `merge_text_and_tables(text_blocks, table_blocks)` | Interleave table blocks into the text block list by Y position |
| `merge_text_and_formulas(text_blocks, formula_blocks)` | Interleave formula blocks, removing duplicate source text |
| `merge_media_blocks(image_blocks, figure_blocks)` | Concatenate image and figure blocks |
| `formula_candidates_to_blocks(...)` | Convert `FormulaCandidate`s into `Block::Formula` or `Block::FormulaReview` |
| `table_candidates_to_blocks(...)` | Convert `TableCandidate`s into `Block::CoordinateTable` |

## Pipeline flow diagram

```
PDF → [MuPDF] → RawPage
                    │
                    ▼
            XY-Cut++ reading order
                    │
                    ▼
     ┌─────────────────────────────┐
     │  Table detection             │
     │  (coordinate + geometry)     │
     └──────────┬──────────────────┘
                │
     ┌──────────▼──────────────────┐
     │  Formula detection           │ ← suppressed inside strong tables
     └──────────┬──────────────────┘
                │
     ┌──────────▼──────────────────┐
     │  Block classification        │
     └──────────┬──────────────────┘
                │
     ┌──────────▼──────────────────┐
     │  Suppress overlapping        │
     │  text (furniture, tables,    │
     │  formulas)                   │
     └──────────┬──────────────────┘
                │
     ┌──────────▼──────────────────┐
     │  Figure detection            │
     └──────────┬──────────────────┘
                │
     ┌──────────▼──────────────────┐
     │  Merge all blocks → Page     │ → Markdown renderer
     └─────────────────────────────┘
```

## CLI flags

| Flag | Effect |
|---|---|
| `--tables`, `--debug-tables` | Table detection mode and debug output |
| `--formulas`, `--debug-formulas`, `--formula-sidecar`, `--formula-emit` | Formula detection and sidecar routing |
| `--figures`, `--debug-figures`, `--figure-dpi`, `--figure-padding` | Figure detection and rendering |
| `--conservative` | Preset: layout tables, audit-only formulas, embedded-only figures |
| `--no-images` | Skip embedded image saving and figure rendering |
| `--hybrid docling` | After local pipeline, route selected pages to Docling backend |

## Cross-references

- [layout-analysis.md](layout-analysis.md) — table detection internals
- [formula-detection.md](formula-detection.md) — formula candidate detection and sidecar
- [figure-extraction.md](figure-extraction.md) — figure snapshot extraction
- [markdown-rendering.md](markdown-rendering.md) — what happens to the assembled blocks
- [ocr-preprocessing.md](ocr-preprocessing.md) — OCR runs before the pipeline
- [hybrid-backend.md](hybrid-backend.md) — hybrid enrichment runs after the pipeline
