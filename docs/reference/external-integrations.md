# External Integration (`src/ocr/`, `src/hybrid/`)

Two optional integration paths: local OCR preprocessing for scanned PDFs, and Docling hybrid enrichment for pages the local pipeline can't handle well.

---

## OCR Preprocessing (`src/ocr/`)

Runs OCRmyPDF/Tesseract *before* the main pipeline to add a searchable text layer to scanned or damaged-text PDFs.

### Source files

| File | Purpose |
|---|---|
| `mod.rs` | OCRmyPDF integration: command resolution, PDF preparation, decision logic |
| `triage.rs` | Scan detection: decides whether a PDF needs OCR |

### Key types

| Type | Purpose |
|---|---|
| `PreparedPdf` | Output: `effective_path` (original or OCR'd) and `decision` metadata |
| `OcrDecision` | Structured record: `mode`, `status` (Off/Skipped/CacheHit/Ran), `provider`, `command`, `pages_needing_ocr`, `reason` |
| `OcrRuntimeStatus` | Doctor output: `available`, `command`, `source`, `searched` paths, `hint` |
| `OcrCommandResolution` | Resolved command path and source (Explicit/Env/Bundled/Path) |

### Key functions

| Function | Description |
|---|---|
| `prepare_pdf(input_path, options) -> Result<PreparedPdf>` | Main entry. Decides whether to OCR based on mode and triage, runs ocrmypdf if needed |
| `resolve_ocr_command(options) -> Result<OcrCommandResolution>` | Resolves path from: `--command` / `PDFP_OCR_COMMAND` / bundled `tools/ocr/ocrmypdf` / `PATH` |
| `probe_ocr_runtime() -> OcrRuntimeStatus` | Doctor check: scans well-known locations for OCRmyPDF |
| `page_needs_ocr(page) -> bool` | Triage: no extractable text, image-only, or high replacement-char density |
| `compute_cache_key(source_path, lang) -> String` | Hash-based cache key for reusable OCR derivatives |

### CLI flags

| Flag | Effect |
|---|---|
| `--ocr off\|auto\|force` | OCR mode |
| `--lang <LANGS>` | Tesseract language (e.g. `eng+deu`) |
| `--ocr-cache-dir <DIR>` | Cache searchable OCR PDFs |
| `--ocr-timeout-secs <N>` | ocrmypdf timeout (default 600) |
| `--ocr-command <PATH>` | Explicit ocrmypdf path |
| `--command <PATH>` | Alias for `--ocr-command` |

Command resolution order: `--ocr-command` → `PDFP_OCR_COMMAND` → bundled `tools/ocr/ocrmypdf` → `ocrmypdf` on `PATH`.

### Dependencies

- **OCRmyPDF** — Python tool adding a searchable text layer. **Tesseract** — OCR engine with per-language packs. Neither is bundled in the binary. The installer script can install them via the platform package manager.

---

## Hybrid Backend (`src/hybrid/`)

Routes pages through an external Docling service *after* the local pipeline runs, replacing `page.override_markdown` on routed pages.

### Source files

| File | Purpose |
|---|---|
| `mod.rs` | Orchestration: `apply_to_document()`, `RoutingPolicy`, `HybridStats` |
| `client.rs` | HTTP client via `reqwest::blocking` with `rustls-tls` |
| `triage.rs` | Per-page routing: `should_route(page)` based on formula/table/math density |
| `page_extract.rs` | Single-page PDF extraction for upload |

### Key types

| Type | Purpose |
|---|---|
| `RoutingPolicy` | `Auto` (per-page triage) or `All` (every page) |
| `HybridStats` | Counts: `pages_total`, `pages_routed`, `pages_failed`, `pages_cached` |

### Key functions

| Function | Description |
|---|---|
| `apply_to_document(doc, pdf_path, policy, url, timeout, cache_dir, verbose)` | Iterates pages, routes qualifying ones through Docling, caches per-page results |

Per-page flow: triage → cache check → extract single-page PDF → POST to Docling → store `override_markdown`. Failures log to stderr and continue.

| Function | Description |
|---|---|
| `should_route(page) -> bool` | Routes when: formula candidates present, high math-symbol density, tables detected, or low readable text density |
| `DoclingClient::new(url, timeout)` | Creates HTTP client for a Docling-serve instance |
| `client.convert_bytes_to_markdown(pdf_bytes) -> Result<String>` | POSTs single-page PDF to `/v1/convert/file` |
| `extract_page_as_pdf_bytes(pdf_path, page_num) -> Result<Vec<u8>>` | Extracts one page via MuPDF's `save_page_to_bytes()` |

### CLI flags

| Flag | Effect |
|---|---|
| `--hybrid docling` | Enable hybrid enrichment |
| `--hybrid-url <URL>` | Docling backend URL (default `http://localhost:5001`) |
| `--hybrid-timeout-secs <N>` | Backend timeout (default 600) |
| `--hybrid-policy auto\|all` | Page routing policy |
| `--hybrid-cache-dir <DIR>` | Per-page markdown cache |

### Dependencies

- **reqwest** with `rustls-tls`. **Docling-serve** — external Python service (MIT), not bundled.

---

## Cross-references

- `wiki/algorithms/ocr.md`, `wiki/topics/scans-and-ocr.md` — OCR pipeline
- `wiki/tools/frameworks.md` — Docling architecture
- [pipeline.md](pipeline.md) — OCR runs before, hybrid runs after the local pipeline
- [pdf-extraction.md](pdf-extraction.md) — where OCR-prepared PDFs feed in
