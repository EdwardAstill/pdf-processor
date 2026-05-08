# Measurable PDF Quality Improvements Plan

Status: draft
Date: 2026-05-08
Repo: `/home/eastill/projects/pdf-processor`

## Goal

Reduce the current example-audit warning count with changes that have a clear before/after metric. The first target is the Bialetti financial statement because it is the only current `glued_numeric_rows` warning and has a direct pass/fail target: `2 -> 0`.

## Research Summary

Current command:

```bash
bash scripts/example-audit.sh
```

Current measurable state:

- Top-level examples: `22/22` pass.
- Quality warning count: `7`.
- Bialetti warning: `glued_numeric_rows = 2`.
- Bialetti debug formula candidates: `9`, all are financial rows misread as formulas.
- Bialetti table candidates: `3`, but all are low-confidence `Layout` tables with confidence around `0.48`, `0.52`, and `0.61`.

Observed Bialetti failures in generated Markdown:

```md
1. 16) Altri proventi finanziari57.000461.0001.466.000635.000
Altri proventi finanziari57.000461.0001.466.000635.000
1. 17) Interessi e altri oneri finanziari3.027.0002.996.0002.949.0002.973.000
```

The metric in `scripts/quality-report.sh` is:

```bash
grep -Ec '[[:alpha:]][0-9]{2,3}\.[0-9]{3}'
```

So the direct measurable goal is to eliminate label+number glue in the Bialetti Markdown, not merely increase generic table candidates.

Local code findings:

- `src/render/markdown.rs::try_render_implicit_numeric_table` only emits a Markdown table when it sees at least three consecutive numeric rows with the same dominant value count. The problematic Bialetti rows are isolated or separated by label-only rows, so they escape this path.
- `src/layout/table.rs` already has word-geometry table detection, but the financial islands around lines `16)` and `17)` are not absorbed into the larger table candidates.
- `src/pipeline.rs::suppress_formula_candidates_overlapping_tables` currently suppresses formulas only inside table candidates with confidence `>= 0.70`; the Bialetti layout table candidates are below that threshold.
- Existing `.warden/plans/2026-05-05-local-ocr-and-quality-plan.md` already recommended moving table/form recovery upstream instead of adding more renderer-only financial hacks.

## Best Approach

Do the work in this order:

1. Build a focused Bialetti red test and metric gate.
2. Add a financial-row repair pass using word geometry, not string-only cleanup.
3. Broaden formula suppression for numeric layout-table regions.
4. Rerun the full audit and only then decide whether to attack heading-density or image-density warnings.

This should produce the first measurable corpus improvement: `quality_warning_count 7 -> 6`.

## Assumptions

| id | statement | type | source | check | if false | owner |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | Bialetti remains present under `example/pdf/`. | repo-state | current audit | `test -f example/pdf/golden__issue-336-conto-economico-bialetti.pdf` | Use ignored `test-corpus` fixture or skip Bialetti-specific gate. | Task 1 |
| A2 | The Bialetti metric remains `glued_numeric_rows` in `scripts/quality-report.sh`. | repo-state | local script read | `rg -n 'glued_numeric_rows' scripts/quality-report.sh` | Update acceptance metric to match script. | Task 1 |
| A3 | Financial row repair should use `RawWord` geometry and table candidates before Markdown rendering. | architectural | local code + prior plan | `rg -n 'RawWord|detect_coordinate_tables|CoordinateTable' src/layout/table.rs src/pipeline.rs` | If raw geometry is insufficient, fall back to renderer parser with a narrow regression test. | Task 2 |
| A4 | GPL/AGPL sidecars should remain benchmark-only. | policy | repo MIT license + research report | `rg -n '^license = "MIT"' Cargo.toml` | Keep sidecars outside default build. | Task 7 |

## Tasks

### 1. Add Bialetti Metric Gate

Block: execute
Skill: `test-driven-development` + `rust`

Instruction:

Add a focused integration test that runs the Bialetti example through `pdfp`, reads the Markdown, and asserts no label+number glue remains. Also assert the audit JSON reports `glued_numeric_rows: 0` when run for only that fixture.

Acceptance:

- `cargo test --test quality bialetti_glued_numeric_rows_are_zero -- --nocapture` -> fails before the implementation, then passes.
- `PDFP_AUDIT_FIXTURES=golden__issue-336-conto-economico-bialetti.pdf bash scripts/example-audit.sh` -> exits 0.
- `jq '.cases[] | select(.pdf=="golden__issue-336-conto-economico-bialetti.pdf") | .glued_numeric_rows' target/example-audit/quality/report.json` -> prints `0`.

### 2. Add Financial Row Island Detection

Block: execute
Skill: `rust`

Instruction:

In `src/layout/table.rs`, add a financial-row island detector that finds rows with a text label followed by 3-5 money-like values, even when the rows are too isolated for the existing table-run detector. Learn numeric column anchors from nearby table candidates or from page-wide money rows, then create low-risk `TableRender::Layout` table candidates that suppress the source text blocks.

Acceptance:

- `cargo test layout::table::tests::detects_financial_row_islands -- --nocapture` -> passes.
- `cargo test layout::table -- --nocapture` -> passes.
- `PDFP_AUDIT_FIXTURES=golden__issue-336-conto-economico-bialetti.pdf bash scripts/example-audit.sh` -> Bialetti `glued_numeric_rows` is `0`.

