use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct ExpectedHeading {
    pub text: String,
    pub level: u8,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
pub struct PageExpectation {
    pub page: usize,
    #[serde(default)]
    pub expected_formula_count: usize,
    #[serde(default)]
    pub expected_headings: Vec<ExpectedHeading>,
    #[serde(default)]
    pub expected_tables: usize,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
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
