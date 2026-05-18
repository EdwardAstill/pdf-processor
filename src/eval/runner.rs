use crate::cli::{ConvertArgs, ConvertOptions, FigureMode};
use crate::eval::fixtures::FixtureFile;
use crate::eval::metrics::{aggregate, compute_page_metrics, DocMetrics, PageMetrics};
use crate::pipeline::process_pdf_to_document;

#[derive(Debug, Clone)]
pub struct DocResult {
    pub doc_name: String,
    pub metrics: DocMetrics,
    pub error: Option<String>,
}

pub fn run_eval(fixtures: &[FixtureFile]) -> Vec<DocResult> {
    fixtures.iter().map(run_one).collect()
}

fn run_one(fixture: &FixtureFile) -> DocResult {
    let pdf_path = fixture.pdf_path();
    let doc_name = pdf_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();

    let args = ConvertArgs {
        input: pdf_path.to_string_lossy().into_owned(),
        options: eval_convert_options(&doc_name),
    };

    let document = match process_pdf_to_document(&pdf_path, &args) {
        Ok(document) => document,
        Err(err) => {
            return DocResult {
                doc_name,
                metrics: DocMetrics::default(),
                error: Some(format!("{err:#}")),
            };
        }
    };

    let mut page_metrics: Vec<PageMetrics> = Vec::new();
    for expectation in &fixture.pages {
        let Some(page) = document
            .pages
            .iter()
            .find(|page| page.page_num + 1 == expectation.page)
        else {
            continue;
        };
        page_metrics.push(compute_page_metrics(page, expectation));
    }

    DocResult {
        doc_name,
        metrics: aggregate(&page_metrics),
        error: None,
    }
}

fn eval_convert_options(doc_name: &str) -> ConvertOptions {
    let safe_name = doc_name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let output_dir = std::env::temp_dir()
        .join("pdfp-eval-media")
        .join(std::process::id().to_string())
        .join(safe_name);
    let _ = std::fs::remove_dir_all(&output_dir);

    ConvertOptions {
        output: Some(output_dir),
        no_images: false,
        figures: FigureMode::Snapshot,
        figure_dpi: 96,
        ..ConvertOptions::default()
    }
}
