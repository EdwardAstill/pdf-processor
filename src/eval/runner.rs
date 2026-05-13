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
        options: eval_convert_options(),
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

fn eval_convert_options() -> ConvertOptions {
    ConvertOptions {
        no_images: true,
        figures: FigureMode::None,
        ..ConvertOptions::default()
    }
}
