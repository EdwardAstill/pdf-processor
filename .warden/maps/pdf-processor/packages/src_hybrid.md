# Package: src/hybrid


## `src/hybrid/client.rs`

- `<module>` (module) — `src/hybrid/client.rs:<module>`
- `ConvertResponse` (type) — `client::ConvertResponse`
- `DOCLING_OPTIONS` (constant) — `client::DOCLING_OPTIONS`
- `DoclingClient` (type) — `client::DoclingClient`
- `DoclingDocumentField` (type) — `client::DoclingDocumentField`
- `convert_bytes_to_markdown` (function) — `client::convert_bytes_to_markdown`
- `extract_markdown` (function) — `client::extract_markdown`
- `extract_markdown_from_document_md_content` (function) — `client::extract_markdown_from_document_md_content`
- `extract_markdown_from_top_level_md_content` (function) — `client::extract_markdown_from_top_level_md_content`
- `extract_markdown_none_when_all_empty` (function) — `client::extract_markdown_none_when_all_empty`
- `extract_markdown_none_when_missing` (function) — `client::extract_markdown_none_when_missing`
- `extract_markdown_prefers_top_level_md_content` (function) — `client::extract_markdown_prefers_top_level_md_content`
- `new` (function) — `client::new`
- `send_and_parse` (function) — `client::send_and_parse`

## `src/hybrid/mod.rs`

- `<module>` (module) — `src/hybrid/mod.rs:<module>`
- `HybridStats` (type) — `mod::HybridStats`
- `RoutingPolicy` (type) — `mod::RoutingPolicy`
- `apply_to_document` (function) — `mod::apply_to_document`
- `apply_uses_cached_markdown_without_backend_or_pdf_extraction` (function) — `mod::apply_uses_cached_markdown_without_backend_or_pdf_extraction`
- `cache_key` (function) — `mod::cache_key`
- `cache_path` (function) — `mod::cache_path`
- `routed_page` (function) — `mod::routed_page`
- `write_cache_entry` (function) — `mod::write_cache_entry`

## `src/hybrid/page_extract.rs`

- `<module>` (module) — `src/hybrid/page_extract.rs:<module>`
- `extract_page_as_pdf_bytes` (function) — `page_extract::extract_page_as_pdf_bytes`
- `temp_file_path` (function) — `page_extract::temp_file_path`

## `src/hybrid/triage.rs`

- `<module>` (module) — `src/hybrid/triage.rs:<module>`
- `MATH_SYMBOL_THRESHOLD` (constant) — `triage::MATH_SYMBOL_THRESHOLD`
- `MIN_TEXT_AREA_FRACTION` (constant) — `triage::MIN_TEXT_AREA_FRACTION`
- `ScanReport` (type) — `triage::ScanReport`
- `block` (function) — `triage::block`
- `block_counts_as_readable_text` (function) — `triage::block_counts_as_readable_text`
- `count_math_chars` (function) — `triage::count_math_chars`
- `empty_page_is_low_density` (function) — `triage::empty_page_is_low_density`
- `formula_candidate_routes_page` (function) — `triage::formula_candidate_routes_page`
- `has_formula_candidate` (function) — `triage::has_formula_candidate`
- `has_readable_text` (function) — `triage::has_readable_text`
- `has_table` (function) — `triage::has_table`
- `image_only_page_does_not_trigger_low_density` (function) — `triage::image_only_page_does_not_trigger_low_density`
- `image_only_page_is_not_readable_text` (function) — `triage::image_only_page_is_not_readable_text`
- `is_image_only` (function) — `triage::is_image_only`
- `is_low_density` (function) — `triage::is_low_density`
- `is_math_char` (function) — `triage::is_math_char`
- `is_math_heavy` (function) — `triage::is_math_heavy`
- `likely_scan_like` (function) — `triage::likely_scan_like`
- `math_heavy_page_gets_routed` (function) — `triage::math_heavy_page_gets_routed`
- `math_threshold_requires_at_least_four_symbols` (function) — `triage::math_threshold_requires_at_least_four_symbols`
- `nearly_empty_page_is_low_density` (function) — `triage::nearly_empty_page_is_low_density`
- `page` (function) — `triage::page`
- `plain_prose_is_not_routed` (function) — `triage::plain_prose_is_not_routed`
- `running_footer_text_is_excluded_from_math_count` (function) — `triage::running_footer_text_is_excluded_from_math_count`
- `scan_report` (function) — `triage::scan_report`
- `scan_report_does_not_flag_normal_prose_document` (function) — `triage::scan_report_does_not_flag_normal_prose_document`
- `scan_report_flags_fully_image_only_document` (function) — `triage::scan_report_flags_fully_image_only_document`
- `should_route` (function) — `triage::should_route`
- `table_present_is_routed` (function) — `triage::table_present_is_routed`
