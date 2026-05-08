# Coordinate Table Reconstruction Baseline

date: 2026-05-05
status: complete

## Commands

```sh
cargo test
cargo test coordinate_table
cargo test --test cli_help
cargo run --quiet -- convert "/home/eastill/projects/literature/specs/crosby/Crosby & Gunnebo Industries Catalog - Metric 2025-2026.pdf" -o /tmp/pdfp-crosby-table-test --tables native --no-images
cargo run --quiet -- convert "/home/eastill/projects/literature/specs/crosby/Crosby & Gunnebo Industries Catalog - Metric 2025-2026.pdf" -o /tmp/pdfp-crosby-layout-test --tables layout --no-images
cargo run --quiet -- convert "/home/eastill/projects/literature/specs/crosby/Crosby & Gunnebo Industries Catalog - Metric 2025-2026.pdf" -o /tmp/pdfp-crosby-debug-test --tables native --debug-tables --no-images
cargo run --quiet -- convert "/home/eastill/projects/literature/specs/crosby/Crosby & Gunnebo Industries Catalog - Metric 2025-2026.pdf" -o /tmp/pdfp-crosby-auto-test --no-images
```

## Results

- Full Rust test suite passed: 134 unit tests plus CLI, figure, golden, hybrid, OCR, and quality integration tests. Ignored slow/live tests remained ignored.
- Crosby native table run completed successfully with 2 scan-like page warnings out of 433 pages.
- Crosby page 23 / printed page 22 now renders `G-213 Round Pin Anchor Shackles` as a Markdown table.
- Default `--tables auto` also renders the G-213 table as Markdown.
- The native output includes expected values: `1018017`, `1018295`, and `1082330`.
- The first G-213 table debug file was written at `/tmp/pdfp-crosby-debug-test/Crosby & Gunnebo Industries Catalog - Metric 2025-2026/debug/tables/page23.json`.
- Page 23 debug confidence for the G-213 table was `0.8022956` with render mode `Markdown`.
- Layout mode produced fenced fixed-width table text for the same G-213 table and retained the expected values.

## Remaining Quality Notes

- The G-213 table is materially improved and meets the target fixture checks.
- Some later Crosby catalogue tables still show imperfect headers or split mixed-number cells. This is a refinement issue, not a blocker for the first coordinate-table pass.
- `--tables layout` is the safest mode for manufacturer catalogues when exact visual alignment matters more than strict Markdown cell structure.
