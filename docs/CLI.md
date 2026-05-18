# PDF Processor CLI Guide

`pdf-processor` is a command-line tool. The installed command is `pdfp`.

Use `pdfp --help` to see the command tree, and use `--help` on any command or subcommand to see its flags:

```sh
pdfp --help
pdfp convert --help
pdfp ocr --help
pdfp doctor --help
pdfp inspect --help
pdfp search --help
pdfp eval --help
pdfp pages --help
pdfp pages extract --help
pdfp impose booklet --help
pdfp page resize --help
```

## Install

Install the latest Linux release with one command:

```sh
curl -fsSL https://github.com/EdwardAstill/pdf-processor/releases/latest/download/install.sh | sh
```

The installer places `pdfp` under `~/.local/share/pdfp`, symlinks it into `~/.local/bin`, and installs OCR dependencies through the platform package manager when they are missing. Set `PDFP_INSTALL_OCR=0` to skip OCR dependency installation.

Confirm the binary is available:

```sh
pdfp --version
pdfp --help
pdfp doctor
```

## Mental Model

`pdfp` is one CLI with several PDF workflows:

| Workflow | Command | Output |
| --- | --- | --- |
| Convert PDF to Markdown | `pdfp convert input.pdf` | Markdown folder |
| Create a searchable OCR PDF | `pdfp ocr input.pdf -o output.pdf` | New searchable PDF |
| Check runtime dependencies | `pdfp doctor` | Human summary or JSON |
| Inspect PDF pages | `pdfp inspect input.pdf` | Human summary or JSON, optionally OCR-assisted |
| Search embedded text | `pdfp search input.pdf "needle"` | Matching pages or JSON, optionally OCR-assisted |
| Evaluate extraction quality | `pdfp eval fixtures/` | Formula, heading, table, and image metrics |
| Extract/delete/split/reorder/merge pages | `pdfp pages ...` | New PDF files |
| Create 2-up or booklet layouts | `pdfp impose ...` | New PDF files |
| Resize pages | `pdfp page resize ...` | New PDF file |

The old shorthand still works:

```sh
pdfp input.pdf -o out/
```

That is the same as:

```sh
pdfp convert input.pdf -o out/
```

## Convert to Markdown

Convert one PDF:

```sh
pdfp convert paper.pdf -o out/
```

Convert every PDF in a directory:

```sh
pdfp convert papers/ -o out/ --verbose
```

Convert a shell glob. Quote the glob so `pdfp` can resolve it consistently:

```sh
pdfp convert "papers/*.pdf" -o out/
```

Useful conversion flags:

| Flag | Meaning |
| --- | --- |
| `-o`, `--output <DIR>` | Output directory |
| `--no-images` | Skip extracted image files |
| `--conservative` | Prefer review-safe fallbacks over heuristic reconstruction |
| `--figures embedded|snapshot|both|none` | Choose embedded image objects, rendered figure snapshots, both, or no image output |
| `--figure-dpi <N>` | Snapshot render resolution, default `200` |
| `--figure-padding <PTS>` | Padding around detected figure regions, default `8.0` |
| `--debug-figures` | Write figure candidate JSON under `debug/figures/` |
| `--tables auto|native|layout|off` | Choose coordinate table handling |
| `--debug-tables` | Write table candidate JSON under `debug/tables/` |
| `--formulas auto|local|hybrid|off` | Detect, audit, or route formula candidates |
| `--debug-formulas` | Write formula candidate JSON and crops under `debug/formulas/` |
| `--formula-sidecar <SIDECAR>` | Run optional formula OCR on high-confidence crops; accepts commands, `cmd:<command>`, or `onnx:<model-dir>` in `onnx-ocr` builds |
| `--ocr off|auto|force` | Run optional local OCR preprocessing |
| `--ocr-lang <LANGS>` | OCR languages, such as `eng` or `eng+deu` |
| `--ocr-cache-dir <DIR>` | Cache searchable OCR derivative PDFs |
| `--ocr-timeout-secs <N>` | OCR command timeout |
| `--ocr-command <PATH>` | OCRmyPDF command path, defaults to `ocrmypdf` |
| `--min-h-gap <PTS>` | Tune horizontal layout cuts |
| `--min-v-gap <PTS>` | Tune vertical layout cuts |
| `--hybrid docling` | Use a running Docling backend for harder pages |
| `--hybrid-policy auto|all` | Route selected pages or every page to the backend |
| `--hybrid-cache-dir <DIR>` | Cache hybrid Markdown by PDF/page |
| `-v`, `--verbose` | Print progress to stderr |

