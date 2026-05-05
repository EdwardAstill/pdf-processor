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
}

#[derive(Args, Debug)]
pub struct InspectArgs {
    /// Input PDF
    pub input: PathBuf,

    /// Emit machine-readable JSON to stdout
    #[arg(long)]
    pub json: bool,
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
    Inspect(InspectArgs),
    Search(SearchArgs),
    Pages(PagesCommand),
    Impose(ImposeCommand),
    Page(PageCommand),
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum HybridPolicy {
    Auto,
    All,
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
