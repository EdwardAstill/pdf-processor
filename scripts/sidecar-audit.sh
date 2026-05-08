#!/usr/bin/env bash
set -u

ROOT="${PDFP_SIDECAR_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
CORPUS="${PDFP_SIDECAR_CORPUS:-"$ROOT/example/pdf"}"
OUT="${PDFP_SIDECAR_OUT:-"$ROOT/target/sidecar-audit"}"
BIN="${PDFP_SIDECAR_BIN:-"$ROOT/target/debug/pdfp"}"
SUMMARY="$OUT/summary.md"
BACKENDS="${PDFP_SIDECAR_BACKENDS:-native docling gmft img2table unimernet}"
FIXTURES="${PDFP_SIDECAR_FIXTURES:-math-number-theory.pdf golden__issue-336-conto-economico-bialetti.pdf golden__chinese_scan.pdf}"

slugify() {
  printf '%s' "${1%.pdf}" | sed 's#[^A-Za-z0-9._-]#__#g'
}

command_available() {
  command -v "$1" >/dev/null 2>&1
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
  printf -- '- Use `PDFP_GMFT_COMMAND`, `PDFP_IMG2TABLE_COMMAND`, or `PDFP_UNIMERNET_COMMAND` for wrappers that normalize backend output.\n'
} >> "$SUMMARY"

printf 'summary: %s\n' "$SUMMARY"

if [[ "$native_failures" -gt 0 ]]; then
  exit 1
fi
