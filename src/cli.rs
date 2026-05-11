use clap::{Args, Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "pdfp", about = "Local PDF processor", version)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Input PDF: file path, directory, or glob pattern.
    ///
    /// Backwards-compatible alias for `pdfp convert <INPUT>`.
    pub input: Option<String>,

    #[command(flatten)]
    pub convert_options: ConvertOptions,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Convert PDF files into markdown
    Convert(ConvertArgs),
    /// Create a searchable OCR PDF
    Ocr(OcrArgs),
    /// Check installed and bundled runtime dependencies
    Doctor(DoctorArgs),
    /// Inspect PDF metadata, page sizes, and scan-like signals
    Inspect(InspectArgs),
    /// Search PDF text and report matching pages
    Search(SearchArgs),
    /// Page extraction, deletion, splitting, reordering, and merging
    Pages(PagesCommand),
    /// Print imposition operations such as 2-up and booklet output
    Impose(ImposeCommand),
    /// Page-level geometry operations
    Page(PageCommand),
}

#[derive(Args, Debug, Clone)]
pub struct ConvertArgs {
    /// Input PDF: file path, directory, or glob pattern
    pub input: String,

    #[command(flatten)]
    pub options: ConvertOptions,
}

#[derive(Args, Debug, Clone)]
pub struct ConvertOptions {
    /// Output directory (default: next to input file)
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Minimum vertical gap for horizontal cuts in points (PDF XY-Cut tuning)
    #[arg(long, default_value = "8.0")]
    pub min_h_gap: f32,

    /// Minimum horizontal gap for vertical cuts in points (PDF XY-Cut tuning)
    #[arg(long, default_value = "12.0")]
    pub min_v_gap: f32,

    /// Skip image extraction
    #[arg(long)]
    pub no_images: bool,

    /// Prefer audit/fallback output over heuristic reconstruction.
    ///
    /// Conservative mode avoids speculative Markdown tables, formula rendering,
    /// and rendered figure snapshots. It is a preset for review-safe conversion;
    /// debug flags may still be used to inspect candidates.
    #[arg(long)]
    pub conservative: bool,

    /// Figure/image output mode for markdown conversion
    #[arg(long, value_enum, default_value = "embedded")]
    pub figures: FigureMode,

    /// Resolution for rendered figure snapshots
    #[arg(long, default_value = "200")]
    pub figure_dpi: u32,

    /// Padding around detected figure regions, in PDF points
    #[arg(long, default_value = "8.0")]
    pub figure_padding: f32,

    /// Write figure candidate debug JSON under debug/figures/
    #[arg(long)]
    pub debug_figures: bool,

    /// Table extraction mode for markdown conversion
    #[arg(long, value_enum, default_value = "auto")]
    pub tables: TableMode,

    /// Write table detection debug JSON under debug/tables/
    #[arg(long)]
    pub debug_tables: bool,

    /// Formula handling mode for markdown conversion
    #[arg(long, value_enum, default_value = "auto")]
    pub formulas: FormulaMode,

    /// Write formula detection debug JSON and crops under debug/formulas/
    #[arg(long)]
    pub debug_formulas: bool,

    /// Optional formula OCR sidecar command. Receives a crop PNG path and prints LaTeX to stdout.
    #[arg(long, value_name = "CMD")]
    pub formula_sidecar: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Route PDFs through an external backend for higher-quality extraction
    /// (LaTeX formulas, complex tables, OCR). `off` (default) = fully local;
    /// `docling` = POST the whole PDF to a running `docling-serve` instance.
    #[arg(long, value_enum, default_value = "off")]
    pub hybrid: HybridMode,

    /// Base URL of the hybrid backend (docling-serve). Only used when
    /// `--hybrid` is not `off`.
    #[arg(long, default_value = "http://localhost:5001")]
    pub hybrid_url: String,

    /// Timeout in seconds for the hybrid backend call. Large scanned PDFs on
    /// CPU can take minutes.
    #[arg(long, default_value = "600")]
    pub hybrid_timeout_secs: u64,

