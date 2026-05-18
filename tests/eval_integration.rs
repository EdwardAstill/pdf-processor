use pdf_processor::document::types::{Bbox, Block, BlockKind, DetectedTable, Page, TableRender};
use pdf_processor::eval::fixtures::{load_fixtures, ExpectedHeading, FixtureFile, PageExpectation};
use pdf_processor::eval::metrics::compute_page_metrics;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bbox() -> Bbox {
    Bbox {
        x0: 0.0,
        y0: 0.0,
        x1: 100.0,
        y1: 20.0,
    }
}

fn make_block(kind: BlockKind, text: &str) -> Block {
    Block {
        id: 0,
        bbox: bbox(),
        text: text.to_string(),
        kind,
        font_size: 12.0,
        font_name: "Times".to_string(),
        page_num: 0,
        reading_order: 0,
        bold: false,
        italic: false,
    }
}

fn make_table_block(bbox: Bbox) -> Block {
    Block {
        id: 0,
        bbox,
        text: String::new(),
        kind: BlockKind::CoordinateTable {
            table: DetectedTable {
                bbox,
                rows: vec![vec!["A".to_string(), "B".to_string()]],
                confidence: 0.9,
                render: TableRender::Markdown,
            },
        },
        font_size: 12.0,
        font_name: "table".to_string(),
        page_num: 0,
        reading_order: 0,
        bold: false,
        italic: false,
    }
}

fn make_image_block(bbox: Bbox) -> Block {
    make_block(
        BlockKind::Image {
            path: Some("images/logo.png".to_string()),
        },
        "",
    )
    .with_bbox(bbox)
}

fn make_figure_block(bbox: Bbox, caption: Option<&str>) -> Block {
    make_block(
        BlockKind::Figure {
            path: Some("images/figure.png".to_string()),
            caption: caption.map(str::to_string),
        },
        "",
    )
    .with_bbox(bbox)
}

trait TestBlockExt {
    fn with_bbox(self, bbox: Bbox) -> Self;
}

impl TestBlockExt for Block {
    fn with_bbox(mut self, bbox: Bbox) -> Self {
        self.bbox = bbox;
        self
    }
}

fn make_page(blocks: Vec<Block>) -> Page {
    Page {
        page_num: 0,
        width: 612.0,
        height: 792.0,
        blocks,
        override_markdown: None,
    }
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdfp"))
}

#[test]
fn load_fixtures_parses_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(
        dir.path().join("sample.json"),
        r#"{
            "pdf": "sample.pdf",
            "pages": [
                {
                    "page": 1,
                    "expected_formula_count": 2,
                    "expected_headings": [{"text": "Introduction", "level": 1}],
                    "expected_tables": 0,
                    "expected_table_regions": [
                        {"x0": 40.0, "y0": 100.0, "x1": 500.0, "y1": 180.0}
                    ]
                }
            ]
        }"#,
    )
    .unwrap();

    let fixtures = load_fixtures(dir.path()).expect("load fixtures");
    assert_eq!(fixtures.len(), 1);
    assert_eq!(fixtures[0].pdf_path(), dir.path().join("sample.pdf"));
    assert_eq!(fixtures[0].pages[0].expected_formula_count, 2);
    assert_eq!(
        fixtures[0].pages[0].expected_headings[0].text,
        "Introduction"
    );
    assert_eq!(fixtures[0].pages[0].expected_headings[0].level, 1);
    assert_eq!(fixtures[0].pages[0].expected_table_regions.len(), 1);
    assert_eq!(fixtures[0].pages[0].expected_table_regions[0].x0, 40.0);
}

#[test]
fn load_fixtures_skips_non_json_files() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("README.md"), "# readme").unwrap();

    let fixtures = load_fixtures(dir.path()).expect("load fixtures");
    assert!(fixtures.is_empty());
}

#[test]
fn formula_recall_counts_formula_blocks() {
    let page = make_page(vec![
        make_block(
            BlockKind::Formula {
                latex: "E = mc^2".to_string(),
                display: true,
            },
            "",
        ),
        make_block(BlockKind::Paragraph, "Normal text"),
    ]);
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 2,
        expected_headings: vec![],
        expected_tables: 0,
        expected_table_regions: vec![],
        expected_decorative_images: 0,
        expected_meaningful_figures: 0,
        expected_figure_captions: 0,
        expected_vector_only_regions: 0,
        skip_text_metrics: false,
        skip_table_metrics: false,
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!((metrics.formula_recall - 0.5).abs() < 0.01);
    assert!((metrics.formula_precision - 1.0).abs() < 0.01);
    assert_eq!(metrics.formula_false_positives, 0);
}

