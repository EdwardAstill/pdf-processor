# PDF Processor CLI Guide

`pdf-processor` is a command-line tool. The installed command is `pdfp`.

Use `pdfp --help` to see the command tree, and use `--help` on any command or subcommand to see its flags:

```sh
pdfp --help
pdfp convert --help
pdfp inspect --help
pdfp search --help
pdfp pages --help
pdfp pages extract --help
pdfp impose booklet --help
pdfp page resize --help
```

## Install

Install the prebuilt Linux binary from the latest GitHub release:

```sh
mkdir -p ~/.local/bin
gh release download --repo EdwardAstill/pdf-processor --pattern pdfp --output ~/.local/bin/pdfp --clobber
chmod +x ~/.local/bin/pdfp
```

Confirm the binary is available:

```sh
pdfp --version
pdfp --help
```

## Mental Model

`pdfp` is one CLI with several PDF workflows:

| Workflow | Command | Output |
| --- | --- | --- |
| Convert PDF to Markdown | `pdfp convert input.pdf` | Markdown folder |
| Inspect PDF pages | `pdfp inspect input.pdf` | Human summary or JSON |
| Search embedded text | `pdfp search input.pdf "needle"` | Matching pages or JSON |
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

Search only sees text embedded in the PDF. Image-only scans need OCR first; the OCR sidecar is planned separately.

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
- `--hybrid docling` requires a separate Docling server.
- Page editing commands write new PDFs and refuse to use the input path as the output path.
- Search currently uses embedded PDF text only.
- Merge/reorder/imposition preserve page contents conservatively, but document-level metadata, outlines, forms, and annotations are not yet guaranteed.
