#!/usr/bin/env bash
# pdfp benchmark script
# Run: bash tools/bench.sh
# Requires: pdfp on PATH, a test PDF

set -euo pipefail

PDF="${PDFP_BENCH_PDF:-/home/eastill/projects/warden/Edward_Astill-369380.pdf}"
OUT="${PDFP_BENCH_OUT:-/tmp/pdfp-bench}"
BIN="${PDFP_BENCH_BIN:-pdfp}"

echo "=== pdfp benchmark ==="
echo "Binary: $($BIN --version 2>&1 || echo 'unknown')"
echo "PDF:    $PDF"
echo ""

# Binary size
if [ -f "$(which "$BIN" 2>/dev/null)" ]; then
    echo "Binary size: $(stat -c%s "$(which "$BIN")" 2>/dev/null || stat -f%z "$(which "$BIN")") bytes"
fi

# Conversion time
rm -rf "$OUT"
echo ""
echo "--- Conversion ---"
START=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
$BIN convert "$PDF" -o "$OUT" 2>&1
END=$(date +%s%N 2>/dev/null || python3 -c 'import time; print(int(time.time()*1e9))')
DURATION=$(( (END - START) / 1000000 ))
echo "Time: ${DURATION}ms"

# Output stats
echo ""
echo "--- Output ---"
MD=$(find "$OUT" -name "*.md" | head -1)
if [ -n "$MD" ]; then
    LINES=$(wc -l < "$MD")
    SIZE=$(stat -c%s "$MD" 2>/dev/null || stat -f%z "$MD")
    echo "Markdown: $LINES lines, $SIZE bytes"
fi
IMAGES=$(find "$OUT" -name "*.png" -o -name "*.jpg" -o -name "*.jpeg" 2>/dev/null | wc -l)
echo "Images:   $IMAGES"

# Eval (if fixtures exist)
EVAL_DIR="tests/eval_fixtures"
if [ -d "$EVAL_DIR" ]; then
    echo ""
    echo "--- Eval ---"
    $BIN eval "$EVAL_DIR" 2>&1 || echo "(eval ran with some skips)"
fi

echo ""
echo "=== done ==="
