pub mod detect;
pub mod geometric;
pub mod ocr;
#[cfg(feature = "onnx-ocr")]
pub mod ocr_onnx;
pub mod visual;

pub use detect::{detect_formula_candidates, FormulaCandidate};
pub use visual::detect_visual_formula_candidates;
