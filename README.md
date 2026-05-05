# pdf-processor

`pdf-processor` is a local PDF processor. Its most mature workflow is converting PDFs into AI-friendly markdown, and the same binary now also has inspection, search, page editing, imposition, and resizing commands.

The active codepath is:

1. Open PDF with MuPDF.
2. Extract text blocks and images.
3. Reconstruct reading order with XY-Cut++.
4. Classify blocks into headings, lists, tables, captions, and paragraphs.
5. Write markdown plus extracted images.

## Scope

Current top-level scope:

- PDF input only
- Markdown output for conversion
- PDF output for page operations, imposition, and resizing
- Inspect/search operations over embedded PDF text
- Safe page operations that write new PDFs instead of editing inputs in place
- Prototype page layout operations: 2-up, booklet, and page resize
- Optional hybrid Docling assist for hard pages
- Local-first processing

Out of active scope at the repo root:

- DOCX / EPUB / PPTX / HTML conversion
- Markdown to Typst
- SVG to PNG
- RAG / KG / wiki / JSON export modes

## Install

Install the prebuilt Linux binary from the latest GitHub release:

```sh
mkdir -p ~/.local/bin
gh release download --repo EdwardAstill/pdf-processor --pattern pdfp --output ~/.local/bin/pdfp --clobber
chmod +x ~/.local/bin/pdfp
```

To build from source, install `clang`/`libclang` and run:

```sh
cargo build --release --bin pdfp
```

## Usage

`pdf-processor` is a CLI tool. The installed command is `pdfp`.

```sh
pdfp <INPUT> [OPTIONS]
pdfp convert <INPUT> [OPTIONS]
pdfp inspect <INPUT> [--json]
pdfp search <INPUT> <TEXT> [--json]
pdfp pages <extract|delete|split|reorder|merge> ...
pdfp impose <2up|booklet> ...
pdfp page resize <INPUT> -o <OUTPUT>
```

Bare `pdfp <INPUT>` remains a backwards-compatible alias for `pdfp convert <INPUT>`.

Every command and nested command has help:

```sh
pdfp --help
pdfp convert --help
pdfp pages extract --help
pdfp impose booklet --help
pdfp page resize --help
```

See the full CLI guide at [`docs/CLI.md`](docs/CLI.md).

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

# Hybrid assist for harder pages
pdfp convert math-paper.pdf --hybrid docling -o out/

# Hybrid assist with cached OCR/table output
pdfp convert scan.pdf --hybrid docling --hybrid-cache-dir .pdfp-cache -o out/
```

### Inspect and Search

```sh
# Human page/scan summary
pdfp inspect paper.pdf

# Machine-readable page metadata and scan-like signals
pdfp inspect paper.pdf --json

# Find embedded text and report matching pages
pdfp search paper.pdf "Fourier"
pdfp search paper.pdf "Fourier" --json
```

`search` uses text already present in the PDF. Image-only scans need the planned OCR sidecar before they become searchable.

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

The markdown uses standard headings, lists, fenced code blocks, GFM tables, and image references like `![image](images/page1_img1.png)`.

Processor commands produce PDFs or JSON/human summaries. They currently preserve page contents conservatively, but outlines, document-level metadata, forms, and annotations are not yet guaranteed across merge/reorder/imposition workflows.

## Tests

```sh
cargo test
```

See [`docs/TESTING.md`](docs/TESTING.md) for the full matrix.

## Wiki

For deeper implementation notes on PDF structure, text extraction, layout recovery, tables, OCR, Markdown rendering, and evaluation, see the [`wiki/`](wiki/README.md).
