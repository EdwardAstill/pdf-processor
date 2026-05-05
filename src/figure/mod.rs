pub mod detect;
pub mod render;

pub use detect::{detect_figure_candidates, FigureCandidate, FigureDetectionOptions};
pub use render::render_figure_snapshots;
