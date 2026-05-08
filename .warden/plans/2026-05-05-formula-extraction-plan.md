# Plan: Formula Extraction And Standards Formula Coverage

**Spec:** none
**Created:** 2026-05-05T16:20+08:00
**Status:** done
**Shape:** research-plan-execute-review
**Human checkpoints:** 1
**Refinement passes:** 1

**Completed:** 2026-05-05T17:05+08:00

## Goal

Add a formula-aware path to `pdfp` so standards processing can detect, audit, and recover equations instead of silently shipping formula gaps into PySeas wiki pages.

## Background

Research is saved in `.warden/research/formula-extraction-tools/REPORT.md`. The strongest near-term tool is Docling formula enrichment because `pdfp` already has a Docling hybrid path. The best future local sidecar is a formula crop recognizer such as UniMERNet/PDF-Extract-Kit, but that should be justified by corpus evidence before adding model management.

## Assumptions

- `A1` — `pdfp` should stay local-first and keep formula recovery optional.
  - Type: architectural
  - Source: project docs and prior OCR/sidecar decisions
  - Check: `rg -n "local-first|optional local OCR|hybrid Docling" README.md docs wiki`
  - If false: re-plan around a model-first converter backend such as MinerU or Marker.
  - Owner: sub-task 1

- `A2` — Docling formula enrichment is the first backend to test because `src/hybrid/client.rs` already sends `do_formula_enrichment=true`.
  - Type: repo-state
  - Source: `src/hybrid/client.rs` and research report
  - Check: `rg -n "do_formula_enrichment|hybrid docling" src docs README.md`
  - If false: add Docling formula options before benchmark work, or choose UniMERNet sidecar first.
  - Owner: sub-task 1

- `A3` — DNV-ST-N001 formula gaps are measurable from local standards PDFs and converted Markdown.
  - Type: external
  - Source: user report from standard-processing work
  - Check: `find /home/eastill/projects/literature/standards -iname '*N001*' -o -iname '*DNV*'`
  - If false: use the math fixtures listed in `docs/TESTING.md` and add DNV acceptance later.
  - Owner: sub-task 2

- `A4` — Standard-processing must remain conservative: pages with unresolved formulas stay `draft` or `pdf_review_status: partial`.
  - Type: policy
  - Source: `/home/eastill/projects/warden/core/skills/pyseas/standard-processing/SKILL.md`
  - Check: `rg -n "formula|draft|pdf_review_status|coverage ledger" /home/eastill/projects/warden/core/skills/pyseas/standard-processing/SKILL.md`
  - If false: update the standard-processing skill first.
  - Owner: sub-task 8

## Sub-tasks

### Sub-task 1 — Lock the formula feature contract

**Block:** execute
**Skill:** system-designing
**Depends on:** none
**Assumption refs:** A1, A2

**Instruction:**

Design the formula extraction contract before coding. Define the CLI surface, internal data model, debug artifact shape, backend escalation policy, and standard-processing handoff. Keep defaults conservative. Expected contract:

- `--formulas auto|local|hybrid|off`, default `auto`
- `--debug-formulas`
- internal formula candidates with `page_num`, `bbox`, `source_text`, `equation_number`, `confidence`, `status`, and optional `latex`
- debug output under `debug/formulas/page{N}.json`
- Markdown emission as `$$ ... $$` only when confidence is high or backend LaTeX is available
- unresolved formula warnings in debug/audit output, not fake reconstructed equations

**Inputs (pre-staged):**
- file: `src/cli.rs`
- file: `src/document/types.rs`
- file: `src/pipeline.rs`
- file: `src/render/markdown.rs`
- file: `src/hybrid/client.rs`
- file: `.warden/research/formula-extraction-tools/REPORT.md`

**Acceptance:**
- `test -f .warden/research/formula-extraction-tools/REPORT.md` → exits 0
- `rg -n "do_formula_enrichment" src/hybrid/client.rs` → exits 0
- `rg -n "Formula" src/document/types.rs src/render/markdown.rs` → exits 0

### Sub-task 2 — Build a DNV/formula baseline harness

**Block:** execute
**Skill:** test-driven-development
**Depends on:** sub-task 1
**Assumption refs:** A3

**Instruction:**

Create a small formula baseline harness before changing extraction. It should run `pdfp` on representative math-heavy fixtures and, when available, selected DNV-ST-N001 pages from `/home/eastill/projects/literature/standards`. Record:

- page number
- equation-number candidates
- local extracted text around the equation
- whether the Markdown contains usable formula content
- whether `--hybrid docling` improves the output when a Docling server is available

Keep the harness optional for private standards PDFs; tests must pass when DNV fixtures are absent.

**Inputs (pre-staged):**
- file: `docs/TESTING.md`
- file: `tests/golden.rs`
- file: `tests/hybrid.rs`
- file: `tests/quality.rs`

