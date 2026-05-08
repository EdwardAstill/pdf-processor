#!/usr/bin/env bash
set -u

ROOT="${PDFP_AUDIT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
CORPUS="${PDFP_AUDIT_CORPUS:-"$ROOT/example/pdf"}"
OUT="${PDFP_AUDIT_OUT:-"$ROOT/target/example-audit"}"
BIN="${PDFP_AUDIT_BIN:-"$ROOT/target/debug/pdfp"}"
QUALITY_OUT="$OUT/quality"
SUMMARY="$OUT/summary.md"

DEFAULT_FIXTURES=(
  "attention.pdf"
  "math-number-theory.pdf"
  "golden__issue-336-conto-economico-bialetti.pdf"
  "golden__chinese_scan.pdf"
)

slugify() {
  printf '%s' "${1%.pdf}" | sed 's#[^A-Za-z0-9._-]#__#g'
}

count_files() {
  local dir="$1"
  local pattern="$2"
  if [[ ! -d "$dir" ]]; then
    printf '0'
    return
  fi
  find "$dir" -maxdepth 1 -type f -name "$pattern" | wc -l | tr -d ' '
}

count_json_items() {
  local dir="$1"
  if [[ ! -d "$dir" ]]; then
    printf '0'
    return
  fi

  mapfile -t json_files < <(find "$dir" -maxdepth 1 -type f -name 'page*.json' | sort)
  if [[ "${#json_files[@]}" -eq 0 ]]; then
    printf '0'
    return
  fi

  if command -v jq >/dev/null 2>&1; then
    jq -s 'map(length) | add // 0' "${json_files[@]}" 2>/dev/null || printf 'unknown'
  else
    printf 'n/a'
  fi
}

write_skip_summary() {
  local reason="$1"
  mkdir -p "$OUT"
  {
    printf '# Example Audit\n\n'
    printf -- '- Status: skipped\n'
    printf -- '- Reason: %s\n' "$reason"
    printf -- '- Corpus: `%s`\n' "$CORPUS"
  } > "$SUMMARY"
  printf 'SKIP %s\nsummary: %s\n' "$reason" "$SUMMARY"
}

if [[ ! -d "$CORPUS" ]]; then
  write_skip_summary "missing corpus"
  exit 0
fi

mkdir -p "$OUT/debug"

if [[ ! -x "$BIN" ]]; then
  cargo build --quiet --bin pdfp --manifest-path "$ROOT/Cargo.toml"
fi

quality_status="ok"
if ! PDFP_QUALITY_ROOT="$ROOT" \
  PDFP_QUALITY_CORPUS="$CORPUS" \
  PDFP_QUALITY_RECURSIVE="${PDFP_AUDIT_RECURSIVE:-0}" \
  PDFP_QUALITY_OUT="$QUALITY_OUT" \
  PDFP_QUALITY_BIN="$BIN" \
  bash "$ROOT/scripts/quality-report.sh"; then
  quality_status="failed"
fi

IFS=' ' read -r -a requested_fixtures <<< "${PDFP_AUDIT_FIXTURES:-${DEFAULT_FIXTURES[*]}}"

case_rows=()
conversion_failures=0

for fixture in "${requested_fixtures[@]}"; do
  pdf="$CORPUS/$fixture"
  slug="$(slugify "$fixture")"
  case_out="$OUT/debug/$slug"

  if [[ ! -f "$pdf" ]]; then
    case_rows+=("| \`$fixture\` | missing | 0 | 0 | 0 | 0 | 0 | Fixture not present |")
    continue
  fi

  rm -rf "$case_out"
  mkdir -p "$case_out"

  status="ok"
  if ! "$BIN" convert "$pdf" \
    -o "$case_out" \
    --debug-formulas \
    --debug-tables \
    --debug-figures \
    --figures both \
    > "$case_out/stdout.txt" \
    2> "$case_out/stderr.txt"; then
    status="failed"
    conversion_failures=$((conversion_failures + 1))
  fi

  output_dir="$(find "$case_out" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
  formula_candidates="$(count_json_items "$output_dir/debug/formulas")"
  formula_crops="$(count_files "$output_dir/debug/formulas" '*_formula*.png')"
  table_candidates="$(count_json_items "$output_dir/debug/tables")"
  figure_candidates="$(count_json_items "$output_dir/debug/figures")"
  images="$(count_files "$output_dir/images" '*.png')"

  note="review markdown and debug JSON"
  if rg -q 'scan-heavy|OCR' "$case_out/stderr.txt" 2>/dev/null; then
    note="scan/OCR or hybrid path"
  elif [[ "$fixture" == *"conto-economico"* ]]; then
    note="financial table stress case"
  elif [[ "$formula_candidates" != "0" && "$formula_candidates" != "n/a" ]]; then
    note="formula audit stress case"
  fi

  case_rows+=("| \`$fixture\` | $status | $formula_candidates | $formula_crops | $table_candidates | $figure_candidates | $images | $note |")
done

{
  printf '# Example Audit\n\n'
  printf -- '- Generated: `%s`\n' "$(date -u +"%Y-%m-%dT%H:%M:%SZ")"
  printf -- '- Corpus: `%s`\n' "$CORPUS"
  printf -- '- Output: `%s`\n' "$OUT"
  printf -- '- Binary: `%s`\n' "$BIN"
  printf -- '- Quality status: `%s`\n\n' "$quality_status"

  printf '## Quality Summary\n\n'
  if [[ -f "$QUALITY_OUT/report.json" ]] && command -v jq >/dev/null 2>&1; then
    printf '```json\n'
    jq '.summary' "$QUALITY_OUT/report.json"
    printf '```\n\n'
    printf 'Warnings:\n\n'
    printf '```json\n'
    jq '.quality_warnings' "$QUALITY_OUT/report.json"
    printf '```\n\n'
  else
    printf 'Report: `%s`\n\n' "$QUALITY_OUT/report.json"
  fi

  printf '## Debug Cases\n\n'
  printf '| PDF | Status | Formula candidates | Formula crops | Table candidates | Figure candidates | Image files | Note |\n'
  printf '| --- | --- | ---: | ---: | ---: | ---: | ---: | --- |\n'
  for row in "${case_rows[@]}"; do
    printf '%s\n' "$row"
  done

  printf '\n## Next Triage Pass\n\n'
  printf '1. Open the Markdown and debug JSON for the highest-warning PDFs.\n'
  printf '2. Classify each issue as table, formula, figure/image, OCR/scan, or reading order.\n'
  printf '3. Change one algorithm at a time and rerun this script plus the normal cargo checks.\n'
} > "$SUMMARY"

printf 'summary: %s\n' "$SUMMARY"

if [[ "$quality_status" != "ok" || "$conversion_failures" -gt 0 ]]; then
  exit 1
fi
