#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
DIST="${PDFP_DIST:-$ROOT/dist}"
CRATE_NAME="${PDFP_CRATE_NAME:-pdf-processor}"
TARGET="${PDFP_TARGET:-x86_64-linux}"
BIN_NAME="${PDFP_BIN_NAME:-pdfp}"
STAGE="$DIST/stage/$CRATE_NAME-$TARGET"

mkdir -p "$DIST"
rm -rf "$STAGE"
mkdir -p "$STAGE/bin" "$STAGE/tools/ocr" "$STAGE/docs"

cargo build --release --bin "$BIN_NAME" --manifest-path "$ROOT/Cargo.toml"
cp "$ROOT/target/release/$BIN_NAME" "$STAGE/bin/$BIN_NAME"
strip "$STAGE/bin/$BIN_NAME" || true

cp "$ROOT/scripts/install.sh" "$STAGE/install.sh"
cp "$ROOT/README.md" "$STAGE/README.md"
cp "$ROOT/docs/CLI.md" "$STAGE/docs/CLI.md"

if [[ -n "${PDFP_BUNDLED_OCR_DIR:-}" ]]; then
  cp -R "$PDFP_BUNDLED_OCR_DIR"/. "$STAGE/tools/ocr/"
elif command -v ocrmypdf >/dev/null 2>&1; then
  cat > "$STAGE/tools/ocr/README.md" <<'EOF'
This release was built without a hermetic OCR runtime.

The installer can install OCRmyPDF/Tesseract through the platform package
manager by default, and pdfp will also discover OCRmyPDF from PATH. Future full
bundles can place ocrmypdf, tesseract, qpdf, ghostscript, and tessdata in this
folder.
EOF
else
  cat > "$STAGE/tools/ocr/README.md" <<'EOF'
OCR tools are not bundled in this archive.

Run scripts/install.sh from the GitHub release to install OCRmyPDF/Tesseract
through a supported package manager, or place a full OCR runtime here:

  tools/ocr/ocrmypdf
  tools/ocr/tesseract
  tools/ocr/qpdf
  tools/ocr/gs
  tools/ocr/tessdata/eng.traineddata
EOF
fi

tar -C "$STAGE" -czf "$DIST/$CRATE_NAME-$TARGET.tar.gz" .
cp "$STAGE/bin/$BIN_NAME" "$DIST/$BIN_NAME"
cp "$ROOT/scripts/install.sh" "$DIST/install.sh"

ls -lh "$DIST/$CRATE_NAME-$TARGET.tar.gz" "$DIST/$BIN_NAME" "$DIST/install.sh"
