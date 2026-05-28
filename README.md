<p align="center">
  <img src="docs/assets/pdfp-logo.svg" alt="pdf-processor logo" width="220">
</p>

# pdf-processor

`pdf-processor` is a local PDF processor. Its most mature workflow is converting PDFs into AI-friendly markdown, and the same binary now also has inspection, metadata, search, page editing, imposition, and resizing commands.

**What makes pdfp different:**
- **Single binary, zero Python** — no GPU, no venv, just one ~46MB compiled binary
- **Offline-first** — all processing is local; no cloud API calls unless you opt in
- **Full pipeline** — extraction → layout analysis (XY-Cut++) → block classification → Markdown rendering, not just text dumping
- **Built-in eval** — precision/recall metrics for formula detection, heading accuracy, table recall
- **Page operations** — extract, delete, split, reorder, merge, resize, impose — all in the same tool
- **Conservative mode** — `--conservative` disables speculative reconstruction for engineering/legal documents

See [ARCHITECTURE.md](ARCHITECTURE.md) for the full architecture and [docs/TOOL_COMPARISON.md](docs/TOOL_COMPARISON.md) for how pdfp compares to Docling, MinerU, Marker, and other tools.

The active codepath is:

1. Open PDF with MuPDF.
2. Extract text blocks, word positions, and images.
3. Reconstruct reading order with XY-Cut++.
4. Classify blocks into headings, lists, coordinate tables, captions, and paragraphs.
5. Write markdown plus extracted images.

## Scope

**In active scope:**
- PDF input → Markdown conversion (the core workflow)
- PDF page operations: extract, delete, split, reorder, merge, resize
- Imposition: 2-up, booklet
- Inspection: metadata, page geometry, scan detection, text density
- Search: embedded text search with page reporting
- Document info metadata: read, set, clear
- Optional OCR preprocessing via OCRmyPDF
- Optional hybrid Docling assist for hard pages
- Quality evaluation against fixture JSON files

**Remaining polish areas:**
- Formula and standards-quality closeout
- Evaluation and benchmark polish
- PDF operations polish: outlines, annotations, encryption, optimize
- Forms, accessibility, international text, and hybrid routing polish

**Out of active scope at the repo root:**
- DOCX / EPUB / PPTX / HTML conversion (removed in v0.4.0)
- Markdown to Typst (removed in v0.4.0)
- SVG to PNG (removed in v0.4.0)
- RAG / KG / wiki / JSON export modes (removed in v0.4.0)

## Install

Install the latest Linux release with one command:

```sh
curl -fsSL https://github.com/EdwardAstill/pdf-processor/releases/latest/download/install.sh | sh
```

The installer places `pdfp` under `~/.local/share/pdfp`, symlinks it into `~/.local/bin`, and installs OCR dependencies with the platform package manager when they are missing. Set `PDFP_INSTALL_OCR=0` to skip OCR dependency installation.

To build from source, install `clang`/`libclang` and run:

```sh
cargo build --release --bin pdfp
```

## Usage

`pdf-processor` is a CLI tool. The installed command is `pdfp`.

```sh
pdfp <INPUT> [OPTIONS]
pdfp convert <INPUT> [OPTIONS]
pdfp ocr <INPUT> -o <OUTPUT> [--mode auto|force]
pdfp doctor [--json]
pdfp inspect <INPUT> [--json] [--ocr auto|force]
pdfp metadata <show|set|clear> ...
pdfp search <INPUT> <TEXT> [--json] [--ocr auto|force]
pdfp eval <FIXTURES_DIR>
pdfp pages <extract|delete|split|reorder|merge|rotate> ...
pdfp impose <2up|booklet> ...
pdfp page <resize|crop> <INPUT> -o <OUTPUT>
```

Bare `pdfp <INPUT>` remains a backwards-compatible alias for `pdfp convert <INPUT>`.

Every command and nested command has help:

```sh
pdfp --help
pdfp doctor --help
pdfp convert --help
pdfp ocr --help
pdfp eval --help
pdfp metadata set --help
pdfp pages extract --help
pdfp pages rotate --help
pdfp impose booklet --help
pdfp page resize --help
pdfp page crop --help
```

See the full CLI guide at [`docs/CLI.md`](docs/CLI.md). For an honest
comparison against Docling, PyMuPDF4LLM, Marker, MinerU, Mathpix, Adobe PDF
Extract, LlamaParse, and Unstructured, see
[`docs/TOOL_COMPARISON.md`](docs/TOOL_COMPARISON.md).

