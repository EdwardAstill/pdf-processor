#!/usr/bin/env bash
set -euo pipefail

REPO="${PDFP_REPO:-EdwardAstill/pdf-processor}"
VERSION="${PDFP_VERSION:-latest}"
INSTALL_ROOT="${PDFP_INSTALL_ROOT:-$HOME/.local/share/pdfp}"
BIN_DIR="${PDFP_BIN_DIR:-$HOME/.local/bin}"
INSTALL_OCR="${PDFP_INSTALL_OCR:-1}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT

detect_asset() {
  local os arch
  os="$(uname -s)"
  arch="$(uname -m)"

  case "$os:$arch" in
    Linux:x86_64|Linux:amd64)
      printf 'pdf-processor-x86_64-linux.tar.gz'
      ;;
    *)
      printf 'unsupported platform: %s %s\n' "$os" "$arch" >&2
      exit 2
      ;;
  esac
}

download_url() {
  local asset="$1"
  if [[ "$VERSION" == "latest" ]]; then
    printf 'https://github.com/%s/releases/latest/download/%s' "$REPO" "$asset"
  else
    printf 'https://github.com/%s/releases/download/%s/%s' "$REPO" "$VERSION" "$asset"
  fi
}

have() {
  command -v "$1" >/dev/null 2>&1
}

install_ocr_deps() {
  if [[ "$INSTALL_OCR" == "0" || "$INSTALL_OCR" == "false" || "$INSTALL_OCR" == "no" ]]; then
    return 0
  fi

  if have ocrmypdf && have tesseract; then
    return 0
  fi

  printf 'pdfp: installing OCR dependencies (ocrmypdf, tesseract, qpdf, ghostscript)\n'

  if have apt-get; then
    sudo apt-get update
    sudo apt-get install -y --no-install-recommends \
      ocrmypdf \
      tesseract-ocr \
      tesseract-ocr-eng \
      qpdf \
      ghostscript
  elif have pacman; then
    sudo pacman -S --needed --noconfirm \
      tesseract \
      tesseract-data-eng \
      qpdf \
      ghostscript
    if ! have ocrmypdf; then
      printf 'pdfp: OCRmyPDF is not available from the Arch pacman repositories on this system.\n' >&2
      printf 'pdfp: install the ocrmypdf AUR package manually, set PDFP_OCR_COMMAND, or continue without OCR.\n' >&2
    fi
  elif have dnf; then
    sudo dnf install -y \
      ocrmypdf \
      tesseract \
      tesseract-langpack-eng \
      qpdf \
      ghostscript
  elif have brew; then
    brew install ocrmypdf tesseract qpdf ghostscript
  else
    printf 'pdfp: no supported package manager found for OCR dependencies.\n' >&2
    printf 'pdfp: install OCRmyPDF and Tesseract manually, or use a release bundle that includes tools/ocr.\n' >&2
    return 1
  fi
}

main() {
  local asset url
  asset="$(detect_asset)"
  url="$(download_url "$asset")"

  mkdir -p "$INSTALL_ROOT" "$BIN_DIR"
  printf 'pdfp: downloading %s\n' "$url"
  curl -fsSL "$url" -o "$TMP_DIR/$asset"
  tar -xzf "$TMP_DIR/$asset" -C "$TMP_DIR"

  rm -rf "$INSTALL_ROOT/bin" "$INSTALL_ROOT/tools"
  mkdir -p "$INSTALL_ROOT"
  cp -R "$TMP_DIR/bin" "$INSTALL_ROOT/bin"
  if [[ -d "$TMP_DIR/tools" ]]; then
    cp -R "$TMP_DIR/tools" "$INSTALL_ROOT/tools"
  fi

  ln -sf "$INSTALL_ROOT/bin/pdfp" "$BIN_DIR/pdfp"

  install_ocr_deps

  printf 'pdfp: installed %s\n' "$BIN_DIR/pdfp"
  if ! "$BIN_DIR/pdfp" doctor; then
    printf 'pdfp: installed, but doctor reported a missing optional dependency.\n' >&2
  fi
}

if [[ "${PDFP_INSTALL_SCRIPT_TESTING:-0}" != "1" ]]; then
  main "$@"
fi
