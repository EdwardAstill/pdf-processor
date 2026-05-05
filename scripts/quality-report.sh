#!/usr/bin/env bash
set -u

ROOT="${PDFP_QUALITY_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
CORPUS="${PDFP_QUALITY_CORPUS:-"$ROOT/test-corpus"}"
OUT="${PDFP_QUALITY_OUT:-/tmp/pdfp-quality}"
BIN="${PDFP_QUALITY_BIN:-"$ROOT/target/debug/pdfp"}"
RECURSIVE="${PDFP_QUALITY_RECURSIVE:-1}"
HEADING_DENSITY_THRESHOLD="${PDFP_QUALITY_HEADING_DENSITY_THRESHOLD:-2.5}"
IMAGE_DENSITY_THRESHOLD="${PDFP_QUALITY_IMAGE_DENSITY_THRESHOLD:-10}"

json_escape() {
  printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g'
}

write_skip_report() {
  local reason="$1"
  mkdir -p "$OUT"
  printf '{"status":"skipped","reason":"%s","corpus_mode":"%s","case_count":0,"cases":[],"quality_warnings":[],"summary":{"total":0,"passed":0,"failed":0,"case_count":0,"corpus_mode":"%s"}}\n' \
    "$(json_escape "$reason")" "$(corpus_mode)" "$(corpus_mode)" \
    > "$OUT/report.json"
  printf 'SKIP %s\n' "$reason"
}

corpus_mode() {
  if [[ "$RECURSIVE" == "0" || "$RECURSIVE" == "false" || "$RECURSIVE" == "no" ]]; then
    printf 'top-level'
  else
    printf 'recursive'
  fi
}

if [[ ! -d "$CORPUS" ]]; then
  write_skip_report "missing corpus: $CORPUS"
  exit 0
fi

if [[ "$(corpus_mode)" == "top-level" ]]; then
  mapfile -t PDFS < <(find "$CORPUS" -maxdepth 1 -type f -name '*.pdf' | sort)
else
  mapfile -t PDFS < <(find "$CORPUS" -type f -name '*.pdf' | sort)
fi
if [[ "${#PDFS[@]}" -eq 0 ]]; then
  write_skip_report "no PDF fixtures under: $CORPUS"
  exit 0
fi

mkdir -p "$OUT"
rm -f "$OUT/report.json"
warnings_path="$OUT/quality-warnings.jsonl"
rm -f "$warnings_path"

if [[ ! -x "$BIN" ]]; then
  cargo build --quiet --bin pdfp --manifest-path "$ROOT/Cargo.toml"
fi

mode="$(corpus_mode)"
printf '{"status":"ok","corpus":"%s","corpus_mode":"%s","case_count":%s,"output":"%s","cases":[\n' \
  "$(json_escape "$CORPUS")" "$mode" "${#PDFS[@]}" "$(json_escape "$OUT")" > "$OUT/report.json"

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
    glued_numeric_rows="$(grep -Ec '[[:alpha:]][0-9]{2,3}\.[0-9]{3}' "$md_path" || true)"
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
    glued_numeric_rows=0
    empty_pages=0
  fi

  images="$(find "$case_out" -type f \( -name '*.png' -o -name '*.jpg' -o -name '*.jpeg' \) | wc -l | tr -d ' ')"
  warnings="$(grep -ci 'warning:' "$stderr_path" || true)"
  heading_density="$(awk -v headings="$headings" -v pages="$pages" 'BEGIN { if (pages > 0) printf "%.4f", headings / pages; else printf "0.0000" }')"
  images_per_page="$(awk -v images="$images" -v pages="$pages" 'BEGIN { if (pages > 0) printf "%.4f", images / pages; else printf "0.0000" }')"

  case_warnings=()
  if awk -v value="$heading_density" -v threshold="$HEADING_DENSITY_THRESHOLD" 'BEGIN { exit !(value > threshold) }'; then
    warning_json="$(printf '{"pdf":"%s","kind":"high_heading_density","metric":"heading_density","value":%s,"threshold":%s}' \
      "$(json_escape "$rel")" "$heading_density" "$HEADING_DENSITY_THRESHOLD")"
    case_warnings+=("$warning_json")
    printf '%s\n' "$warning_json" >> "$warnings_path"
  fi
  if awk -v value="$images_per_page" -v threshold="$IMAGE_DENSITY_THRESHOLD" 'BEGIN { exit !(value > threshold) }'; then
    warning_json="$(printf '{"pdf":"%s","kind":"high_image_density","metric":"images_per_page","value":%s,"threshold":%s}' \
      "$(json_escape "$rel")" "$images_per_page" "$IMAGE_DENSITY_THRESHOLD")"
    case_warnings+=("$warning_json")
    printf '%s\n' "$warning_json" >> "$warnings_path"
  fi
  if [[ "$glued_numeric_rows" -gt 0 && "$rel" == *"issue-336-conto-economico-bialetti"* ]]; then
    warning_json="$(printf '{"pdf":"%s","kind":"glued_numeric_rows","metric":"glued_numeric_rows","value":%s,"threshold":0}' \
      "$(json_escape "$rel")" "$glued_numeric_rows")"
    case_warnings+=("$warning_json")
    printf '%s\n' "$warning_json" >> "$warnings_path"
  fi

  case_warning_json="[]"
  if [[ "${#case_warnings[@]}" -gt 0 ]]; then
    case_warning_json="["
    for idx in "${!case_warnings[@]}"; do
      if [[ "$idx" -gt 0 ]]; then
        case_warning_json+=","
      fi
      case_warning_json+="${case_warnings[$idx]}"
    done
    case_warning_json+="]"
  fi

  if [[ "$first" -eq 0 ]]; then
    printf ',\n' >> "$OUT/report.json"
  fi
  first=0

  printf '  {"pdf":"%s","status":"%s","output":"%s","pages":%s,"warnings":%s,"extracted_images":%s,"images_per_page":%s,"empty_pages":%s,"table_markers":%s,"heading_count":%s,"heading_density":%s,"glued_numeric_rows":%s,"quality_warnings":%s}' \
    "$(json_escape "$rel")" "$status" "$(json_escape "$case_out")" \
    "$pages" "$warnings" "$images" "$images_per_page" "$empty_pages" "$table_markers" "$headings" "$heading_density" "$glued_numeric_rows" "$case_warning_json" \
    >> "$OUT/report.json"
done

printf '\n],"quality_warnings":[' >> "$OUT/report.json"
if [[ -s "$warnings_path" ]]; then
  first_warning=1
  while IFS= read -r warning; do
    if [[ "$first_warning" -eq 0 ]]; then
      printf ',' >> "$OUT/report.json"
    fi
    first_warning=0
    printf '\n  %s' "$warning" >> "$OUT/report.json"
  done < "$warnings_path"
fi

printf '\n],"summary":{"total":%s,"passed":%s,"failed":%s,"case_count":%s,"corpus_mode":"%s","quality_warning_count":' \
  "${#PDFS[@]}" "$passed" "$failed" "${#PDFS[@]}" "$mode" >> "$OUT/report.json"
if [[ -s "$warnings_path" ]]; then
  wc -l < "$warnings_path" | tr -d ' ' >> "$OUT/report.json"
else
  printf '0' >> "$OUT/report.json"
fi
printf '}}\n' >> "$OUT/report.json"
rm -f "$warnings_path"

printf 'quality report: %s total, %s passed, %s failed (%s)\n' "${#PDFS[@]}" "$passed" "$failed" "$mode"
printf 'report: %s\n' "$OUT/report.json"

if [[ "$failed" -gt 0 ]]; then
  exit 1
fi