For conversion, `INPUT` can be:

- one PDF file
- a directory of PDFs
- a quoted glob like `"papers/*.pdf"`

### Convert

Main convert options:

| Flag | Default | Description |
| --- | --- | --- |
| `-o`, `--output <DIR>` | next to input | Output directory |
| `--min-h-gap <pts>` | `8.0` | XY-Cut horizontal-cut tuning |
| `--min-v-gap <pts>` | `12.0` | XY-Cut vertical-cut tuning |
| `--no-images` | off | Skip image extraction |
| `--conservative` | off | Prefer review-safe fallbacks over heuristic reconstruction |
| `--figures <MODE>` | `embedded` | Image output mode: `embedded`, `snapshot`, `both`, or `none` |
| `--figure-dpi <N>` | `200` | DPI for rendered figure snapshots |
| `--figure-padding <pts>` | `8.0` | Padding around detected snapshot regions |
| `--debug-figures` | off | Write figure candidate JSON under `debug/figures/` |
| `--tables <MODE>` | `auto` | Table output mode: `auto`, `native`, `layout`, or `off` |
| `--debug-tables` | off | Write table candidate JSON under `debug/tables/` |
| `--formulas <MODE>` | `auto` | Formula handling: `auto`, `local`, `hybrid`, or `off` |
| `--debug-formulas` | off | Write formula candidate JSON and rendered crops under `debug/formulas/` |
| `--formula-sidecar <SIDECAR>` | off | Run optional formula OCR on high-confidence crops; accepts commands, `cmd:<command>`, or `onnx:<model-dir>` in `onnx-ocr` builds |
| `--ocr <MODE>` | `off` | Local OCR preprocessing: `off`, `auto`, or `force` |
| `--ocr-lang <LANGS>` | `eng` | OCR languages passed to OCRmyPDF/Tesseract, for example `eng+deu` |
| `--ocr-cache-dir <DIR>` | off | Reuse searchable OCR derivative PDFs |
| `--ocr-timeout-secs <N>` | `600` | OCR command timeout |
| `--ocr-command <PATH>` | `ocrmypdf` | OCRmyPDF command path |
| `--hybrid <MODE>` | `off` | `off` or `docling` |
| `--hybrid-url <URL>` | `http://localhost:5001` | Hybrid backend base URL |
| `--hybrid-timeout-secs <N>` | `600` | Hybrid timeout |
| `--hybrid-policy <POLICY>` | `auto` | `auto` or `all` |
| `--hybrid-cache-dir <DIR>` | off | Reuse per-page Docling markdown |
| `-v`, `--verbose` | off | Print progress to stderr |

Examples:

```sh
# One PDF
pdfp convert paper.pdf

# Whole directory
pdfp convert papers/ -o out/ --verbose

# Quoted glob
pdfp convert "papers/*.pdf" -o out/

# Render complete visual figure regions instead of raw embedded image objects
pdfp convert paper.pdf --figures snapshot --figure-dpi 200 -o out/

# Keep both rendered figure snapshots and embedded image objects for inspection
pdfp convert paper.pdf --figures both --debug-figures -o out/

# Preserve hard catalogue tables as fixed-width layout blocks
pdfp convert catalogue.pdf --tables layout -o out/

# Review-safe conversion: no speculative Markdown tables or formula rendering
pdfp convert standard.pdf --conservative --debug-formulas --debug-tables -o out/

# Force coordinate-derived Markdown tables where possible
pdfp convert catalogue.pdf --tables native --debug-tables -o out/

# Audit formula candidates and rendered equation crops
pdfp convert standard.pdf --debug-formulas -o out/

# Recover high-confidence formula crops with a local sidecar command
pdfp convert standard.pdf --formula-sidecar rapid-latex-ocr --debug-formulas -o out/

# Route formula-heavy pages through Docling enrichment
pdfp convert standard.pdf --hybrid docling --formulas hybrid -o out/

# Hybrid assist for harder pages
pdfp convert math-paper.pdf --hybrid docling -o out/

# Hybrid assist with cached OCR/table output
pdfp convert scan.pdf --hybrid docling --hybrid-cache-dir .pdfp-cache -o out/

# Local OCR sidecar for image-only or scan-heavy PDFs
pdfp convert scan.pdf --ocr auto --ocr-lang eng --ocr-cache-dir .pdfp-ocr -o out/

# Force OCR when the embedded text layer is broken
pdfp convert bad-text-layer.pdf --ocr force --ocr-lang eng+deu -o out/
```