#[test]
fn formula_precision_counts_extra_formula_blocks_as_false_positives() {
    let page = make_page(vec![
        make_block(
            BlockKind::Formula {
                latex: "E = mc^2".to_string(),
                display: true,
            },
            "",
        ),
        make_block(
            BlockKind::Formula {
                latex: "F = ma".to_string(),
                display: true,
            },
            "",
        ),
    ]);
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 1,
        expected_headings: vec![],
        expected_tables: 0,
        expected_table_regions: vec![],
        expected_decorative_images: 0,
        expected_meaningful_figures: 0,
        expected_figure_captions: 0,
        expected_vector_only_regions: 0,
        skip_text_metrics: false,
        skip_table_metrics: false,
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!((metrics.formula_recall - 1.0).abs() < 0.01);
    assert!((metrics.formula_precision - 0.5).abs() < 0.01);
    assert_eq!(metrics.formula_false_positives, 1);
}

#[test]
fn heading_accuracy_counts_exact_matches() {
    let page = make_page(vec![
        make_block(BlockKind::Heading { level: 1 }, "Introduction"),
        make_block(BlockKind::Heading { level: 2 }, "Background"),
        make_block(BlockKind::Heading { level: 1 }, "Wrong level"),
    ]);
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 0,
        expected_headings: vec![
            ExpectedHeading {
                text: "Introduction".to_string(),
                level: 1,
            },
            ExpectedHeading {
                text: "Background".to_string(),
                level: 2,
            },
        ],
        expected_tables: 0,
        expected_table_regions: vec![],
        expected_decorative_images: 0,
        expected_meaningful_figures: 0,
        expected_figure_captions: 0,
        expected_vector_only_regions: 0,
        skip_text_metrics: false,
        skip_table_metrics: false,
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!((metrics.heading_accuracy - 1.0).abs() < 0.01);
    assert!((metrics.heading_precision - (2.0 / 3.0)).abs() < 0.01);
    assert_eq!(metrics.heading_false_positives, 1);
}

#[test]
fn table_found_when_coordinate_table_present() {
    let page = make_page(vec![make_block(
        BlockKind::CoordinateTable {
            table: DetectedTable {
                bbox: bbox(),
                rows: vec![vec!["A".to_string(), "B".to_string()]],
                confidence: 0.9,
                render: TableRender::Markdown,
            },
        },
        "",
    )]);
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 0,
        expected_headings: vec![],
        expected_tables: 1,
        expected_table_regions: vec![],
        expected_decorative_images: 0,
        expected_meaningful_figures: 0,
        expected_figure_captions: 0,
        expected_vector_only_regions: 0,
        skip_text_metrics: false,
        skip_table_metrics: false,
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!(metrics.table_found);
    assert!(metrics.table_expected);
    assert!(metrics.table_true_positive);
    assert!(!metrics.table_false_positive);
}

#[test]
fn table_precision_counts_unexpected_table_page_as_false_positive() {
    let page = make_page(vec![make_block(
        BlockKind::CoordinateTable {
            table: DetectedTable {
                bbox: bbox(),
                rows: vec![vec!["A".to_string(), "B".to_string()]],
                confidence: 0.9,
                render: TableRender::Markdown,
            },
        },
        "",
    )]);
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 0,
        expected_headings: vec![],
        expected_tables: 0,
        expected_table_regions: vec![],
        expected_decorative_images: 0,
        expected_meaningful_figures: 0,
        expected_figure_captions: 0,
        expected_vector_only_regions: 0,
        skip_text_metrics: false,
        skip_table_metrics: false,
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!(metrics.table_found);
    assert!(!metrics.table_expected);
    assert!(!metrics.table_true_positive);
    assert!(metrics.table_false_positive);
}