Default output for `paper.pdf`:

```text
paper/
  paper.md
  images/
    page1_img1.png
```

Figure modes:

```sh
# Current default: extract raster image objects embedded in the PDF.
pdfp convert paper.pdf -o out/ --figures embedded

# Render the complete detected visual figure region from the page.
pdfp convert paper.pdf -o out/ --figures snapshot --figure-dpi 200

# Debug mode: keep both asset styles and write candidate metadata.
pdfp convert paper.pdf -o out/ --figures both --debug-figures

# Text-only markdown output.
pdfp convert paper.pdf -o out/ --figures none
```

`embedded` mode is fast and preserves the current `images/pageN_imgM.png` behavior, but it only sees raster image objects. `snapshot` mode renders detected page regions to `images/pageN_figM.png`, so it can include vector graphics, labels, legends, axes, and multi-panel figures that are not stored as one embedded image. Snapshot detection is heuristic; use `--debug-figures` when tuning false positives or missed figures. Higher `--figure-dpi` values produce sharper images but increase runtime and output size.

Conservative mode:

```sh
# Review-safe first pass: preserve ambiguous regions instead of guessing.
pdfp convert standard.pdf -o out/ --conservative --debug-tables --debug-formulas
```

`--conservative` is a preset for standards and other review-sensitive PDFs. It overrides conversion modes to avoid speculative reconstruction:

- figures: embedded assets only, no rendered snapshot candidates
- tables: fixed-width `layout` blocks instead of inferred Markdown tables
- formulas: audit mode, no local heuristic `$$` rendering

Use this mode when omissions or wrong reconstructions are more costly than having a review marker or a visually preserved fallback.

Table modes:

```sh
# Default: native Markdown tables when confident, fixed-width fallback otherwise.
pdfp convert catalogue.pdf -o out/ --tables auto

# Force coordinate-derived Markdown tables.
pdfp convert catalogue.pdf -o out/ --tables native

# Preserve detected table regions as fenced fixed-width text.
pdfp convert catalogue.pdf -o out/ --tables layout

# Disable coordinate table reconstruction.
pdfp convert catalogue.pdf -o out/ --tables off
```

`pdfp` reconstructs born-digital tables from MuPDF word coordinates plus rendered rule-line geometry. This works best when the PDF already has a usable text layer, such as product catalogues and standards with selectable text. `native` mode creates GFM tables from inferred rows and columns. `layout` mode writes a fenced `text` block with visual column spacing, which is safer for very wide engineering tables or multi-row headers. `--debug-tables` writes `table_region` bboxes, rows, confidence, render mode, and table evidence under `debug/tables/`.

OCR is a separate concern. If the page is a scan with no usable text layer, use `--ocr auto` or `--ocr force` before expecting table reconstruction to work.

Formula modes:

```sh
# Default: detect formula candidates and render high-confidence candidates as display math.
pdfp convert standard.pdf -o out/ --formulas auto

# Force local formula candidate rendering for inspection.
pdfp convert standard.pdf -o out/ --formulas local --debug-formulas

# Recover high-confidence formula crops with a local command.
pdfp convert standard.pdf -o out/ --formula-sidecar rapid-latex-ocr --debug-formulas

# Recover formula crops with native ONNX OCR in an onnx-ocr build.
pdfp convert standard.pdf -o out/ --formula-sidecar onnx:$HOME/.local/share/pdfp/rapid-latex-ocr --debug-formulas

# Use formula candidates to route pages through Docling formula enrichment.
pdfp convert standard.pdf -o out/ --hybrid docling --formulas hybrid

# Disable formula detection.
pdfp convert standard.pdf -o out/ --formulas off
```

