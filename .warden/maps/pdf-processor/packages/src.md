# Package: src


## `src/batch.rs`

- `<module>` (module) — `src/batch.rs:<module>`
- `is_supported` (function) — `batch::is_supported`
- `output_dir_for` (function) — `batch::output_dir_for`
- `resolve_inputs` (function) — `batch::resolve_inputs`

## `src/cli.rs`

- `<module>` (module) — `src/cli.rs:<module>`
- `AppCommand` (type) — `cli::AppCommand`
- `Cli` (type) — `cli::Cli`
- `Command` (type) — `cli::Command`
- `ConvertArgs` (type) — `cli::ConvertArgs`
- `ConvertOptions` (type) — `cli::ConvertOptions`
- `DoctorArgs` (type) — `cli::DoctorArgs`
- `FigureMode` (type) — `cli::FigureMode`
- `FormulaMode` (type) — `cli::FormulaMode`
- `HybridMode` (type) — `cli::HybridMode`
- `HybridPolicy` (type) — `cli::HybridPolicy`
- `ImposeArgs` (type) — `cli::ImposeArgs`
- `ImposeCommand` (type) — `cli::ImposeCommand`
- `ImposeSubcommand` (type) — `cli::ImposeSubcommand`
- `InputType` (type) — `cli::InputType`
- `InspectArgs` (type) — `cli::InspectArgs`
- `MergeArgs` (type) — `cli::MergeArgs`
- `OcrArgs` (type) — `cli::OcrArgs`
- `OcrMode` (type) — `cli::OcrMode`
- `OcrOptions` (type) — `cli::OcrOptions`
- `PageCommand` (type) — `cli::PageCommand`
- `PageSelectionArgs` (type) — `cli::PageSelectionArgs`
- `PageSubcommand` (type) — `cli::PageSubcommand`
- `PagesCommand` (type) — `cli::PagesCommand`
- `PagesSubcommand` (type) — `cli::PagesSubcommand`
- `ResizeArgs` (type) — `cli::ResizeArgs`
- `SUPPORTED_EXTENSIONS` (constant) — `cli::SUPPORTED_EXTENSIONS`
- `SearchArgs` (type) — `cli::SearchArgs`
- `SplitArgs` (type) — `cli::SplitArgs`
- `StandaloneOcrMode` (type) — `cli::StandaloneOcrMode`
- `TableMode` (type) — `cli::TableMode`
- `conservative_mode_uses_review_safe_conversion_modes` (function) — `cli::conservative_mode_uses_review_safe_conversion_modes`
- `convert_options` (function) — `cli::convert_options`
- `default` (function) — `cli::default`
- `effective_figure_mode` (function) — `cli::effective_figure_mode`
- `effective_formula_mode` (function) — `cli::effective_formula_mode`
- `effective_table_mode` (function) — `cli::effective_table_mode`
- `extensions` (function) — `cli::extensions`
- `from` (function) — `cli::from`
- `from_path` (function) — `cli::from_path`
- `into_command` (function) — `cli::into_command`
- `is_on` (function) — `cli::is_on`
- `non_conservative_mode_preserves_selected_conversion_modes` (function) — `cli::non_conservative_mode_preserves_selected_conversion_modes`

## `src/commands.rs`

- `<module>` (module) — `src/commands.rs:<module>`
- `process_one` (function) — `commands::process_one`
- `run` (function) — `commands::run`
- `run_convert` (function) — `commands::run_convert`

## `src/error.rs`

- `<module>` (module) — `src/error.rs:<module>`
- `VtvError` (type) — `error::VtvError`

## `src/main.rs`

- `<module>` (module) — `src/main.rs:<module>`
- `main` (function) — `main::main`

## `src/pipeline.rs`

- `<module>` (module) — `src/pipeline.rs:<module>`
- `BuiltPage` (type) — `pipeline::BuiltPage`
- `PageBuildContext` (type) — `pipeline::PageBuildContext`
- `apply_hybrid_if_enabled` (function) — `pipeline::apply_hybrid_if_enabled`
- `bbox_overlap_ratio` (function) — `pipeline::bbox_overlap_ratio`
- `build_document` (function) — `pipeline::build_document`
- `build_document_from_raw` (function) — `pipeline::build_document_from_raw`
- `build_page` (function) — `pipeline::build_page`
- `formula_candidates_to_blocks` (function) — `pipeline::formula_candidates_to_blocks`
- `merge_media_blocks` (function) — `pipeline::merge_media_blocks`
- `merge_text_and_formulas` (function) — `pipeline::merge_text_and_formulas`
- `merge_text_and_images` (function) — `pipeline::merge_text_and_images`
- `merge_text_and_tables` (function) — `pipeline::merge_text_and_tables`
- `process_pdf` (function) — `pipeline::process_pdf`
- `save_page_images` (function) — `pipeline::save_page_images`
- `suppress_text_covered_by_tables` (function) — `pipeline::suppress_text_covered_by_tables`
- `table_candidates_to_blocks` (function) — `pipeline::table_candidates_to_blocks`
- `warn_on_formula_candidate_summary` (function) — `pipeline::warn_on_formula_candidate_summary`
- `warn_on_scan_like_pages` (function) — `pipeline::warn_on_scan_like_pages`
- `write_document` (function) — `pipeline::write_document`
- `write_figure_debug` (function) — `pipeline::write_figure_debug`
- `write_formula_debug` (function) — `pipeline::write_formula_debug`
- `write_table_debug` (function) — `pipeline::write_table_debug`
