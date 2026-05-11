# DNV Formula Path Plan

status: completed
created: 2026-05-11
worktree: .worktrees/dnv-formula-path

## Goal

Implement the next `next.md` increment: detect visible equation bands that the word-based formula detector misses, especially DNV page-670-style equations, and surface them as debug crops plus Markdown review markers without breaking conservative mode.

## Assumptions

- A1: Existing MuPDF rendering is sufficient for low-DPI visual band detection.
  - type: architectural
  - source: `src/figure/render.rs`
  - check: compile and tests using `mupdf::Page::run` path
  - if false: defer to external image library or sidecar design
  - owner: implementation
- A2: Visual-only candidates should not emit blank display math.
  - type: design
  - source: `next.md` review-marker requirement
  - check: renderer test for formula-review comments
  - if false: represent visual-only candidates only in debug JSON
  - owner: renderer tests
- A3: Heavy page rasterization should be opt-in via `--debug-formulas` for now.
  - type: policy
  - source: default CLI performance concerns
  - check: pipeline only calls visual detector when debug formulas are enabled
  - if false: add a separate CLI mode later
  - owner: pipeline implementation

## Subtasks

1. Research existing rendering/debug hooks.
   - block: research
   - skill: rust
   - instruction: map formula candidate creation, formula crop writing, and markdown rendering extension points.

2. Add visual formula detector.
   - block: execute
   - skill: rust
   - instruction: add a module under `src/formula/` that renders a page at low DPI, scans horizontal dark-pixel bands, filters by isolation/cue words/reference pages, and returns `FormulaCandidate`s with `source_text=""`, backend `visual-page-render`, and review reason.
   - acceptance: `cargo test formula::visual` -> visual detector tests pass.

3. Integrate visual candidates into pipeline/debug output.
   - block: execute
   - skill: rust
   - instruction: when `--debug-formulas` is enabled and formulas are not off, merge visual candidates after word candidates and table suppression, dedupe by overlap, and crop them with existing `write_formula_debug`.
   - acceptance: `cargo test pipeline::tests` -> pipeline tests pass.

4. Add Markdown review markers for unresolved visual formula regions.
   - block: execute
   - skill: rust
   - instruction: add a document block representation and renderer output for `<!-- formula-review: ... -->` comments; never emit blank `$$`.
   - acceptance: `cargo test render::markdown::tests::*formula*` or relevant renderer tests pass.

5. Update docs/next note.
   - block: execute
   - skill: writing
   - instruction: update CLI/docs/next note to describe visual formula audit as opt-in under debug formulas.
   - acceptance: `rg -n "visual|formula-review|debug-formulas" docs README.md next.md` shows updated docs.

6. Verify.
   - block: review
   - skill: verification-before-completion
   - instruction: run `cargo fmt --check`, `cargo test`, and `cargo clippy --all-targets -- -D warnings` fresh.

## Verification Result

- `cargo fmt --check` passed.
- `cargo test` passed: 157 unit tests plus integration tests; existing slow/live tests remained ignored.
- `cargo clippy --all-targets -- -D warnings` passed.
- DNV conservative debug run produced a visual review crop for page 670 and no visual candidates on pages 69 or 597.