PDF formulas are not stored as formulas. They are glyphs, positions, font encodings, and sometimes vector drawings. Auto mode detects likely display-equation regions, emits high-confidence candidates as display math, and writes a formula coverage ledger when `--debug-formulas` is enabled. `--debug-formulas` also runs a page-render visual scan for isolated equation bands near formula cues such as `Hence:` and `where:`. Visual-only regions get `pageN_formulaM.png` crops and Markdown `formula-review` comments rather than guessed LaTeX. `--formula-sidecar <CMD>` or `--formula-sidecar cmd:<CMD>` sends high-confidence crops to a local command such as `rapid-latex-ocr`; the command receives the crop PNG path and should print LaTeX to stdout. Builds made with `--features onnx-ocr` also accept `--formula-sidecar onnx:<model-dir>`, where the model directory contains `encoder.onnx`, `decoder.onnx`, and `vocab.txt` from RapidLaTeX-OCR. `--formulas local` is available for inspection and renders all text-backed local candidates, but it does not guarantee perfect LaTeX. Use `--conservative` for standard-processing review when heuristic math rendering is too risky. For recovery through Docling, run a backend and use `--hybrid docling --formulas hybrid`.

To prepare native ONNX OCR:

```sh
mkdir -p ~/.local/share/pdfp/rapid-latex-ocr
cd ~/.local/share/pdfp/rapid-latex-ocr
wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/encoder.onnx
wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/decoder.onnx
wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/vocab.txt
cargo build --release --features onnx-ocr
```

## Local OCR

OCR is opt-in. It never edits the input PDF in place. When OCR is needed, `pdfp` writes a searchable derivative PDF to a temporary or cache directory, then runs the normal conversion, inspection, or search path against that derivative while keeping output names based on the original file.

Use automatic OCR only when scan triage says it is needed:

```sh
pdfp convert scan.pdf --ocr auto --ocr-lang eng --ocr-cache-dir .pdfp-ocr -o out/
```

Force OCR when the embedded text layer exists but is damaged:

```sh
pdfp convert bad-text-layer.pdf --ocr force --ocr-lang eng+deu -o out/
```

Inspect the OCR decision:

```sh
pdfp inspect scan.pdf --ocr auto --json
```

Search a scan through the OCR sidecar:

```sh
pdfp search scan.pdf "needle" --ocr auto --json
```

Local OCR calls OCRmyPDF, so the machine also needs Tesseract and any requested language packs. If `--ocr auto` is used on a clean born-digital PDF, OCR is skipped and missing OCR tools do not matter. If a scan-heavy PDF needs OCR and the command is missing, `pdfp` exits with a message naming the missing OCRmyPDF command.

`pdfp` resolves OCRmyPDF in this order:

1. `--command <PATH>` / `--ocr-command <PATH>`
2. `PDFP_OCR_COMMAND`
3. `tools/ocr/ocrmypdf` bundled next to the installed `pdfp`
4. `ocrmypdf` from `PATH`

Check the runtime setup:

```sh
pdfp doctor
pdfp doctor --json
```

### Standalone OCR PDF

Use `pdfp ocr` when the desired output is a searchable PDF rather than Markdown:

```sh
pdfp ocr scan.pdf -o scan.searchable.pdf --mode auto --lang eng
```

Useful standalone OCR flags:

| Flag | Meaning |
| --- | --- |
| `-o`, `--output <PDF>` | Output searchable PDF |
| `--mode auto|force` | Skip readable PDFs/pages or force raster OCR |
| `--lang <LANGS>` | OCR languages, such as `eng`, `eng+deu`, or `chi_sim` |
| `--cache-dir <DIR>` | Reuse OCRmyPDF sidecar results before copying to output |
| `--timeout-secs <N>` | OCR timeout |
| `--command <PATH>` | OCRmyPDF command path |
| `--json` | Emit OCR decision/provenance JSON |