#[test]
fn table_region_precision_counts_extra_broad_table_as_false_positive() {
    let tight = Bbox::new(40.0, 100.0, 500.0, 180.0);
    let broad = Bbox::new(30.0, 20.0, 560.0, 760.0);
    let page = make_page(vec![make_table_block(tight), make_table_block(broad)]);
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 0,
        expected_headings: vec![],
        expected_tables: 1,
        expected_table_regions: vec![tight.into()],
        expected_decorative_images: 0,
        expected_meaningful_figures: 0,
        expected_figure_captions: 0,
        expected_vector_only_regions: 0,
        skip_text_metrics: false,
        skip_table_metrics: false,
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert_eq!(metrics.table_regions_found, 2);
    assert_eq!(metrics.table_regions_expected, 1);
    assert_eq!(metrics.table_region_matches, 1);
    assert_eq!(metrics.table_region_false_positives, 1);
    assert!((metrics.table_region_precision - 0.5).abs() < 0.01);
    assert!((metrics.table_region_recall - 1.0).abs() < 0.01);
}

#[test]
fn image_metrics_count_decorative_suppression_and_caption_pairing() {
    let page = make_page(vec![
        make_image_block(Bbox::new(470.0, 24.0, 540.0, 72.0)),
        make_figure_block(
            Bbox::new(90.0, 140.0, 510.0, 320.0),
            Some("Figure 1: Model overview"),
        ),
        make_figure_block(Bbox::new(80.0, 360.0, 520.0, 520.0), None),
    ]);
    let expectation = PageExpectation {
        page: 1,
        expected_formula_count: 0,
        expected_headings: vec![],
        expected_tables: 0,
        expected_table_regions: vec![],
        expected_decorative_images: 2,
        expected_meaningful_figures: 3,
        expected_figure_captions: 2,
        expected_vector_only_regions: 1,
        skip_text_metrics: false,
        skip_table_metrics: false,
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert_eq!(metrics.decorative_images_expected, 2);
    assert_eq!(metrics.decorative_images_emitted, 1);
    assert_eq!(metrics.decorative_images_suppressed, 1);
    assert!((metrics.decorative_image_suppression_rate - 0.5).abs() < 0.01);
    assert_eq!(metrics.meaningful_figures_found, 2);
    assert_eq!(metrics.meaningful_figures_expected, 3);
    assert_eq!(metrics.meaningful_figure_matches, 2);
    assert!((metrics.meaningful_figure_retention_rate - (2.0 / 3.0)).abs() < 0.01);
    assert_eq!(metrics.figure_caption_pairs_found, 1);
    assert_eq!(metrics.figure_caption_pairs_expected, 2);
    assert!((metrics.figure_caption_pairing_rate - 0.5).abs() < 0.01);
    assert_eq!(metrics.vector_only_regions_expected, 1);
    assert_eq!(metrics.vector_only_regions_acknowledged, 1);
}

#[test]
fn runner_returns_error_for_missing_fixture_pdf_without_panicking() {
    use pdf_processor::eval::runner::run_eval;

    let dir = tempfile::tempdir().unwrap();
    let fixtures = vec![FixtureFile {
        pdf: "missing.pdf".to_string(),
        pages: vec![PageExpectation {
            page: 1,
            expected_formula_count: 0,
            expected_headings: vec![],
            expected_tables: 0,
            expected_table_regions: vec![],
            expected_decorative_images: 0,
            expected_meaningful_figures: 0,
            expected_figure_captions: 0,
            expected_vector_only_regions: 0,
            skip_text_metrics: false,
            skip_table_metrics: false,
        }],
        fixture_dir: dir.path().to_path_buf(),
    }];

    let results = run_eval(&fixtures);
    assert_eq!(results.len(), 1);
    assert!(results[0].error.is_some());
}

#[test]
fn eval_command_errors_on_missing_dir() {
    let output = Command::new(bin_path())
        .args(["eval", "/tmp/this_dir_does_not_exist_pdfp_eval_test"])
        .output()
        .unwrap();

    assert!(!output.status.success());
}

#[test]
fn tracked_fixture_json_files_are_valid() {
    let dir = Path::new("tests/eval_fixtures");
    let mut checked = 0usize;
    for entry in std::fs::read_dir(dir).expect("read eval fixtures dir") {
        let path = entry.expect("fixture entry").path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let json = std::fs::read_to_string(&path).expect("read fixture json");
        let _: FixtureFile = serde_json::from_str(&json)
            .unwrap_or_else(|err| panic!("{} must be valid: {err}", path.display()));
        checked += 1;
    }
    assert!(checked >= 3, "expected sample and local baseline fixtures");
}
