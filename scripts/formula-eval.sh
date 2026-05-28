#!/usr/bin/env bash
set -u

ROOT="${PDFP_FORMULA_EVAL_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
PDF="${1:-${PDFP_FORMULA_EVAL_PDF:-$ROOT/example/pdf/math-number-theory.pdf}}"
OUT="${2:-${PDFP_FORMULA_EVAL_OUT:-$ROOT/target/formula-eval}}"
BIN="${PDFP_FORMULA_EVAL_BIN:-$ROOT/target/debug/pdfp}"
PROVIDERS="${PDFP_FORMULA_EVAL_PROVIDERS:-native rapid-latex-ocr docling onnx}"
SUMMARY_JSON="$OUT/summary.json"
SUMMARY_MD="$OUT/summary.md"

command_available() {
	command -v "$1" >/dev/null 2>&1
}

ensure_binary() {
	if [[ ! -x "$BIN" ]]; then
		cargo build --quiet --bin pdfp --manifest-path "$ROOT/Cargo.toml"
	fi
}

slugify() {
	printf '%s' "$1" | sed 's#[^A-Za-z0-9._-]#__#g'
}

record_skip() {
	local provider="$1"
	local dest="$2"
	local reason="$3"
	mkdir -p "$dest"
	printf '{"provider":"%s","status":"skipped","reason":"%s"}\n' "$provider" "$reason" >"$dest/result.json"
}

run_provider() {
	local provider="$1"
	local dest="$OUT/runs/$(slugify "$provider")"
	rm -rf "$dest"
	mkdir -p "$dest"

	case "$provider" in
	native)
		"$BIN" convert "$PDF" -o "$dest" --no-images --debug-formulas >"$dest/stdout.txt" 2>"$dest/stderr.txt"
		return $?
		;;
	rapid-latex-ocr | sidecar)
		local cmd="${PDFP_FORMULA_EVAL_SIDECAR_COMMAND:-rapid-latex-ocr}"
		if ! command_available "$cmd"; then
			record_skip "$provider" "$dest" "unavailable command: $cmd"
			return 0
		fi
		"$BIN" convert "$PDF" -o "$dest" --no-images --debug-formulas --formula-sidecar "$cmd" >"$dest/stdout.txt" 2>"$dest/stderr.txt"
		return $?
		;;
	docling)
		local url="${PDFP_FORMULA_EVAL_DOCLING_URL:-http://localhost:5001}"
		if ! command_available curl || ! curl -fsS --max-time 2 "$url/docs" >/dev/null 2>&1; then
			record_skip "$provider" "$dest" "unavailable at $url"
			return 0
		fi
		"$BIN" convert "$PDF" -o "$dest" --no-images --debug-formulas --hybrid docling --hybrid-url "$url" --hybrid-policy all >"$dest/stdout.txt" 2>"$dest/stderr.txt"
		return $?
		;;
	onnx)
		local model_dir="${PDFP_FORMULA_EVAL_ONNX_MODEL_DIR:-}"
		if [[ -z "$model_dir" ]]; then
			record_skip "$provider" "$dest" "PDFP_FORMULA_EVAL_ONNX_MODEL_DIR not set"
			return 0
		fi
		"$BIN" convert "$PDF" -o "$dest" --no-images --debug-formulas --formula-sidecar "onnx:$model_dir" >"$dest/stdout.txt" 2>"$dest/stderr.txt"
		return $?
		;;
	*)
		record_skip "$provider" "$dest" "unknown provider"
		return 0
		;;
	esac
}

