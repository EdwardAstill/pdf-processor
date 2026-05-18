#!/usr/bin/env bash
set -u

ROOT="${PDFP_SIDECAR_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
CORPUS="${PDFP_SIDECAR_CORPUS:-"$ROOT/example/pdf"}"
OUT="${PDFP_SIDECAR_OUT:-"$ROOT/target/sidecar-audit"}"
BIN="${PDFP_SIDECAR_BIN:-"$ROOT/target/debug/pdfp"}"
SUMMARY="$OUT/summary.md"
BACKENDS="${PDFP_SIDECAR_BACKENDS:-native pdftotext-layout pymupdf4llm pdfplumber pdfminer camelot tabula ocrmypdf docling gmft img2table unimernet}"
FIXTURES="${PDFP_SIDECAR_FIXTURES:-math-number-theory.pdf golden__issue-336-conto-economico-bialetti.pdf golden__chinese_scan.pdf}"

slugify() {
  printf '%s' "${1%.pdf}" | sed 's#[^A-Za-z0-9._-]#__#g'
}

command_available() {
  command -v "$1" >/dev/null 2>&1
}

python_module_available() {
  local module="$1"
  python - "$module" <<'PY' >/dev/null 2>&1
import importlib.util
import sys

sys.exit(0 if importlib.util.find_spec(sys.argv[1]) else 1)
PY
}

write_summary_header() {
  mkdir -p "$OUT"
  {
    printf '# Sidecar Audit\n\n'
    printf -- '- Generated: `%s`\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
    printf -- '- Corpus: `%s`\n' "$CORPUS"
    printf -- '- Output: `%s`\n' "$OUT"
    printf -- '- Backends: `%s`\n\n' "$BACKENDS"
    printf '| Backend | Fixture | Status | Output | Note |\n'
    printf '| --- | --- | --- | --- | --- |\n'
  } > "$SUMMARY"
}

append_row() {
  local backend="$1"
  local fixture="$2"
  local status="$3"
  local output="$4"
  local note="$5"
  printf '| `%s` | `%s` | %s | `%s` | %s |\n' "$backend" "$fixture" "$status" "$output" "$note" >> "$SUMMARY"
}

ensure_native_binary() {
  if [[ ! -x "$BIN" ]]; then
    cargo build --quiet --bin pdfp --manifest-path "$ROOT/Cargo.toml"
  fi
}

run_native() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/native/$(slugify "$fixture")"
  rm -rf "$dest"
  mkdir -p "$dest"
  if "$BIN" convert "$pdf" \
    -o "$dest" \
    --debug-formulas \
    --debug-tables \
    --debug-figures \
    --figures both \
    > "$dest/stdout.txt" \
    2> "$dest/stderr.txt"; then
    append_row "native" "$fixture" "ok" "$dest" "local Rust path"
    return 0
  fi
  append_row "native" "$fixture" "failed" "$dest" "native conversion failed"
  return 1
}

run_docling() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/docling/$(slugify "$fixture")"
  local url="${PDFP_SIDECAR_DOCLING_URL:-http://localhost:5001}"
  mkdir -p "$dest"
  if ! command_available curl || ! curl -fsS --max-time 2 "$url/docs" >/dev/null 2>&1; then
    append_row "docling" "$fixture" "skipped" "$dest" "unavailable at $url"
    return 0
  fi
  if "$BIN" convert "$pdf" \
    -o "$dest" \
    --hybrid docling \
    --hybrid-url "$url" \
    --hybrid-policy all \
    --debug-formulas \
    --debug-tables \
    > "$dest/stdout.txt" \
    2> "$dest/stderr.txt"; then
    append_row "docling" "$fixture" "ok" "$dest" "docling hybrid output"
  else
    append_row "docling" "$fixture" "failed" "$dest" "docling request failed"
  fi
}

