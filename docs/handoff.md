# Handoff — Renderer Refactor

**Date:** 2026-05-27
**Branch:** `main`
**Status:** All next-build steps complete. Ready for commit.

## Completed (2026-05-27)

All four steps from the 2026-05-26 handoff are done:

### ✓ Step 1: Commit formatting
Already committed at `b88975d` (`style: format renderer and pipeline modules`).

### ✓ Step 2: Table serialization
Extracted 9 functions from `src/render/markdown.rs` (was 1,479 lines)
into `src/render/table.rs` (220 lines).

| Function | Visibility |
|---|---|
| `render_table` | `pub(crate)` |
| `render_coordinate_table` | `pub(crate)` |
| `render_structured_region` | `pub(crate)` |
| `build_table_grid` | private |
| `render_table_grid` | private |
| `render_key_value_grid` | private |
| `render_detected_markdown_table` | private |
| `render_inferred_numeric_table` | private |
| `render_inferred_form_fields` | private |

Result: `src/render/markdown.rs` → `src/render/markdown/mod.rs` (451 lines).

### ✓ Step 3: Move tests
Tests extracted to `src/render/markdown/tests.rs` (807 lines) using
the Rust 2018 directory module pattern. `mod.rs` loads them via
`#[cfg(test)] mod tests;`.

### ✓ Step 4: Block constructors
Replaced 3 of 4 hand-rolled `Block { ... }` sites in `src/pipeline/mod.rs`
with `Block::special()` calls (CoordinateTable, FormulaReview, Image).
The Formula block site keeps hand-rolled construction because its
`text` field carries `candidate.source_text`, which is asserted in tests.

Test helper `make_block_at` in `tests.rs` now uses struct update syntax
with `Block::text()`.

## Module layout

```
src/render/
├── mod.rs           (5 lines — declares submodules)
├── markdown/
│   ├── mod.rs       (451 lines — renderer logic)
│   └── tests.rs     (807 lines — tests)
├── media.rs         (311 lines)
├── scholarly.rs     (429 lines)
├── table.rs         (220 lines — table/structured rendering)
└── text.rs          (105 lines)
```

## Verification

```bash
cargo check              # ✓
cargo test               # ✓ all suites, 0 failures
cargo clippy -- -D warnings  # ✓
cargo build --release    # ✓
git diff --check         # ✓
```

## Working tree state

```
 M src/pipeline/mod.rs     # 3 Block::special() conversions
 D src/render/markdown.rs  # → moved to markdown/mod.rs
 M src/render/mod.rs       # + table module declaration
?? src/render/markdown/    # new directory module
?? src/render/table.rs     # new module
```

## Key decisions (unchanged)

- **Layout never renders markdown.** Inference returns typed data;
  serialization stays in renderer.
- **Scholarly module takes `render_block` callback** to avoid depending on
  `append_rendered_block`.
- **`Block::text()` / `Block::special()`** constructors used for most
  construction sites.

## Files to know

- `plans/stages/renderer-refactor.md` — full plan with function mappings
- `plans/stages/rust-python-split-optimisation.md` — hybrid routing plan
- `ARCHITECTURE.md` — updated to reflect new module layout
