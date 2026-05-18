# PDF Tool Comparison Table Plan

**Status:** completed
**Created:** 2026-05-15
**Branch:** `stage-8-heading-formula`

## Task

Answer whether `pdfp` is on par with leading PDF extraction tools and add a
maintainable comparison table to the repo.

## Shape

Research -> execute docs -> review.

## Assumptions

- `A1`
  - statement: The comparison should be honest and source-backed, not a claim
    that `pdfp` is generally best because local fixture numbers improved.
  - type: external
  - source: user asked about "best tools out there".
  - check: compare against current official docs for leading local/open-source
    and cloud/API parsers.
  - if false: reduce the work to a short status answer only.
  - owner: research
- `A2`
  - statement: The repo already has enough local benchmark machinery to hold a
    comparison table, even if third-party runs are not installed yet.
  - type: repo-state
  - source: `pdfp eval` and `scripts/sidecar-audit.sh`.
  - check: `rg -n "sidecar-audit|pdfp eval" docs scripts`.
  - if false: first add benchmark harness work instead of a table.
  - owner: research

## Tasks

1. Research current tool capabilities.
   - block: research
   - skill: `warden:deep-online-research`
   - instruction: Check official docs/repos for Docling, PyMuPDF4LLM, Marker,
     MinerU, LlamaParse, Adobe PDF Extract, Mathpix, and Unstructured. Record
     strengths, deployment model, licensing/dependency cautions, and what each
     can benchmark against `pdfp`.

2. Add a repo comparison table.
   - block: execute
   - skill: `warden:writing`
   - instruction: Add a concise comparison table under `docs/` that separates
     measured `pdfp` numbers from unmeasured third-party claims and states what
     must be run before declaring parity.
   - acceptance: `rg -n "Tool Comparison|Docling|MinerU|Mathpix|LlamaParse" docs` -> exits 0.

3. Update the stage/future-work records.
   - block: execute
   - skill: `warden:writing`
   - instruction: Link the comparison table from the quality/stage notes and
     make Stage 10's quality matrix requirement point to measured comparison
     data.
   - acceptance: `rg -n "TOOL_COMPARISON|comparison table|not on par" .warden docs README.md` -> exits 0.

4. Verify.
   - block: review
   - skill: `warden:verification-before-completion`
   - instruction: Run doc-focused checks plus the existing eval command to make
     sure the comparison uses current measured numbers.

## Result

- Added `docs/TOOL_COMPARISON.md` with the parity verdict, current measured
  `pdfp` floors, and a sourced capability matrix across Docling, PyMuPDF4LLM,
  Marker, MinerU, Mathpix, Adobe PDF Extract, LlamaParse, and Unstructured.
- Refocused the primary comparison on deterministic peers: PyMuPDF4LLM, Poppler
  `pdftotext`, pdfminer.six, pdfplumber, Camelot, Tabula, and OCRmyPDF.
- Extended `scripts/sidecar-audit.sh` so those deterministic peers run or skip
  cleanly when the relevant command/module is unavailable.
- Added research artifacts under `.warden/research/pdf-tool-comparison/`.
- Linked the table from `README.md`, `docs/QUALITY_LOOP.md`, Stage 10 planning,
  and `.warden/NEXT.md`.

## Verification

- `rg -n "Tool Comparison|Docling|MinerU|Mathpix|LlamaParse|not broadly on par|sidecar-audit" docs README.md .warden/plans/2026-05-13-next-stage-goals.md .warden/NEXT.md .warden/research/pdf-tool-comparison` -> pass.
- `test -f .warden/research/pdf-tool-comparison/RESEARCH_PLAN.md && test -f .warden/research/pdf-tool-comparison/evidence.jsonl && test -f .warden/research/pdf-tool-comparison/contradictions.md && test -f .warden/research/pdf-tool-comparison/REPORT.md && test -f docs/TOOL_COMPARISON.md` -> pass.
- `cargo run --quiet -- eval tests/eval_fixtures/` -> pass; evaluated 5
  documents and skipped the intentionally missing sample fixture.
- `bash -n scripts/sidecar-audit.sh` -> pass.
- `bash scripts/sidecar-audit.sh` -> pass; native and `pdftotext-layout` ran
  on the three default fixtures, while missing Python/sidecar tools skipped
  cleanly.
- `git diff --check` -> pass.