run_command_sidecar() {
  local backend="$1"
  local command_name="$2"
  local pdf="$3"
  local fixture="$4"
  local dest="$OUT/$backend/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! command_available "$command_name"; then
    append_row "$backend" "$fixture" "skipped" "$dest" "unavailable command: $command_name"
    return 0
  fi
  if "$command_name" "$pdf" "$dest" > "$dest/stdout.txt" 2> "$dest/stderr.txt"; then
    append_row "$backend" "$fixture" "ok" "$dest" "command completed"
  else
    append_row "$backend" "$fixture" "failed" "$dest" "command exited non-zero"
  fi
}

run_pdftotext_layout() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/pdftotext-layout/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! command_available pdftotext; then
    append_row "pdftotext-layout" "$fixture" "skipped" "$dest" "unavailable command: pdftotext"
    return 0
  fi
  if pdftotext -layout "$pdf" "$dest/output.txt" > "$dest/stdout.txt" 2> "$dest/stderr.txt"; then
    append_row "pdftotext-layout" "$fixture" "ok" "$dest" "Poppler layout text baseline"
  else
    append_row "pdftotext-layout" "$fixture" "failed" "$dest" "pdftotext exited non-zero"
  fi
}

run_pymupdf4llm() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/pymupdf4llm/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! python_module_available pymupdf4llm; then
    append_row "pymupdf4llm" "$fixture" "skipped" "$dest" "unavailable Python module: pymupdf4llm"
    return 0
  fi
  if python - "$pdf" "$dest" > "$dest/stdout.txt" 2> "$dest/stderr.txt" <<'PY'
from pathlib import Path
import sys

import pymupdf4llm

pdf = sys.argv[1]
dest = Path(sys.argv[2])
dest.mkdir(parents=True, exist_ok=True)
md = pymupdf4llm.to_markdown(pdf, write_images=True, image_path=str(dest / "images"))
(dest / "output.md").write_text(md, encoding="utf-8")
PY
  then
    append_row "pymupdf4llm" "$fixture" "ok" "$dest" "PyMuPDF4LLM Markdown baseline"
  else
    append_row "pymupdf4llm" "$fixture" "failed" "$dest" "pymupdf4llm conversion failed"
  fi
}

run_pdfplumber() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/pdfplumber/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! python_module_available pdfplumber; then
    append_row "pdfplumber" "$fixture" "skipped" "$dest" "unavailable Python module: pdfplumber"
    return 0
  fi
  if python - "$pdf" "$dest" > "$dest/stdout.txt" 2> "$dest/stderr.txt" <<'PY'
from pathlib import Path
import json
import sys

import pdfplumber

pdf = sys.argv[1]
dest = Path(sys.argv[2])
dest.mkdir(parents=True, exist_ok=True)
pages = []
tables = []
with pdfplumber.open(pdf) as doc:
    for index, page in enumerate(doc.pages, start=1):
        text = page.extract_text(layout=True) or ""
        pages.append(f"<!-- page:{index} -->\n{text}")
        for table in page.extract_tables() or []:
            tables.append({"page": index, "rows": table})
(dest / "output.txt").write_text("\n\n".join(pages), encoding="utf-8")
(dest / "tables.json").write_text(json.dumps(tables, indent=2), encoding="utf-8")
PY
  then
    append_row "pdfplumber" "$fixture" "ok" "$dest" "pdfplumber text/table baseline"
  else
    append_row "pdfplumber" "$fixture" "failed" "$dest" "pdfplumber extraction failed"
  fi
}

run_pdfminer() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/pdfminer/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! python_module_available pdfminer; then
    append_row "pdfminer" "$fixture" "skipped" "$dest" "unavailable Python module: pdfminer.six"
    return 0
  fi
  if python - "$pdf" "$dest" > "$dest/stdout.txt" 2> "$dest/stderr.txt" <<'PY'
from pathlib import Path
import sys

from pdfminer.high_level import extract_text

