# Quickstart

The three most common `pdfp` workflows in 30 seconds.

## 1. Convert a single PDF to Markdown

```sh
pdfp convert paper.pdf
```

Output: `paper/paper.md` plus extracted images in `paper/images/`. The Markdown uses clean reflowed paragraphs, GFM tables when detected, `$$...$$` for high-confidence formulas, and `![image](images/...)` for embedded images.

```sh
# With verbose progress
pdfp convert paper.pdf --verbose

# Review-safe mode for engineering/legal documents (no speculative tables/formulas)
pdfp convert standard.pdf --conservative

# Render figure snapshots instead of raw embedded images
pdfp convert paper.pdf --figures snapshot --figure-dpi 200
```

## 2. Convert a whole directory

```sh
pdfp convert papers/ -o out/
```

Converts every PDF in `papers/`, writing Markdown next to each source or to `out/`. Use quoted globs for filtering:

```sh
pdfp convert "papers/2024-*.pdf" -o out/2024/
```

## 3. Debug conversion quality

```sh
# Audit tables, formulas, and figures
pdfp convert paper.pdf --debug-tables --debug-formulas --debug-figures -o out/

# What's happening under the hood?
pdfp inspect paper.pdf --json

# Check if OCR tools are available
pdfp doctor
```

Debug output goes under `out/paper/debug/`:
- `tables/page1.json` — detected table candidates with evidence and confidence
- `formulas/page1.json` — formula candidates with crop paths
- `formulas/index.json` — aggregate formula audit ledger
- `figures/page1.json` — detected figure candidates
- `formulas/page1_formula1.png` — rendered equation crops

## Next steps

- Full CLI reference: [CLI.md](CLI.md)
- How the pipeline works: [reference/pipeline.md](reference/pipeline.md)
- Architecture and module map: [architecture.md](architecture.md)
- Comparison with other tools: [TOOL_COMPARISON.md](TOOL_COMPARISON.md)
