#!/usr/bin/env bash
set -euo pipefail

if [[ "$#" -ne 2 ]]; then
  printf 'usage: %s OLD_REPORT NEW_REPORT\n' "$0" >&2
  exit 2
fi

old_report="$1"
new_report="$2"

if ! command -v jq >/dev/null 2>&1; then
  printf 'quality-diff requires jq\n' >&2
  exit 2
fi

jq -n --slurpfile old_input "$old_report" --slurpfile new_input "$new_report" '
  ($old_input[0]) as $old
  | ($new_input[0]) as $new
  |
  def total($r):
    $r.summary.total // $r.summary.top_level_total // $r.case_count // ($r.cases | length);
  def passed($r):
    $r.summary.passed // $r.summary.top_level_passed // 0;
  def failed($r):
    $r.summary.failed // $r.summary.top_level_failed // 0;
  def metric($case; $name):
    $case[$name] // 0;
  def case_index($r):
    reduce ($r.cases // [])[] as $case ({}; .[$case.pdf] = $case);
  def changed_case_metrics($old_case; $new_case):
    ["pages", "warnings", "extracted_images", "empty_pages", "table_markers", "heading_count", "glued_numeric_rows"]
    | map(select(metric($old_case; .) != metric($new_case; .)))
    | map({metric: ., old: metric($old_case; .), new: metric($new_case; .)});

  {
    old_summary: {
      total: total($old),
      passed: passed($old),
      failed: failed($old),
      corpus_mode: ($old.corpus_mode // $old.summary.corpus_mode // "unknown")
    },
    new_summary: {
      total: total($new),
      passed: passed($new),
      failed: failed($new),
      corpus_mode: ($new.corpus_mode // $new.summary.corpus_mode // "unknown")
    },
    changed_cases: (
      case_index($old) as $old_cases
      | case_index($new) as $new_cases
      | (($old_cases | keys_unsorted) + ($new_cases | keys_unsorted) | unique)
      | map({
          pdf: .,
          status: (
            if ($old_cases[.] == null) then "added"
            elif ($new_cases[.] == null) then "removed"
            else "changed"
            end
          ),
          metrics: changed_case_metrics($old_cases[.] // {}; $new_cases[.] // {})
        })
      | map(select(.status != "changed" or (.metrics | length) > 0))
    )
  }
'
