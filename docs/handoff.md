# Handoff — Renderer Refactor

**Date:** 2026-05-26
**Branch:** `main` (committed at `f1f5a34`)
**Status:** Renderer split complete. Working tree has `cargo fmt` diffs only.

## Project status

Renderer refactor done. `src/render/markdown.rs` reduced from ~2,670 to 1,479 lines.
Four focused modules extracted:

| Module | Lines | Role |
|---|---|---|
| `src/render/text.rs` | 105 | Text normalization, inline-wrap, escaping |
| `src/render/media.rs` | 311 | Media dedup, page media plan, RenderContext |
| `src/render/scholarly.rs` | 429 | First-page scholarly front matter |
| `src/layout/table_inference.rs` | 495 | Table/form inference (returns structural data, not markdown) |

The circular dependency is broken: `layout/table_inference` returns
`StructuredRegionKind` variants. `render/markdown.rs` serializes them to
markdown. Layout no longer calls `render_table()`.

## Verification (all passing)

```bash
cargo check
cargo test          # all suites, 0 failures
cargo clippy -- -D warnings
cargo build --release
git diff --check
```

## Working tree state

11 modified files — all `cargo fmt` reformatting only (no semantic changes):
`src/figure/detect.rs`, `src/figure/render.rs`, `src/layout/table_inference.rs`,
`src/pipeline/merge.rs`, `src/pipeline/mod.rs`, `src/render/markdown.rs`,
`src/render/media.rs`, `src/render/mod.rs`, `src/render/scholarly.rs`,
`src/render/text.rs`, `tests/eval_integration.rs`

Untracked artifacts from earlier session: `plans/`, `tools/eval_benchmarks/`,
`wiki/`, `.pied/`, `.pi-lens/`.

## Next build steps

1. **Commit formatting** — stage and commit the `cargo fmt` diffs.
2. **Table serialization** — extract `render_table`, `build_table_grid`,
   `render_table_grid`, `render_key_value_grid`, `render_coordinate_table`,
   `render_detected_markdown_table`, `render_structured_region`,
   `render_inferred_numeric_table`, `render_inferred_form_fields` into
   `src/render/table.rs`.
3. **Move tests** — extract `#[cfg(test)] mod tests` from `markdown.rs` into
   `src/render/markdown/tests.rs` (Rust 2018 submodule pattern).
4. **Block constructors** — `Block::text()` and `Block::special()` exist but
   are not yet used everywhere. Replace remaining hand-rolled Block
   construction in `src/pipeline/mod.rs` and `src/render/markdown.rs` tests
   with constructors.

## Key decisions

- **Layout never renders markdown.** Inference returns typed data;
  serialization stays in renderer.
- **Scholarly module takes `render_block` callback** to avoid depending on
  `append_rendered_block` (which lives in markdown.rs).
- **`Block::text()` / `Block::special()`** constructors reduce ~18
  construction sites to typed factory calls.

## Files to know

- `plans/stages/renderer-refactor.md` — full plan with function mappings
- `plans/stages/rust-python-split-optimisation.md` — hybrid routing plan
- `ARCHITECTURE.md` — needs update to reflect new module layout