pdf = sys.argv[1]
dest = Path(sys.argv[2])
dest.mkdir(parents=True, exist_ok=True)
(dest / "output.txt").write_text(extract_text(pdf), encoding="utf-8")
PY
  then
    append_row "pdfminer" "$fixture" "ok" "$dest" "pdfminer.six text baseline"
  else
    append_row "pdfminer" "$fixture" "failed" "$dest" "pdfminer.six extraction failed"
  fi
}

run_camelot() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/camelot/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! python_module_available camelot; then
    append_row "camelot" "$fixture" "skipped" "$dest" "unavailable Python module: camelot"
    return 0
  fi
  if python - "$pdf" "$dest" > "$dest/stdout.txt" 2> "$dest/stderr.txt" <<'PY'
from pathlib import Path
import sys

import camelot

pdf = sys.argv[1]
dest = Path(sys.argv[2])
dest.mkdir(parents=True, exist_ok=True)
for flavor in ("lattice", "stream"):
    flavor_dir = dest / flavor
    flavor_dir.mkdir(exist_ok=True)
    tables = camelot.read_pdf(pdf, pages="all", flavor=flavor)
    for index, table in enumerate(tables, start=1):
        table.to_csv(str(flavor_dir / f"table_{index}.csv"))
    (dest / f"{flavor}_count.txt").write_text(str(len(tables)), encoding="utf-8")
PY
  then
    append_row "camelot" "$fixture" "ok" "$dest" "Camelot lattice/stream table baseline"
  else
    append_row "camelot" "$fixture" "failed" "$dest" "Camelot extraction failed"
  fi
}

run_tabula() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/tabula/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! python_module_available tabula; then
    append_row "tabula" "$fixture" "skipped" "$dest" "unavailable Python module: tabula-py"
    return 0
  fi
  if ! command_available java; then
    append_row "tabula" "$fixture" "skipped" "$dest" "unavailable command: java"
    return 0
  fi
  if python - "$pdf" "$dest" > "$dest/stdout.txt" 2> "$dest/stderr.txt" <<'PY'
from pathlib import Path
import sys

import tabula

pdf = sys.argv[1]
dest = Path(sys.argv[2])
dest.mkdir(parents=True, exist_ok=True)
for mode in ("lattice", "stream"):
    mode_dir = dest / mode
    mode_dir.mkdir(exist_ok=True)
    kwargs = {"pages": "all", "multiple_tables": True}
    if mode == "lattice":
        kwargs["lattice"] = True
    else:
        kwargs["stream"] = True
    tables = tabula.read_pdf(pdf, **kwargs) or []
    for index, table in enumerate(tables, start=1):
        table.to_csv(mode_dir / f"table_{index}.csv", index=False)
    (dest / f"{mode}_count.txt").write_text(str(len(tables)), encoding="utf-8")
PY
  then
    append_row "tabula" "$fixture" "ok" "$dest" "Tabula lattice/stream table baseline"
  else
    append_row "tabula" "$fixture" "failed" "$dest" "Tabula extraction failed"
  fi
}

run_ocrmypdf() {
  local pdf="$1"
  local fixture="$2"
  local dest="$OUT/ocrmypdf/$(slugify "$fixture")"
  mkdir -p "$dest"
  if ! command_available ocrmypdf; then
    append_row "ocrmypdf" "$fixture" "skipped" "$dest" "unavailable command: ocrmypdf"
    return 0
  fi
  for dependency in qpdf gs tesseract; do
    if ! command_available "$dependency"; then
      append_row "ocrmypdf" "$fixture" "skipped" "$dest" "unavailable command: $dependency"
      return 0
    fi
  done
  if ocrmypdf --skip-text "$pdf" "$dest/ocr.pdf" > "$dest/stdout.txt" 2> "$dest/stderr.txt"; then
    if "$BIN" convert "$dest/ocr.pdf" -o "$dest/pdfp" > "$dest/pdfp.stdout.txt" 2> "$dest/pdfp.stderr.txt"; then
      append_row "ocrmypdf" "$fixture" "ok" "$dest" "OCRmyPDF preprocess plus pdfp conversion"
    else
      append_row "ocrmypdf" "$fixture" "failed" "$dest" "pdfp conversion after OCR failed"
    fi
  else
    append_row "ocrmypdf" "$fixture" "failed" "$dest" "ocrmypdf exited non-zero"
  fi
}

