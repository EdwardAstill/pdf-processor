# PDF Quality Algorithm Execution Plan

Status: approved
Date: 2026-05-08
Repo: `/home/eastill/projects/pdf-processor`
Research: `.warden/research/pdf-quality-algorithms-imports/REPORT.md`

## Goal

Implement the practical parts of the research plan in the current repo: strengthen native table detection, suppress formula false positives inside strong tables, improve figure snapshot candidate scoring, and add a repeatable sidecar comparison harness for Docling/gmft/img2table/UniMERNet-style experiments.

## Workspace Note

The Warden planner normally prefers a `.worktrees/` checkout. This execution is intentionally in-place because the current workspace already contains uncommitted polish and quality-loop changes that this plan builds on.

## Assumptions

| id | statement | type | source | check | if false | owner |
| --- | --- | --- | --- | --- | --- | --- |
| A1 | The repo should remain MIT-compatible. | policy | `Cargo.toml` license and research report | `rg -n '^license = \"MIT\"' Cargo.toml` | Keep GPL/AGPL tools benchmark-only. | sidecar harness |
| A2 | Native table improvement should reuse existing `RawWord` geometry. | architectural | `src/layout/table.rs` | `rg -n 'RawWord|detect_coordinate_tables' src/layout/table.rs` | Re-scope to extractor changes before table graph. | table work |
| A3 | Formula recognition stays sidecar-driven; local code only detects/audits. | design | `docs/pdf-internals.md` and formula report | `rg -n 'debug-formulas|UniMERNet' docs src` | Do not emit local heuristic LaTeX as recovered math. | formula work |
| A4 | Figure snapshots already render page bboxes, so detection is the main gap. | architectural | `src/figure/detect.rs`, `src/figure/render.rs` | `rg -n 'render_figure_snapshots|detect_figure_candidates' src/figure` | Re-scope to renderer if snapshots fail. | figure work |

## Tasks

### 1. TDD: Table Graph / Alignment Tests

Block: execute
Skill: `test-driven-development` + `rust`
Instruction: Add focused unit tests in `src/layout/table.rs` for non-numeric text tables and rows with wrapped labels/missing cells.
Acceptance:

- `cargo test layout::table::tests::coordinate_table_detects_text_alignment_grid -- --nocapture` -> initially fails before implementation, then passes.
- `cargo test layout::table::tests::coordinate_table_keeps_wrapped_label_rows_in_layout_fallback -- --nocapture` -> initially fails before implementation, then passes.

### 2. Implement Native Table Hardening

Block: execute
Skill: `rust`
Instruction: Extend `detect_coordinate_tables` with a pdfplumber/Camelot-inspired alignment fallback that detects repeated text-column anchors, handles non-numeric aligned tables, and chooses layout fallback when cells are ambiguous.
Acceptance:

- `cargo test layout::table -- --nocapture` -> passes.
- `cargo test --test quality -- --nocapture` -> passes.

### 3. TDD: Formula/Table Conflict Tests

Block: execute
Skill: `test-driven-development` + `rust`
Instruction: Add a pipeline unit test proving formula candidates overlapping high-confidence coordinate tables are suppressed before rendering/debug summary counts.
Acceptance:

- `cargo test pipeline::tests::suppresses_formula_candidates_inside_strong_tables -- --nocapture` -> initially fails before implementation, then passes.

### 4. Implement Formula/Table Suppression

Block: execute
Skill: `rust`
Instruction: Add a helper in `src/pipeline.rs` that removes or downgrades formula candidates overlapping strong table candidates before debug JSON, crops, warning counts, and formula blocks are produced.
Acceptance:

- `cargo test pipeline::tests::suppresses_formula_candidates_inside_strong_tables -- --nocapture` -> passes.
- `cargo test --test formulas -- --nocapture` -> passes.

### 5. TDD: Figure Proposal Scoring Tests

Block: execute
Skill: `test-driven-development` + `rust`
Instruction: Add unit tests in `src/figure/detect.rs` for caption-only regions preferring the side with visual/whitespace evidence and rejecting body-text-contaminated caption estimates.
Acceptance:

- `cargo test figure::detect::tests::caption_only_region_scores_against_body_text -- --nocapture` -> initially fails before implementation, then passes.
- `cargo test figure::detect::tests::caption_only_region_can_choose_region_below_caption -- --nocapture` -> initially fails before implementation, then passes.

### 6. Implement Figure Proposal Scoring

Block: execute
Skill: `rust`
Instruction: Improve caption-only and caption-near-image candidates with proposal scoring that penalizes body text contamination, considers regions above and below captions, and records confidence/reason details.
Acceptance:

- `cargo test figure::detect -- --nocapture` -> passes.
- `cargo test --test figure_snapshots -- --nocapture` -> passes.

### 7. Add Sidecar Benchmark Harness

Block: execute
Skill: `rust` + `writing`
Instruction: Add `scripts/sidecar-audit.sh` to run native plus optional external sidecars into separate directories without requiring those sidecars to be installed. Update docs to describe the harness and opt-in environment variables.
Acceptance:

- `bash scripts/sidecar-audit.sh` -> exits 0 when optional sidecars are unavailable and writes `target/sidecar-audit/summary.md`.
- `rg -n 'sidecar-audit|gmft|img2table|UniMERNet' docs/QUALITY_LOOP.md docs/TESTING.md` -> exits 0.

### 8. Review And Verification

Block: review
Skill: `verification-before-completion`
Instruction: Run focused tests, formatting, and the sidecar harness. Report exact pass/fail status and any remaining limits.
Acceptance:

- `cargo fmt -- --check` -> passes.
- `cargo test layout::table figure::detect pipeline::tests::suppresses_formula_candidates_inside_strong_tables -- --nocapture` -> passes.
- `cargo test --test formulas --test figure_snapshots --test quality -- --nocapture` -> passes.
- `bash scripts/sidecar-audit.sh` -> passes or cleanly skips unavailable optional sidecars.
