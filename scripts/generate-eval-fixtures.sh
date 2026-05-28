#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET="${1:-all}"

require_command() {
	local command_name="$1"
	if ! command -v "$command_name" >/dev/null 2>&1; then
		printf 'missing required command: %s\n' "$command_name" >&2
		exit 1
	fi
}

generate_formula_corpus() {
	require_command typst

	local source_dir="$ROOT/tests/eval_fixtures/formula_corpus"
	for name in simple numbered heavy; do
		typst compile --root "$ROOT" "$source_dir/$name.typ" "$source_dir/$name.pdf"
		printf 'generated %s\n' "$source_dir/$name.pdf"
	done

	local struct_dir="$source_dir/structural"
	for name in subscripts fractions sums roots; do
		typst compile --root "$ROOT" "$struct_dir/$name.typ" "$struct_dir/$name.pdf"
		printf 'generated %s\n' "$struct_dir/$name.pdf"
	done
}

generate_stage9_hard_images() {
	require_command rsvg-convert
	require_command typst

	local source_dir="$ROOT/tests/eval_fixtures/stage9_hard_images"
	local asset_dir="$ROOT/test-corpus/eval/stage9-assets"
	local out_pdf="$ROOT/test-corpus/eval/stage9-hard-images.pdf"

	mkdir -p "$asset_dir" "$(dirname "$out_pdf")"
	rsvg-convert "$source_dir/decorative-banner.svg" -o "$asset_dir/decorative-banner.png"
	rsvg-convert "$source_dir/meaningful-chart.svg" -o "$asset_dir/meaningful-chart.png"
	typst compile --root "$ROOT" "$source_dir/stage9-hard-images.typ" "$out_pdf"
	printf 'generated %s\n' "$out_pdf"
}

case "$TARGET" in
all)
	generate_formula_corpus
	generate_stage9_hard_images
	;;
formula-corpus)
	generate_formula_corpus
	;;
stage9-hard-images)
	generate_stage9_hard_images
	;;
*)
	printf 'unknown fixture target: %s\n' "$TARGET" >&2
	printf 'available targets: all, formula-corpus, stage9-hard-images\n' >&2
	exit 1
	;;
esac