    /// Which pages to route through the hybrid backend. `auto` (default)
    /// triages per page based on math-symbol count, table presence, and
    /// text density — only formula-/table-/scan-heavy pages pay the
    /// backend cost. `all` routes every page (useful for testing).
    #[arg(long, value_enum, default_value = "auto")]
    pub hybrid_policy: HybridPolicy,

    /// Optional directory for cached hybrid markdown, keyed by source PDF
    /// metadata and page number.
    #[arg(long)]
    pub hybrid_cache_dir: Option<PathBuf>,

    #[command(flatten)]
    pub ocr: OcrOptions,
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Input PDF
    pub input: PathBuf,

    /// Emit machine-readable JSON to stdout
    #[arg(long)]
    pub json: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[command(flatten)]
    pub ocr: OcrOptions,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Input PDF
    pub input: PathBuf,

    /// Text to search for
    pub needle: String,

    /// Optional 1-indexed page range to search, e.g. `1-3,9`
    #[arg(long)]
    pub pages: Option<String>,

    /// Emit machine-readable JSON to stdout
    #[arg(long)]
    pub json: bool,

    /// Number of surrounding characters to include in human output.
    ///
    /// The first implementation reports page numbers and hit boxes; context is
    /// accepted now so the CLI contract does not need to change later.
    #[arg(long, default_value = "0")]
    pub context: usize,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,

    #[command(flatten)]
    pub ocr: OcrOptions,
}

#[derive(Args, Debug)]
pub struct OcrArgs {
    /// Input PDF
    pub input: PathBuf,

    /// Output searchable PDF
    #[arg(short, long)]
    pub output: PathBuf,

    /// OCR mode. `auto` skips born-digital/readable pages; `force` rasterizes
    /// all pages and rebuilds the text layer.
    #[arg(long, value_enum, default_value = "auto")]
    pub mode: StandaloneOcrMode,

    /// OCR language(s), passed to OCRmyPDF/Tesseract, e.g. `eng` or `eng+deu`.
    #[arg(long, alias = "ocr-lang", default_value = "eng")]
    pub lang: String,

    /// Optional cache directory for derived searchable PDFs.
    #[arg(long, alias = "ocr-cache-dir")]
    pub cache_dir: Option<PathBuf>,

    /// Timeout in seconds for OCR preprocessing.
    #[arg(long, alias = "ocr-timeout-secs", default_value = "600")]
    pub timeout_secs: u64,

    /// OCRmyPDF executable path or command name.
    #[arg(long, alias = "ocr-command", default_value = "ocrmypdf")]
    pub command: PathBuf,

