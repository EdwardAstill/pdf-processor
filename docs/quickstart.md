# Quickstart

The three most common `pdfp` workflows in 30 seconds.

## 1. Convert a single PDF to Markdown

```sh
pdfp paper.pdf
```

Output: `paper.md`. The Markdown uses clean reflowed paragraphs, GFM tables when detected, `$$...$$` for high-confidence formulas, and OCR automatically when the PDF looks scan-heavy.

```sh
# With verbose progress
pdfp paper.pdf --verbose

# Also save visual assets
pdfp paper.pdf --images --tables --equations

# Force OCR when the embedded text layer is damaged
pdfp bad-text-layer.pdf --ocr force --lang eng
```

## 2. Convert a whole directory

```sh
pdfp convert papers/ -o out/
```

Converts every PDF in `papers/`, writing Markdown next to each source or to `out/`. Use quoted globs for filtering:

```sh
pdfp convert "papers/2024-*.pdf" -o out/2024/
```

## 3. Inspect or Troubleshoot

```sh
# What's happening under the hood?
pdfp inspect paper.pdf --json

# Check if OCR tools are available
pdfp doctor
```

For conversion asset output, use `--images`, `--tables`, and `--equations`. For a single PDF with `-o out/`, output goes directly under `out/`; directory and glob conversion use one subdirectory per input to avoid asset collisions.

## Next steps

- Full CLI reference: [CLI.md](CLI.md)
- How the pipeline works: [reference/pipeline.md](reference/pipeline.md)
- Architecture and module map: [architecture.md](architecture.md)
- Comparison with other tools: [TOOL_COMPARISON.md](TOOL_COMPARISON.md)
