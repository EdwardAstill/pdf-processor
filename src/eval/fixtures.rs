use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::document::types::Bbox;

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExpectedHeading {
    pub text: String,
    pub level: u8,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct ExpectedTableRegion {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

impl ExpectedTableRegion {
    pub fn bbox(self) -> Bbox {
        Bbox::new(self.x0, self.y0, self.x1, self.y1)
    }
}

impl From<Bbox> for ExpectedTableRegion {
    fn from(bbox: Bbox) -> Self {
        Self {
            x0: bbox.x0,
            y0: bbox.y0,
            x1: bbox.x1,
            y1: bbox.y1,
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PageExpectation {
    pub page: usize,
    #[serde(default)]
    pub expected_formula_count: usize,
    #[serde(default)]
    pub expected_formula_detection_count: Option<usize>,
    #[serde(default)]
    pub expected_formula_latex_snippets: Vec<String>,
    #[serde(default)]
    pub formula_false_positive_budget: usize,
    #[serde(default)]
    pub expected_headings: Vec<ExpectedHeading>,
    #[serde(default)]
    pub expected_tables: usize,
    #[serde(default)]
    pub expected_table_regions: Vec<ExpectedTableRegion>,
    #[serde(default)]
    pub expected_decorative_images: usize,
    #[serde(default)]
    pub expected_meaningful_figures: usize,
    #[serde(default)]
    pub expected_figure_captions: usize,
    #[serde(default)]
    pub expected_vector_only_regions: usize,
    #[serde(default)]
    pub skip_text_metrics: bool,
    #[serde(default)]
    pub skip_table_metrics: bool,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FixtureFile {
    /// PDF path, relative to the fixture JSON directory unless absolute.
    pub pdf: String,
    pub pages: Vec<PageExpectation>,
    /// Directory containing the fixture JSON. Set by the loader.
    #[serde(skip)]
    pub fixture_dir: PathBuf,
}

impl FixtureFile {
    pub fn pdf_path(&self) -> PathBuf {
        self.fixture_dir.join(&self.pdf)
    }
}

/// Load all `.json` fixture files from `dir` without descending recursively.
pub fn load_fixtures(dir: &Path) -> anyhow::Result<Vec<FixtureFile>> {
    let mut fixtures = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let json = std::fs::read_to_string(&path)?;
        let mut fixture: FixtureFile = serde_json::from_str(&json)
            .map_err(|err| anyhow::anyhow!("fixture parse error {}: {err}", path.display()))?;
        fixture.fixture_dir = path.parent().unwrap_or(dir).to_path_buf();
        fixtures.push(fixture);
    }
    fixtures.sort_by(|a, b| a.pdf.cmp(&b.pdf));
    Ok(fixtures)
}
