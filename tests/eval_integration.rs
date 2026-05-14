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
                    "expected_tables": 0
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
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!((metrics.formula_recall - 0.5).abs() < 0.01);
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
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!((metrics.heading_accuracy - 1.0).abs() < 0.01);
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
    };

    let metrics = compute_page_metrics(&page, &expectation);
    assert!(metrics.table_found);
    assert!(metrics.table_expected);
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