`--mode auto` first checks whether the PDF already has readable text. If OCR is not needed, `pdfp` copies the input to the output path and reports `status: skipped` in JSON. `--mode force` sends every page through OCRmyPDF and is the right option only when the embedded text layer is damaged.

## Inspect PDFs

Print a readable summary:

```sh
pdfp inspect input.pdf
```

Print JSON for scripts:

```sh
pdfp inspect input.pdf --json
```

Use this before OCR or page editing to answer:

- How many pages does this PDF have?
- What are the page sizes?
- Does it look scan-heavy?
- Which pages have readable embedded text?

## Search Embedded Text

Search all pages:

```sh
pdfp search input.pdf "Fourier"
```

Search selected pages:

```sh
pdfp search input.pdf "Fourier" --pages 1-10
```

Return JSON:

```sh
pdfp search input.pdf "Fourier" --json
```

By default, search only sees text embedded in the PDF. Add `--ocr auto` to let scan-heavy PDFs be converted to a searchable derivative first, or `--ocr force` when an existing text layer is unusable.

## Evaluate Quality

Run the local conversion pipeline against fixture JSON files and report recall,
precision, false-positive counts, and image/figure retention:

```sh
pdfp eval tests/eval_fixtures/
```

Each fixture JSON names a PDF and the expected formula count, headings, table
presence/regions, and image/figure expectations for selected pages. Missing PDFs
are skipped with a clear message, which keeps large local corpora out of the
repository while preserving the evaluation contract. See
`tests/eval_fixtures/README.md` for the schema and Stage 8/9 benchmark notes.

## Page Operations

Page ranges are 1-indexed. Supported forms:

| Selection | Meaning |
| --- | --- |
| `1` | Page 1 |
| `1-3` | Pages 1 through 3 |
| `1,3,5-7` | Pages 1, 3, 5, 6, 7 |
| `odd` | Odd-numbered pages |
| `even` | Even-numbered pages |
| `all` | All pages |

Extract pages:

```sh
pdfp pages extract input.pdf --pages 1-3,9 -o excerpt.pdf
```

Delete pages:

```sh
pdfp pages delete input.pdf --pages 2,4-6 -o edited.pdf
```

Split into chunks:

```sh
pdfp pages split input.pdf --every 10 -o chunks/
```

Reorder pages:

```sh
pdfp pages reorder input.pdf --pages 1,3,2,4-8 -o reordered.pdf
```

Merge PDFs:

```sh
pdfp pages merge one.pdf two.pdf three.pdf -o merged.pdf
```

These commands write new files. They do not edit the input PDF in place.

## Imposition and Resizing

Put two source pages onto each output page:

```sh
pdfp impose 2up input.pdf -o two-up.pdf
```

Create booklet spread order:

```sh
pdfp impose booklet input.pdf -o booklet.pdf
```

Resize pages to A4:

```sh
pdfp page resize input.pdf --paper a4 --fit contain -o resized.pdf
```

Supported resize options:

| Flag | Values |
| --- | --- |
| `--paper` | `a4`, `letter` |
| `--fit` | `contain`, `cover`, `stretch` |

## Safety and Limits

- `pdfp` is local-first. Normal conversion, search, inspection, and page operations do not require a network service.
- `--ocr auto` and `--ocr force` require OCRmyPDF plus Tesseract only when OCR is actually run.
- `--hybrid docling` requires a separate Docling server.
- Page editing commands write new PDFs and refuse to use the input path as the output path.
- Search uses embedded PDF text unless local OCR is explicitly requested.
- Merge/reorder/imposition preserve page contents conservatively, but document-level metadata, outlines, forms, and annotations are not yet guaranteed.
