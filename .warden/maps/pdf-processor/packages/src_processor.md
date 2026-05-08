# Package: src/processor


## `src/processor/doctor.rs`

- `<module>` (module) — `src/processor/doctor.rs:<module>`
- `DoctorReport` (type) — `doctor::DoctorReport`
- `PdfpStatus` (type) — `doctor::PdfpStatus`
- `run` (function) — `doctor::run`

## `src/processor/impose.rs`

- `<module>` (module) — `src/processor/impose.rs:<module>`
- `booklet` (function) — `impose::booklet`
- `booklet_order_pads_to_multiple_of_four` (function) — `impose::booklet_order_pads_to_multiple_of_four`
- `ensure_parent_dir` (function) — `impose::ensure_parent_dir`
- `open_doc` (function) — `impose::open_doc`
- `page_if_real` (function) — `impose::page_if_real`
- `run` (function) — `impose::run`
- `two_up` (function) — `impose::two_up`
- `write_two_up` (function) — `impose::write_two_up`

## `src/processor/inspect.rs`

- `<module>` (module) — `src/processor/inspect.rs:<module>`
- `InspectReport` (type) — `inspect::InspectReport`
- `MIN_TEXT_AREA_FRACTION` (constant) — `inspect::MIN_TEXT_AREA_FRACTION`
- `PageReport` (type) — `inspect::PageReport`
- `inspect_pdf` (function) — `inspect::inspect_pdf`
- `metadata_from_open_document` (function) — `inspect::metadata_from_open_document`
- `print_human_report` (function) — `inspect::print_human_report`
- `run` (function) — `inspect::run`
- `scan_like_requires_no_readable_text` (function) — `inspect::scan_like_requires_no_readable_text`

## `src/processor/mod.rs`

- `<module>` (module) — `src/processor/mod.rs:<module>`

## `src/processor/ocr_cmd.rs`

- `<module>` (module) — `src/processor/ocr_cmd.rs:<module>`
- `run` (function) — `ocr_cmd::run`

## `src/processor/page_range.rs`

- `<module>` (module) — `src/processor/page_range.rs:<module>`
- `parse_one_indexed` (function) — `page_range::parse_one_indexed`
- `parse_page_selection` (function) — `page_range::parse_page_selection`
- `parses_all_odd_and_even` (function) — `page_range::parses_all_odd_and_even`
- `parses_single_pages_and_ranges` (function) — `page_range::parses_single_pages_and_ranges`
- `push_page` (function) — `page_range::push_page`
- `push_range` (function) — `page_range::push_range`
- `rejects_descending_ranges` (function) — `page_range::rejects_descending_ranges`
- `rejects_out_of_range_pages` (function) — `page_range::rejects_out_of_range_pages`

## `src/processor/pages.rs`

- `<module>` (module) — `src/processor/pages.rs:<module>`
- `absolutize` (function) — `pages::absolutize`
- `delete` (function) — `pages::delete`
- `ensure_output_is_not_input` (function) — `pages::ensure_output_is_not_input`
- `extract` (function) — `pages::extract`
- `merge` (function) — `pages::merge`
- `page_count` (function) — `pages::page_count`
- `refuses_same_input_and_output_path` (function) — `pages::refuses_same_input_and_output_path`
- `reorder` (function) — `pages::reorder`
- `run` (function) — `pages::run`
- `split` (function) — `pages::split`
- `write_copied_pages` (function) — `pages::write_copied_pages`
- `write_selected_pages` (function) — `pages::write_selected_pages`

## `src/processor/resize.rs`

- `<module>` (module) — `src/processor/resize.rs:<module>`
- `FitMode` (type) — `resize::FitMode`
- `knows_a4_dimensions` (function) — `resize::knows_a4_dimensions`
- `paper_size` (function) — `resize::paper_size`
- `parse` (function) — `resize::parse`
- `parses_fit_modes` (function) — `resize::parses_fit_modes`
- `resize` (function) — `resize::resize`
- `run` (function) — `resize::run`

## `src/processor/search.rs`

- `<module>` (module) — `src/processor/search.rs:<module>`
- `PointReport` (type) — `search::PointReport`
- `QuadReport` (type) — `search::QuadReport`
- `SearchPageMatch` (type) — `search::SearchPageMatch`
- `SearchReport` (type) — `search::SearchReport`
- `point_report` (function) — `search::point_report`
- `print_human_report` (function) — `search::print_human_report`
- `quad_report` (function) — `search::quad_report`
- `quad_report_preserves_points` (function) — `search::quad_report_preserves_points`
- `run` (function) — `search::run`
- `search_pdf` (function) — `search::search_pdf`