Local OCR uses `ocrmypdf`, which in turn needs Tesseract and its language packs. OCR is not part of the default path. With `--ocr auto`, clean born-digital PDFs skip OCR even if OCRmyPDF is not installed; scan-heavy PDFs fail with an actionable missing-command message if OCR is requested but unavailable.

`pdfp` resolves OCRmyPDF in this order:

1. `--command <PATH>` / `--ocr-command <PATH>`
2. `PDFP_OCR_COMMAND`
3. `tools/ocr/ocrmypdf` bundled next to the installed `pdfp`
4. `ocrmypdf` from `PATH`

Check the runtime setup with:

```sh
pdfp doctor
pdfp doctor --json
```

### OCR PDF

Use `pdfp ocr` when you want a searchable PDF as its own artifact:

```sh
# Create a searchable PDF, skipping readable pages
pdfp ocr scan.pdf -o scan.searchable.pdf --mode auto --lang eng

# Force a fresh OCR layer when the existing text layer is damaged
pdfp ocr bad-text-layer.pdf -o fixed.searchable.pdf --mode force --lang eng

# Print machine-readable OCR provenance
pdfp ocr scan.pdf -o scan.searchable.pdf --json
```

`--mode auto` is the safer default. It copies readable PDFs to the requested output without requiring OCRmyPDF. `--mode force` rasterizes all pages through OCRmyPDF, so it is slower and can flatten interactive PDF features.

### Inspect and Search

```sh
# Human page/scan summary
pdfp inspect paper.pdf

# Machine-readable page metadata and scan-like signals
pdfp inspect paper.pdf --json

# Include the OCR decision/provenance in JSON
pdfp inspect scan.pdf --ocr auto --json

# Find embedded text and report matching pages
pdfp search paper.pdf "Fourier"
pdfp search paper.pdf "Fourier" --json

# Search a scan after local OCR preprocessing
pdfp search scan.pdf "invoice" --ocr auto --json
```

By default, `search` uses text already present in the PDF. Add `--ocr auto` or `--ocr force` when image-only scans or damaged text layers need a searchable OCR derivative first.

### Metadata

Use `pdfp metadata` for document information dictionary fields such as title, author, subject, keywords, creator, producer, creation date, and modification date.

```sh
# Show document information metadata
pdfp metadata show paper.pdf
pdfp metadata show paper.pdf --json

# Write a new PDF with updated fields
pdfp metadata set paper.pdf -o paper.metadata.pdf \
  --title "Revised Report" \
  --author "Engineering Team" \
  --keywords "pdf,metadata"

# Preserve the existing modification date while changing text fields
pdfp metadata set paper.pdf -o paper.titled.pdf \
  --title "Revised Report" \
  --no-touch-mod-date

# Set dates with `now`, RFC3339, or raw PDF date syntax
pdfp metadata set paper.pdf -o paper.dated.pdf \
  --creation-date 2026-05-19T12:30:00Z \
  --mod-date now

# Clear selected fields
pdfp metadata clear paper.pdf -o paper.cleaned.pdf --fields title,author
```

Metadata writes are Info-dictionary only. If a PDF also has XMP metadata, `pdfp` preserves it and reports a warning because XMP can still contain older values. PDFs that appear to contain signature fields are refused by default; use `--force-signed` only when you accept that writing a new file may invalidate signatures.

### Evaluation

Use `pdfp eval` to run the local conversion pipeline against fixture JSON files
and report formula recall, heading accuracy, and table recall:

```sh
pdfp eval tests/eval_fixtures/
```

Fixtures are documented in [`tests/eval_fixtures/README.md`](tests/eval_fixtures/README.md).
Missing local corpus PDFs are reported as skipped instead of crashing the run.

### Page Operations

```sh
# Extract pages 1-3 and 9 into a new PDF
pdfp pages extract input.pdf --pages 1-3,9 -o excerpt.pdf

# Delete selected pages and write a new PDF
pdfp pages delete input.pdf --pages 2,4-6 -o edited.pdf

# Split into chunks of 10 pages
pdfp pages split input.pdf --every 10 -o chunks/

# Reorder selected pages into a new PDF
pdfp pages reorder input.pdf --pages 1,3,2,4-8 -o reordered.pdf

# Merge PDFs in order
pdfp pages merge one.pdf two.pdf three.pdf -o merged.pdf

# Set page rotation on selected pages
pdfp pages rotate input.pdf --pages 1,3 --degrees 90 -o rotated.pdf
```