**Acceptance:**
- `cargo test --test quality formula_baseline_skips_when_standards_absent` → passes
- `cargo test --test quality formula_candidate_report_contains_page_and_status` → passes
- `rg -n "formula baseline|formula_candidate|FORMULA" docs/TESTING.md tests/quality.rs` → exits 0

### Sub-task 3 — Add local formula candidate detection

**Block:** execute
**Skill:** rust
**Depends on:** sub-task 2
**Assumption refs:** A1

**Instruction:**

Add a local candidate detector that works from `RawWord` and `RawTextBlock` geometry. Create `src/formula/mod.rs` and `src/formula/detect.rs`. Detect likely display equations using:

- math-symbol density;
- superscript/subscript-heavy words;
- centered short lines;
- nearby equation numbers such as `(1)`, `(P.3-2)`, or `(16.4.1)`;
- sparse line layout that does not look like prose or a table.

The output should be candidates only. Do not claim high-confidence LaTeX reconstruction yet.

**Inputs (pre-staged):**
- file: `src/document/types.rs`
- file: `src/pdf/extractor.rs`
- file: `src/layout/table.rs`
- file: `src/hybrid/triage.rs`

**Acceptance:**
- `cargo test formula::detect::tests::detects_centered_equation_with_number` → passes
- `cargo test formula::detect::tests::ignores_normal_paragraph_with_parentheses` → passes
- `cargo test formula::detect::tests::keeps_formula_candidate_bbox_inside_page` → passes
- `rg -n "pub mod formula|detect_formula_candidates" src/main.rs src/formula src/pipeline.rs` → exits 0

### Sub-task 4 — Add formula debug ledger and rendered crops

**Block:** execute
**Skill:** rust
**Depends on:** sub-task 3
**Assumption refs:** A1

**Instruction:**

Add `--debug-formulas` and write a formula coverage ledger for every page with candidates. Reuse the figure snapshot rendering pattern to crop formula regions into `debug/formulas/page{N}_formula{M}.png` when possible. The JSON ledger should include candidate bbox, reason, equation number, local text, status, backend source, confidence, and crop path.

This is the audit backbone for standards. It must be useful even before formula recognition is implemented.

**Inputs (pre-staged):**
- file: `src/cli.rs`
- file: `src/figure/render.rs`
- file: `src/pipeline.rs`
- file: `src/figure/detect.rs`
- file: `tests/figure_snapshots.rs`
- create: `tests/formulas.rs`

**Acceptance:**
- `cargo test --test formulas debug_formulas_writes_json_for_candidate_page` → passes
- `cargo test --test formulas debug_formulas_writes_crop_for_candidate_page` → passes
- `target/debug/pdfp convert --help | rg -- '--formulas|--debug-formulas'` → exits 0
- `rg -n "debug/formulas|formula.*json|formula.*png" src tests docs/TESTING.md` → exits 0

### Sub-task 5 — Route formula-heavy pages through Docling and preserve LaTeX

**Block:** execute
**Skill:** rust
**Depends on:** sub-task 4
**Assumption refs:** A2

**Instruction:**

Extend the hybrid path so formula-heavy pages route to Docling when `--formulas hybrid` or `--formulas auto` plus `--hybrid docling` is enabled. Preserve backend Markdown containing display formulas. Add response parsing tests for Markdown containing `$$ ... $$`, inline `$...$`, and Docling-style escaped LaTeX.

Do not require a live Docling server in normal tests. Keep `hybrid_live` ignored.

**Inputs (pre-staged):**
- file: `src/hybrid/client.rs`
- file: `src/hybrid/mod.rs`
- file: `src/hybrid/triage.rs`
- file: `src/hybrid/page_extract.rs`
- file: `tests/hybrid.rs`
- file: `src/render/markdown.rs`

**Acceptance:**
- `cargo test --test hybrid hybrid_docling_preserves_display_formula_markdown` → passes
- `cargo test hybrid::triage::tests::formula_candidate_routes_page` → passes
- `cargo test --test formulas formulas_auto_without_hybrid_keeps_local_output_and_warning` → passes
- `rg -n "do_formula_enrichment|formula.*route|Formula" src/hybrid src/render src/document` → exits 0

### Sub-task 6 — Add optional formula sidecar abstraction, but defer concrete model binding

**Block:** execute
**Skill:** system-designing
**Depends on:** sub-task 5
**Assumption refs:** A1

**Instruction:**

Design the sidecar interface for a future UniMERNet/PDF-Extract-Kit or Mathpix backend without committing to either in the first implementation. The contract should accept one rendered formula crop and return JSON:

```json
{
  "latex": "...",
  "confidence": 0.92,
  "engine": "unimernet",
  "warnings": []
}
```

Add a fake sidecar test adapter only if needed for test coverage. Do not download models, vendor Python code, or add hard runtime dependencies in this task.

**Inputs (pre-staged):**
- file: `.warden/research/formula-extraction-tools/REPORT.md`
- file: `src/ocr/mod.rs`
- file: `src/processor/doctor.rs`
- file: `docs/CLI.md`