### 3. Repair Glued Numeric Rows Without String-Only Guessing

Block: execute
Skill: `rust`

Instruction:

If a financial row arrives as one text block with glued values, reconstruct it from `RawWord` positions where possible. Only use a renderer-level split as a fallback, and gate it behind a strict pattern: a label followed by 3-5 money-like values with no alphabetic characters in the numeric suffix.

Acceptance:

- `cargo test render::markdown::tests::repairs_glued_financial_numeric_suffix -- --nocapture` -> passes only if fallback is needed.
- `rg -n '[[:alpha:]][0-9]{2,3}\\.[0-9]{3}' target/example-audit/debug/golden__issue-336-conto-economico-bialetti/**/*.md` -> exits non-zero after the audit run.
- `cargo test --test golden -- --ignored golden_snapshot_financial_statement_structure` -> passes or skips only because ignored fixture is absent.

### 4. Broaden Financial Formula Suppression

Block: execute
Skill: `rust`

Instruction:

In formula detection/routing, suppress rows that are table-like financial rows: 3-5 money-like values, no equation number, no relation-heavy formula structure, and either overlap with any coordinate table candidate or share page-wide financial column anchors. Do not suppress real formulas in `math-number-theory.pdf`.

Acceptance:

- `cargo test formula::detect::tests::ignores_financial_statement_rows -- --nocapture` -> passes.
- `cargo test --test formulas -- --nocapture` -> passes.
- `PDFP_AUDIT_FIXTURES="golden__issue-336-conto-economico-bialetti.pdf math-number-theory.pdf" bash scripts/example-audit.sh` -> Bialetti formula candidates are `0` or `1`, while `math-number-theory.pdf` still has formula candidates and crops.

### 5. Rerun Example Audit And Lock The Improvement

Block: review
Skill: `verification-before-completion`

Instruction:

Run the current top-level audit and record exact before/after metrics in `docs/QUALITY_LOOP.md`.

Acceptance:

- `bash scripts/example-audit.sh` -> `22 passed, 0 failed`.
- `jq '.summary.quality_warning_count' target/example-audit/quality/report.json` -> prints `6` or lower.
- `jq '.cases[] | select(.pdf=="golden__issue-336-conto-economico-bialetti.pdf") | .glued_numeric_rows' target/example-audit/quality/report.json` -> prints `0`.
- `cargo fmt -- --check` -> passes.
- `cargo clippy --all-targets -- -D warnings` -> passes.
- `cargo test` -> passes, with only existing ignored tests ignored.

### 6. Heading-Density Warning Design Pass

Block: research
Skill: `codebase-explainer` + `rust`

Instruction:

Only after Bialetti is green, analyze the remaining `high_heading_density` warnings. For each affected PDF, classify the cause as running furniture, styled paragraph, PDF/UA tagged role mismatch, or real headings. Do not change classifier thresholds until there is a per-PDF reason.

Acceptance:

- `jq '.quality_warnings[] | select(.kind=="high_heading_density")' target/example-audit/quality/report.json` -> produces a list with a written cause next to each PDF in the plan notes.
- A follow-up implementation plan names the exact classifier rule to change and the expected warning-count delta.

### 7. Image-Density Warning Design Pass

Block: research
Skill: `codebase-explainer` + `rust`

Instruction:

Analyze the Danish magazine high-image-density warning. Decide whether the warning is caused by decorative tiny assets, legitimate figures, or repeated page furniture. Prefer decorative-image filtering only if it does not reduce real figure extraction on academic PDFs.

Acceptance:

- `jq '.cases[] | select(.pdf|contains("Magazine-danish")) | {extracted_images, images_per_page}' target/example-audit/quality/report.json` -> current value recorded.
- A follow-up implementation plan defines a safe image-filter gate and expected `images_per_page <= 10` target.

### 8. Documentation Update

Block: execute
Skill: `writing`

Instruction:

Update `docs/QUALITY_LOOP.md` with the Bialetti result, the before/after quality warning count, and the next chosen target.

Acceptance:

- `rg -n 'Bialetti|glued_numeric_rows|quality_warning_count' docs/QUALITY_LOOP.md` -> exits 0.
- `bash scripts/example-audit.sh` -> summary still matches the documented numbers.

## Non-Goals

- Do not import Marker, Surya, PDF-Extract-Kit, or other GPL/AGPL code.
- Do not make Docling/gmft/img2table mandatory for the default audit.
- Do not lower audit thresholds just to reduce warnings.
- Do not render heuristic financial rows as recovered semantic tables unless the row/column reconstruction is observable in debug JSON.

## Expected Outcome

After Tasks 1-5:

- `glued_numeric_rows`: `2 -> 0` for Bialetti.
- `quality_warning_count`: `7 -> 6`.
- Bialetti formula debug candidates: `9 -> 0/1`.
- No regression in `math-number-theory.pdf` formula crops.

After Tasks 6-7, we should have a separate approved plan for reducing heading-density and image-density warnings. Those are likely to move more warnings, but they are less safe than the Bialetti fix and should not be mixed into the same first implementation pass.