if [[ ! -d "$CORPUS" ]]; then
  mkdir -p "$OUT"
  {
    printf '# Sidecar Audit\n\n'
    printf -- '- Status: skipped\n'
    printf -- '- Reason: missing corpus\n'
    printf -- '- Corpus: `%s`\n' "$CORPUS"
  } > "$SUMMARY"
  printf 'SKIP missing corpus\nsummary: %s\n' "$SUMMARY"
  exit 0
fi

write_summary_header
ensure_native_binary

IFS=' ' read -r -a backend_list <<< "$BACKENDS"
IFS=' ' read -r -a fixture_list <<< "$FIXTURES"

native_failures=0
for fixture in "${fixture_list[@]}"; do
  pdf="$CORPUS/$fixture"
  if [[ ! -f "$pdf" ]]; then
    for backend in "${backend_list[@]}"; do
      append_row "$backend" "$fixture" "missing" "$OUT/$backend/$(slugify "$fixture")" "fixture not present"
    done
    continue
  fi

  for backend in "${backend_list[@]}"; do
    case "$backend" in
      native)
        if ! run_native "$pdf" "$fixture"; then
          native_failures=$((native_failures + 1))
        fi
        ;;
      docling)
        run_docling "$pdf" "$fixture"
        ;;
      pdftotext-layout)
        run_pdftotext_layout "$pdf" "$fixture"
        ;;
      pymupdf4llm)
        run_pymupdf4llm "$pdf" "$fixture"
        ;;
      pdfplumber)
        run_pdfplumber "$pdf" "$fixture"
        ;;
      pdfminer)
        run_pdfminer "$pdf" "$fixture"
        ;;
      camelot)
        run_camelot "$pdf" "$fixture"
        ;;
      tabula)
        run_tabula "$pdf" "$fixture"
        ;;
      ocrmypdf)
        run_ocrmypdf "$pdf" "$fixture"
        ;;
      gmft)
        run_command_sidecar "gmft" "${PDFP_GMFT_COMMAND:-gmft}" "$pdf" "$fixture"
        ;;
      img2table)
        run_command_sidecar "img2table" "${PDFP_IMG2TABLE_COMMAND:-img2table}" "$pdf" "$fixture"
        ;;
      unimernet)
        run_command_sidecar "unimernet" "${PDFP_UNIMERNET_COMMAND:-unimernet}" "$pdf" "$fixture"
        ;;
      *)
        append_row "$backend" "$fixture" "skipped" "$OUT/$backend/$(slugify "$fixture")" "unknown backend"
        ;;
    esac
  done
done

{
  printf '\n## Notes\n\n'
  printf -- '- Optional backends are skipped unless their command or service is available.\n'
  printf -- '- External commands receive two arguments: input PDF path and output directory.\n'
  printf -- '- Deterministic comparator backends include Poppler pdftotext, PyMuPDF4LLM, pdfplumber, pdfminer.six, Camelot, Tabula, and OCRmyPDF when installed.\n'
  printf -- '- Use `PDFP_GMFT_COMMAND`, `PDFP_IMG2TABLE_COMMAND`, or `PDFP_UNIMERNET_COMMAND` for wrappers that normalize backend output.\n'
} >> "$SUMMARY"

printf 'summary: %s\n' "$SUMMARY"

if [[ "$native_failures" -gt 0 ]]; then
  exit 1
fi