    /// Emit machine-readable OCR decision JSON to stdout
    #[arg(long)]
    pub json: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

#[derive(Args, Debug)]
pub struct DoctorArgs {
    /// Emit machine-readable JSON to stdout
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct PagesCommand {
    #[command(subcommand)]
    pub command: PagesSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum PagesSubcommand {
    /// Extract selected pages into a new PDF
    Extract(PageSelectionArgs),
    /// Delete selected pages and write a new PDF
    Delete(PageSelectionArgs),
    /// Split a PDF into chunks
    Split(SplitArgs),
    /// Reorder pages into a new PDF
    Reorder(PageSelectionArgs),
    /// Merge PDFs
    Merge(MergeArgs),
}

#[derive(Args, Debug)]
pub struct MergeArgs {
    /// Input PDFs, in output order
    #[arg(required = true, num_args = 1..)]
    pub inputs: Vec<PathBuf>,

    /// Output PDF
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct PageSelectionArgs {
    /// Input PDF
    pub input: PathBuf,

    /// 1-indexed page range, e.g. `1-3,9`, `odd`, `even`, or `all`
    #[arg(long)]
    pub pages: String,

    /// Output PDF
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct SplitArgs {
    /// Input PDF
    pub input: PathBuf,

    /// Number of pages per output chunk
    #[arg(long)]
    pub every: usize,

    /// Output directory
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct ImposeCommand {
    #[command(subcommand)]
    pub command: ImposeSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum ImposeSubcommand {
    /// Compose two source pages onto one output page
    #[command(name = "2up")]
    TwoUp(ImposeArgs),
    /// Create booklet imposition output
    Booklet(ImposeArgs),
}

#[derive(Args, Debug)]
pub struct ImposeArgs {
    /// Input PDF
    pub input: PathBuf,

    /// Output PDF
    #[arg(short, long)]
    pub output: PathBuf,
}

#[derive(Args, Debug)]
pub struct PageCommand {
    #[command(subcommand)]
    pub command: PageSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum PageSubcommand {
    /// Resize pages to a target paper size
    Resize(ResizeArgs),
}

#[derive(Args, Debug)]
pub struct ResizeArgs {
    /// Input PDF
    pub input: PathBuf,

    /// Output PDF
    #[arg(short, long)]
    pub output: PathBuf,

    /// Target paper size
    #[arg(long, default_value = "a4")]
    pub paper: String,

    /// Fit mode: contain, cover, or stretch
    #[arg(long, default_value = "contain")]
    pub fit: String,
}

impl Cli {
    pub fn into_command(self) -> anyhow::Result<AppCommand> {
        match self.command {
            Some(Command::Convert(args)) => Ok(AppCommand::Convert(args)),
            Some(Command::Ocr(args)) => Ok(AppCommand::Ocr(args)),
            Some(Command::Doctor(args)) => Ok(AppCommand::Doctor(args)),
            Some(Command::Inspect(args)) => Ok(AppCommand::Inspect(args)),
            Some(Command::Search(args)) => Ok(AppCommand::Search(args)),
            Some(Command::Pages(args)) => Ok(AppCommand::Pages(args)),
            Some(Command::Impose(args)) => Ok(AppCommand::Impose(args)),
            Some(Command::Page(args)) => Ok(AppCommand::Page(args)),
            None => {
                let input = self.input.ok_or_else(|| {
                    anyhow::anyhow!(
                        "missing input; use `pdfp <INPUT>` or a subcommand such as `pdfp inspect <INPUT>`"
                    )
                })?;
                Ok(AppCommand::Convert(ConvertArgs {
                    input,
                    options: self.convert_options,
                }))
            }
        }
    }
}

#[derive(Debug)]
pub enum AppCommand {
    Convert(ConvertArgs),
    Ocr(OcrArgs),
    Doctor(DoctorArgs),
    Inspect(InspectArgs),
    Search(SearchArgs),
    Pages(PagesCommand),
    Impose(ImposeCommand),
    Page(PageCommand),
}

#[derive(Args, Debug, Clone)]
pub struct OcrOptions {
    /// Local OCR preprocessing mode. `auto` OCRs scan-heavy PDFs only; `force`
    /// OCRs regardless of readable text; `off` keeps the current fast path.
    #[arg(long, value_enum, default_value = "off")]
    pub ocr: OcrMode,

    /// OCR language(s), passed to OCRmyPDF/Tesseract, e.g. `eng` or `eng+deu`.
    #[arg(long, default_value = "eng")]
    pub ocr_lang: String,

    /// Optional cache directory for derived searchable PDFs.
    #[arg(long)]
    pub ocr_cache_dir: Option<PathBuf>,

    /// Timeout in seconds for OCR preprocessing.
    #[arg(long, default_value = "600")]
    pub ocr_timeout_secs: u64,

    /// OCRmyPDF executable path or command name.
    #[arg(long, default_value = "ocrmypdf")]
    pub ocr_command: PathBuf,
}

impl Default for OcrOptions {
    fn default() -> Self {
        Self {
            ocr: OcrMode::Off,
            ocr_lang: "eng".to_string(),
            ocr_cache_dir: None,
            ocr_timeout_secs: 600,
            ocr_command: PathBuf::from("ocrmypdf"),
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum OcrMode {
    Off,
    Auto,
    Force,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FigureMode {
    /// Current behavior: extract embedded raster image objects
    Embedded,
    /// Render complete detected figure regions as page snapshots
    Snapshot,
    /// Emit both rendered figure snapshots and embedded image objects
    Both,
    /// Do not emit image or figure assets
    None,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableMode {
    /// Detect tables automatically; emit Markdown when confident, layout text otherwise
    Auto,
    /// Force native coordinate-derived Markdown tables
    Native,
    /// Preserve detected table regions as fenced fixed-width layout text
    Layout,
    /// Disable coordinate table reconstruction
    Off,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormulaMode {
    /// Detect formula candidates for warnings and debug audit files
    Auto,
    /// Force local formula candidate detection and rendering
    Local,
    /// Detect formula candidates for hybrid backend routing and audit
    Hybrid,
    /// Disable formula candidate detection
    Off,
}

impl ConvertOptions {
    pub fn effective_figure_mode(&self) -> FigureMode {
        if self.conservative {
            FigureMode::Embedded
        } else {
            self.figures
        }
    }

    pub fn effective_table_mode(&self) -> TableMode {
        if self.conservative {
            TableMode::Layout
        } else {
            self.tables
        }
    }

    pub fn effective_formula_mode(&self) -> FormulaMode {
        if self.conservative {
            FormulaMode::Auto
        } else {
            self.formulas
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum StandaloneOcrMode {
    Auto,
    Force,
}

impl From<StandaloneOcrMode> for OcrMode {
    fn from(mode: StandaloneOcrMode) -> Self {
        match mode {
            StandaloneOcrMode::Auto => OcrMode::Auto,
            StandaloneOcrMode::Force => OcrMode::Force,
        }
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridPolicy {
    Auto,
    All,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert_options(conservative: bool) -> ConvertOptions {
        ConvertOptions {
            output: None,
            min_h_gap: 8.0,
            min_v_gap: 12.0,
            no_images: false,
            conservative,
            figures: FigureMode::Snapshot,
            figure_dpi: 200,
            figure_padding: 8.0,
            debug_figures: false,
            tables: TableMode::Native,
            debug_tables: false,
            formulas: FormulaMode::Local,
            debug_formulas: false,
            formula_sidecar: None,
            verbose: false,
            hybrid: HybridMode::Off,
            hybrid_url: "http://localhost:5001".to_string(),
            hybrid_timeout_secs: 600,
            hybrid_policy: HybridPolicy::Auto,
            hybrid_cache_dir: None,
            ocr: OcrOptions::default(),
        }
    }

    #[test]
    fn conservative_mode_uses_review_safe_conversion_modes() {
        let options = convert_options(true);

        assert_eq!(options.effective_figure_mode(), FigureMode::Embedded);
        assert_eq!(options.effective_table_mode(), TableMode::Layout);
        assert_eq!(options.effective_formula_mode(), FormulaMode::Auto);
    }

    #[test]
    fn non_conservative_mode_preserves_selected_conversion_modes() {
        let options = convert_options(false);

        assert_eq!(options.effective_figure_mode(), FigureMode::Snapshot);
        assert_eq!(options.effective_table_mode(), TableMode::Native);
        assert_eq!(options.effective_formula_mode(), FormulaMode::Local);
    }
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridMode {
    Off,
    Docling,
}

impl HybridMode {
    pub fn is_on(self) -> bool {
        !matches!(self, HybridMode::Off)
    }
}

/// Detected input type based on file extension.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputType {
    Pdf,
}

#[allow(dead_code)]
impl InputType {
    /// Detect input type from file extension.
    pub fn from_path(path: &std::path::Path) -> Option<Self> {
        let ext = path.extension()?.to_str()?.to_lowercase();
        match ext.as_str() {
            "pdf" => Some(Self::Pdf),
            _ => None,
        }
    }

    /// File extensions associated with this input type.
    pub fn extensions(&self) -> &[&str] {
        match self {
            Self::Pdf => &["pdf"],
        }
    }
}

/// All file extensions that pdfp supports.
pub const SUPPORTED_EXTENSIONS: &[&str] = &["pdf"];