write_summary() {
	python - "$OUT" "$PDF" "$PROVIDERS" <<'PY'
from __future__ import annotations

import json
import sys
from pathlib import Path

out = Path(sys.argv[1])
pdf = sys.argv[2]
providers = sys.argv[3].split()

def slugify(value: str) -> str:
    return ''.join(ch if ch.isalnum() or ch in '._-' else '_' for ch in value)

rows = []
for provider in providers:
    dest = out / 'runs' / slugify(provider)
    skip = dest / 'result.json'
    if skip.exists():
        data = json.loads(skip.read_text())
        rows.append({
            'provider': provider,
            'status': data.get('status', 'skipped'),
            'reason': data.get('reason', ''),
            'output': str(dest),
            'candidate_count': 0,
            'attempted': 0,
            'recovered': 0,
            'emitted': 0,
            'review_blocks': 0,
            'sidecar_failures': 0,
            'average_latency_ms': None,
        })
        continue
    index_paths = list(dest.glob('*/debug/formulas/index.json'))
    if not index_paths:
        rows.append({
            'provider': provider,
            'status': 'failed',
            'reason': 'missing debug/formulas/index.json',
            'output': str(dest),
            'candidate_count': 0,
            'attempted': 0,
            'recovered': 0,
            'emitted': 0,
            'review_blocks': 0,
            'sidecar_failures': 0,
            'average_latency_ms': None,
        })
        continue
    index = json.loads(index_paths[0].read_text())
    candidates = index.get('candidates', [])
    sidecars = [c.get('sidecar') or {} for c in candidates]
    attempted = [s for s in sidecars if s.get('status') not in (None, 'not-attempted', 'rejected-by-policy')]
    latencies = [s.get('duration_ms') for s in attempted if isinstance(s.get('duration_ms'), int)]
    failures = [s for s in attempted if s.get('status') not in ('recovered',)]
    rows.append({
        'provider': provider,
        'status': 'ok',
        'reason': '',
        'output': str(dest),
        'candidate_count': index.get('candidate_count', len(candidates)),
        'attempted': len(attempted),
        'recovered': index.get('backend_recovered_count', 0),
        'emitted': index.get('emitted_count', 0),
        'review_blocks': index.get('review_block_count', 0),
        'sidecar_failures': len(failures),
        'average_latency_ms': (sum(latencies) / len(latencies)) if latencies else None,
    })

summary = {'source_pdf': pdf, 'providers': rows}
(out / 'summary.json').write_text(json.dumps(summary, indent=2), encoding='utf-8')

lines = ['# Formula Provider Evaluation', '', f'- Source PDF: `{pdf}`', f'- Output: `{out}`', '', '| Provider | Status | Candidates | Attempted | Recovered | Emitted | Review | Failures | Avg latency | Note |', '| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |']
for row in rows:
    latency = '' if row['average_latency_ms'] is None else f"{row['average_latency_ms']:.1f} ms"
    lines.append(f"| `{row['provider']}` | {row['status']} | {row['candidate_count']} | {row['attempted']} | {row['recovered']} | {row['emitted']} | {row['review_blocks']} | {row['sidecar_failures']} | {latency} | {row['reason']} |")
(out / 'summary.md').write_text('\n'.join(lines) + '\n', encoding='utf-8')
PY
}

if [[ ! -f "$PDF" ]]; then
	mkdir -p "$OUT"
	printf '{"source_pdf":"%s","status":"skipped","reason":"missing PDF"}\n' "$PDF" >"$SUMMARY_JSON"
	printf '# Formula Provider Evaluation\n\n- Status: skipped\n- Reason: missing PDF `%s`\n' "$PDF" >"$SUMMARY_MD"
	printf 'SKIP missing PDF\nsummary: %s\n' "$SUMMARY_MD"
	exit 0
fi

rm -rf "$OUT"
mkdir -p "$OUT/runs"
ensure_binary

failures=0
IFS=' ' read -r -a provider_list <<<"$PROVIDERS"
for provider in "${provider_list[@]}"; do
	if ! run_provider "$provider"; then
		failures=$((failures + 1))
	fi
done

write_summary
printf 'summary: %s\n' "$SUMMARY_MD"

if [[ "$failures" -gt 0 ]]; then
	exit 1
fi
