use crate::document::types::{BlockKind, Page};
use crate::eval::fixtures::PageExpectation;

#[derive(Debug, Clone, Default)]
pub struct PageMetrics {
    pub formula_found: usize,
    pub formula_expected: usize,
    pub formula_recall: f32,
    pub heading_matches: usize,
    pub heading_expected: usize,
    pub heading_accuracy: f32,
    pub table_found: bool,
    pub table_expected: bool,
}

#[derive(Debug, Clone, Default)]
pub struct DocMetrics {
    pub pages: usize,
    pub formula_found: usize,
    pub formula_expected: usize,
    pub formula_page_recall_sum: f32,
    pub heading_matches: usize,
    pub heading_expected: usize,
    pub heading_page_accuracy_sum: f32,
    pub table_pages_found: usize,
    pub table_pages_expected: usize,
}

impl DocMetrics {
    pub fn formula_recall(&self) -> f32 {
        ratio_or_full(self.formula_found, self.formula_expected)
    }

    pub fn heading_accuracy(&self) -> f32 {
        ratio_or_full(self.heading_matches, self.heading_expected)
    }

    pub fn table_recall(&self) -> f32 {
        ratio_or_full(self.table_pages_found, self.table_pages_expected)
    }
}

pub fn compute_page_metrics(page: &Page, expected: &PageExpectation) -> PageMetrics {
    let formula_found = page
        .blocks
        .iter()
        .filter(|block| {
            matches!(
                block.kind,
                BlockKind::Formula { .. } | BlockKind::FormulaReview { .. }
            )
        })
        .count();
    let formula_expected = expected.expected_formula_count;

    let heading_matches = expected
        .expected_headings
        .iter()
        .filter(|heading| {
            page.blocks.iter().any(|block| match block.kind {
                BlockKind::Heading { level } => {
                    level == heading.level
                        && block.text.trim().eq_ignore_ascii_case(heading.text.trim())
                }
                _ => false,
            })
        })
        .count();
    let heading_expected = expected.expected_headings.len();

    let table_found = page
        .blocks
        .iter()
        .any(|block| matches!(block.kind, BlockKind::CoordinateTable { .. }));
    let table_expected = expected.expected_tables > 0;

    PageMetrics {
        formula_found,
        formula_expected,
        formula_recall: ratio_or_full(formula_found, formula_expected),
        heading_matches,
        heading_expected,
        heading_accuracy: ratio_or_full(heading_matches, heading_expected),
        table_found,
        table_expected,
    }
}

pub fn aggregate(page_metrics: &[PageMetrics]) -> DocMetrics {
    let mut doc = DocMetrics {
        pages: page_metrics.len(),
        ..Default::default()
    };
    for metrics in page_metrics {
        doc.formula_found += metrics.formula_found.min(metrics.formula_expected);
        doc.formula_expected += metrics.formula_expected;
        doc.formula_page_recall_sum += metrics.formula_recall;
        doc.heading_matches += metrics.heading_matches.min(metrics.heading_expected);
        doc.heading_expected += metrics.heading_expected;
        doc.heading_page_accuracy_sum += metrics.heading_accuracy;
        if metrics.table_expected {
            doc.table_pages_expected += 1;
            if metrics.table_found {
                doc.table_pages_found += 1;
            }
        }
    }
    doc
}

pub fn print_report(doc_name: &str, doc: &DocMetrics) {
    println!(
        "{doc_name}\n  pages evaluated:   {}\n  formula recall:    {:.1}% ({}/{})\n  heading accuracy:  {:.1}% ({}/{})\n  table recall:      {:.1}% ({}/{})\n",
        doc.pages,
        100.0 * doc.formula_recall(),
        doc.formula_found.min(doc.formula_expected),
        doc.formula_expected,
        100.0 * doc.heading_accuracy(),
        doc.heading_matches.min(doc.heading_expected),
        doc.heading_expected,
        100.0 * doc.table_recall(),
        doc.table_pages_found,
        doc.table_pages_expected,
    );
}

fn ratio_or_full(found: usize, expected: usize) -> f32 {
    if expected == 0 {
        1.0
    } else {
        found.min(expected) as f32 / expected as f32
    }
}
