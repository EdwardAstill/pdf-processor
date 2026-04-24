#!/usr/bin/env bash
set -u

ROOT="${CNV_QUALITY_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
CORPUS="${CNV_QUALITY_CORPUS:-"$ROOT/test-corpus"}"
OUT="${CNV_QUALITY_OUT:-/tmp/cnv-quality}"
BIN="${CNV_QUALITY_BIN:-"$ROOT/target/debug/cnv"}"

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

write_skip_report() {
  local reason="$1"
  mkdir -p "$OUT"
  printf '{"status":"skipped","reason":"%s","cases":[]}\n' "$(json_escape "$reason")" \
    > "$OUT/report.json"
  printf 'SKIP %s\n' "$reason"
}

if [[ ! -d "$CORPUS" ]]; then
  write_skip_report "missing corpus: $CORPUS"
  exit 0
fi

mapfile -t PDFS < <(find "$CORPUS" -type f -name '*.pdf' | sort)
if [[ "${#PDFS[@]}" -eq 0 ]]; then
  write_skip_report "no PDF fixtures under: $CORPUS"
  exit 0
fi

mkdir -p "$OUT"
rm -f "$OUT/report.json"

if [[ ! -x "$BIN" ]]; then
  cargo build --quiet --bin cnv --manifest-path "$ROOT/Cargo.toml"
fi

printf '{"status":"ok","corpus":"%s","output":"%s","cases":[\n' \
  "$(json_escape "$CORPUS")" "$(json_escape "$OUT")" > "$OUT/report.json"

first=1
passed=0
failed=0

for pdf in "${PDFS[@]}"; do
  rel="${pdf#"$CORPUS"/}"
  slug="$(printf '%s' "${rel%.pdf}" | sed 's#[^A-Za-z0-9._-]#__#g')"
  case_out="$OUT/$slug"
  stderr_path="$case_out/stderr.txt"
  rm -rf "$case_out"
  mkdir -p "$case_out"

  if "$BIN" "$pdf" -o "$case_out" > "$case_out/stdout.txt" 2> "$stderr_path"; then
    status="ok"
    passed=$((passed + 1))
  else
    status="failed"
    failed=$((failed + 1))
  fi

  md_path="$(find "$case_out" -type f -name '*.md' | head -n 1)"
  if [[ -n "$md_path" ]]; then
    pages="$(grep -c '<!-- page:' "$md_path" || true)"
    table_markers="$(grep -c '^|' "$md_path" || true)"
    headings="$(grep -c '^#' "$md_path" || true)"
    empty_pages="$(awk '
      /^<!-- page:/ {
        if (seen && nonempty == 0) empty++;
        seen = 1;
        nonempty = 0;
        next;
      }
      seen && $0 !~ /^[[:space:]]*$/ { nonempty = 1; }
      END {
        if (seen && nonempty == 0) empty++;
        print empty + 0;
      }
    ' "$md_path")"
  else
    pages=0
    table_markers=0
    headings=0
    empty_pages=0
  fi

  images="$(find "$case_out" -type f \( -name '*.png' -o -name '*.jpg' -o -name '*.jpeg' \) | wc -l | tr -d ' ')"
  warnings="$(grep -ci 'warning:' "$stderr_path" || true)"

  if [[ "$first" -eq 0 ]]; then
    printf ',\n' >> "$OUT/report.json"
  fi
  first=0

  printf '  {"pdf":"%s","status":"%s","output":"%s","pages":%s,"warnings":%s,"extracted_images":%s,"empty_pages":%s,"table_markers":%s,"heading_count":%s}' \
    "$(json_escape "$rel")" "$status" "$(json_escape "$case_out")" \
    "$pages" "$warnings" "$images" "$empty_pages" "$table_markers" "$headings" \
    >> "$OUT/report.json"
done

printf '\n],"summary":{"total":%s,"passed":%s,"failed":%s}}\n' \
  "${#PDFS[@]}" "$passed" "$failed" >> "$OUT/report.json"

printf 'quality report: %s total, %s passed, %s failed\n' "${#PDFS[@]}" "$passed" "$failed"
printf 'report: %s\n' "$OUT/report.json"

if [[ "$failed" -gt 0 ]]; then
  exit 1
fi