**Acceptance:**
- `rg -n "formula sidecar|UniMERNet|Mathpix|PDF-Extract-Kit" docs/CLI.md docs/pdf-internals.md wiki` → exits 0
- `cargo test formula` → passes
- `cargo test --test formulas` → passes

### Sub-task 7 — Update docs and user guide

**Block:** execute
**Skill:** writing
**Depends on:** sub-task 6
**Assumption refs:** A1, A2

**Instruction:**

Document formula handling honestly. Update the CLI guide, README, testing docs, and PDF internals docs. Explain:

- formulas are not generic OCR;
- local mode detects and audits candidates;
- Docling formula enrichment is the first recovery backend;
- formula sidecars are optional future/advanced integrations;
- standards pages with formula gaps must remain draft.

**Inputs (pre-staged):**
- file: `README.md`
- file: `docs/CLI.md`
- file: `docs/TESTING.md`
- file: `docs/pdf-internals.md`
- file: `wiki/opendataloader-ecosystem.md`
- file: `wiki/text-extraction.md`

**Acceptance:**
- `rg -n -- '--formulas|--debug-formulas|formula enrichment|formula gaps|UniMERNet|Docling' README.md docs wiki` → exits 0
- `cargo run --quiet -- convert --help | rg -- '--formulas|--debug-formulas'` → exits 0
- `cargo test --test cli_help` → passes

### Sub-task 8 — Update Warden standard-processing rules

**Block:** execute
**Skill:** designing-skills
**Depends on:** sub-task 7
**Assumption refs:** A4

**Instruction:**

Update `/home/eastill/projects/warden/core/skills/pyseas/standard-processing/SKILL.md` so standards processing uses the formula ledger. The skill must require:

- `pdfp convert ... --debug-formulas` for formula-heavy chapters;
- a formula coverage ledger per chapter;
- cross-check against the PDF page range;
- visible warning and `draft` status for unresolved formula gaps;
- no `active` page when formula extraction is unchecked or partial.

Do not touch unrelated Warden skill files.

**Inputs (pre-staged):**
- file: `/home/eastill/projects/warden/core/skills/pyseas/standard-processing/SKILL.md`
- file: `/home/eastill/projects/warden/core/skills/files/pdf-processing/SKILL.md`

**Acceptance:**
- `rg -n "debug-formulas|formula coverage ledger|FORMULA EXTRACTION REQUIRED|pdf_review_status" /home/eastill/projects/warden/core/skills/pyseas/standard-processing/SKILL.md` → exits 0
- `git -C /home/eastill/projects/warden diff --name-only | rg '^core/skills/(pyseas/standard-processing/SKILL.md|files/pdf-processing/SKILL.md)$'` → only relevant skill files are present for this task

### Sub-task 9 — Benchmark on DNV/math fixtures and decide second backend

**Block:** review
**Skill:** verification-before-completion
**Depends on:** sub-task 8
**Assumption refs:** A3

**Instruction:**

Run the final acceptance gate. Benchmark the feature against:

- the local math fixtures listed in `docs/TESTING.md`;
- DNV-ST-N001 pages if present locally;
- Docling live backend if available.

Produce a short report under `.warden/research/formula-extraction-tools/implementation-results.md` with formula candidates found, formulas recovered, unresolved gaps, runtime, and whether UniMERNet/PDF-Extract-Kit should be implemented next.

**Inputs (pre-staged):**
- file: `docs/TESTING.md`
- file: `.warden/research/formula-extraction-tools/REPORT.md`

**Acceptance:**
- `cargo fmt --check` → exits 0
- `cargo test` → passes
- `cargo run --quiet -- convert --help | rg -- '--formulas|--debug-formulas'` → exits 0
- `test -f .warden/research/formula-extraction-tools/implementation-results.md` → exits 0

### Sub-task 10 — Human checkpoint: approve second backend work

**Block:** human
**Skill:** ask
**Depends on:** sub-task 9
**Assumption refs:** A1, A3

**Instruction:**

Ask the user whether to proceed with a second formula backend. Present the benchmark result and recommend one of:

- stop after Docling because coverage is good enough;
- add UniMERNet/PDF-Extract-Kit local sidecar;
- add Mathpix cloud backend for maximum formula quality;
- test PaddleOCR-VL/MinerU before choosing.

**Inputs (pre-staged):**
- file: `.warden/research/formula-extraction-tools/implementation-results.md`

**Acceptance:**
- User selects a next backend path or confirms no second backend is needed.

## Refinement Check

- `rg --files src tests docs wiki .warden/research/formula-extraction-tools` confirmed referenced in-repo paths exist or are listed as creates.
- Skill names are from the active catalog: `system-designing`, `test-driven-development`, `rust`, `writing`, `designing-skills`, `verification-before-completion`, `ask`.
- Research source exists at `.warden/research/formula-extraction-tools/REPORT.md`.
- Execute acceptance checks are runnable shell commands.
- No implementation work should start until sub-task 1 confirms the contract.