Page selections are 1-indexed and support `1`, `1-3`, comma lists, `odd`, `even`, and `all`. These commands refuse to overwrite the input path; there is no in-place editing mode yet.

### Imposition and Resize

```sh
# Two source pages per output page
pdfp impose 2up input.pdf -o two-up.pdf

# Booklet spread order, padded with blanks as needed
pdfp impose booklet input.pdf -o booklet.pdf

# Resize pages to a target paper size
pdfp page resize input.pdf --paper a4 --fit contain -o resized.pdf

# Set CropBox on selected pages (x0 y0 x1 y1, PDF points)
pdfp page crop input.pdf --pages all --box 0 0 500 700 -o cropped.pdf
```

## Output

For `paper.pdf`, default output looks like:

```text
paper/
  paper.md
  images/
    page1_img1.png
    page2_img1.png
```

The markdown uses standard headings, lists, fenced code blocks, GFM tables, and image references. By default, image links point at embedded image objects such as `![image](images/page1_img1.png)`. With `--figures snapshot`, image links point at rendered visual figure regions such as `![image](images/page1_fig1.png)`.

For review-sensitive work, start with `--conservative`. It keeps the conversion local and avoids speculative reconstruction: tables use fixed-width layout blocks, formulas are audited but not rendered as Markdown math, and rendered figure snapshots are disabled unless you explicitly run a separate inspection pass.

Table handling is local and coordinate-based for born-digital PDFs. `pdfp` combines word alignment with rendered rule-line geometry, so standards tables with explicit horizontal rules can be detected even when the text layer is not already split into cells. `--tables auto` emits Markdown tables when row/column confidence is good and falls back to fenced fixed-width `text` blocks when a region is table-like but too ambiguous. Use `--tables layout` or `--conservative` for engineering catalogues and standards where preserving visual column alignment is more important than getting a strict Markdown table. Scanned tables still need OCR first.

Formula handling is an audit and escalation path, not generic OCR. `--formulas auto` detects likely display equations from word geometry, emits high-confidence candidates as display math, and warns about candidate pages. `--debug-formulas` writes candidate JSON plus rendered equation crops under `debug/formulas/`; it also enables a conservative visual scan for isolated equation bands that are visible in the rendered page but missing from the PDF text layer. Visual-only formula regions are emitted as `formula-review` comments, not guessed LaTeX. Use `--formula-sidecar <CMD>` or `--formula-sidecar cmd:<CMD>` to send high-confidence crops to a local command such as `rapid-latex-ocr`; the command receives the crop PNG path and should print LaTeX to stdout. Builds made with `--features onnx-ocr` also accept `--formula-sidecar onnx:<model-dir>`, where the directory contains `encoder.onnx`, `decoder.onnx`, and `vocab.txt` from RapidLaTeX-OCR. Use `--conservative` when formulas must be audited without heuristic Markdown rendering. Use `--hybrid docling --formulas hybrid` when formulas matter; Docling's formula enrichment is the first recovery backend. `--formulas local` exists for inspection and should not be treated as reliable LaTeX reconstruction.

Optional native formula OCR:

```sh
mkdir -p ~/.local/share/pdfp/rapid-latex-ocr
cd ~/.local/share/pdfp/rapid-latex-ocr
wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/encoder.onnx
wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/decoder.onnx
wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/vocab.txt

cargo build --release --features onnx-ocr
target/release/pdfp convert paper.pdf --formula-sidecar onnx:$HOME/.local/share/pdfp/rapid-latex-ocr -o out/
```

Processor commands produce PDFs or JSON/human summaries. Dedicated metadata commands update document information fields, but page merge/reorder/imposition workflows still do not guarantee outlines, metadata, forms, or annotations are preserved.

## Tests

```sh
cargo test
```

See [`docs/TESTING.md`](docs/TESTING.md) for the full matrix.

## Wiki

For deeper implementation notes on PDF structure, text extraction, layout recovery, tables, OCR, Markdown rendering, and evaluation, see the [`wiki/`](wiki/README.md).

For architecture decisions and module map, see [ARCHITECTURE.md](ARCHITECTURE.md).
