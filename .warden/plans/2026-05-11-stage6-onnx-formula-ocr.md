# Stage 6: Native Formula OCR via ONNX

> **For agentic workers:** REQUIRED SUB-SKILL: Use subagent-driven-development to implement this plan task-by-task when tasks are independent. For same-session manual execution, follow this plan directly. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the Python subprocess sidecar (Stage 3) with a native Rust ONNX inference path using the `ort` crate and a downloaded RapidLaTeX-OCR encoder/decoder model, eliminating the Python runtime dependency.

**Architecture:** A new Cargo feature `onnx-ocr` gates all ONNX code. Behind the feature, `src/formula/ocr_onnx.rs` provides `OnnxFormulaSidecar`, implementing the `FormulaSidecar` trait from Stage 3. The sidecar loads two ONNX sessions at startup (encoder and decoder), preprocesses a formula crop PNG to a fixed 192×672 tensor, runs the encoder, then greedily decodes token IDs using the decoder session until EOS or a 512-token limit. A bundled vocabulary file maps token IDs to LaTeX strings. The existing `SubprocessSidecar` (Stage 3) remains available. `--formula-sidecar onnx:<model-dir>` selects ONNX, `--formula-sidecar cmd:<command>` selects an explicit subprocess, and the current plain `--formula-sidecar <command>` behavior remains supported for compatibility. Without any `--formula-sidecar` flag, formula crops continue emitting `<!-- formula-review -->` markers.

**Tech Stack:** Rust, `ort` 2.0 (ONNX Runtime), `image` crate, `ndarray` (ort dependency), RapidLaTeX-OCR ONNX models (downloaded separately)

**Recommended Skills:** test-driven-development, git

**Recommended MCPs:** none

**Status:** implemented
**Refinement passes:** 1

## Post-Implementation Review

Updated: 2026-05-13

Acceptance results:

- Default build/tests: `cargo test` passes.
- Feature build/tests: `cargo test --features onnx-ocr` passes.
- Lints: `cargo clippy --all-targets -- -D warnings` and `cargo clippy --features onnx-ocr --all-targets -- -D warnings` pass.
- Subprocess compatibility: bare `--formula-sidecar <command>` still passes existing Stage 3 tests; `cmd:<command>` parser coverage added.
- ONNX CLI path: `onnx:<model-dir>` parser and model-dir validation covered; real model end-to-end not run because RapidLaTeX-OCR model files are not present locally.

Scope drift:

- `ort` `load-dynamic` was not used. `ort` 2.0.0-rc.12 compiles provider registration for `load-dynamic` against generated bindings that lack `SessionOptionsAppendExecutionProvider_VitisAI`. The implementation uses `download-binaries`, `copy-dylibs`, and `tls-rustls` instead.
- `src/lib.rs` now exports `cli` so integration tests can cover sidecar parser behavior.

Refactor proposals:

- Add a model-fixture-driven ignored test once encoder/decoder/vocab files are available in local test assets.
- Confirm RapidLaTeX-OCR decoder input order against downloaded model metadata before treating ONNX recognition quality as accepted.

## Assumptions

- `A1` — Stage 3 (`FormulaSidecar` trait, `SubprocessSidecar`, `--formula-sidecar` CLI flag) has been implemented and merged.
  Type: repo-state
  Source: Stage 3 plan: `.warden/plans/2026-05-11-stage3-formula-ocr-sidecar.md`
  Check: `grep -n "FormulaSidecar\|SubprocessSidecar" src/formula/ocr.rs`
  If false: implement Stage 3 first.
  Owner: Task 1

- `A2` — RapidLaTeX-OCR publishes ONNX model files. The encoder input is a single float32 tensor of shape `[1, 1, 192, 672]` (grayscale, normalised 0..1). The decoder accepts `[1, seq_len]` int64 token IDs and produces logits `[1, seq_len, vocab_size]`.
  Type: external
  Source: github.com/RapidAI/RapidLaTeXOCR — confirmed model architecture in session research
  Check: download models and inspect with `python -c "import onnx; m=onnx.load('encoder.onnx'); [print(i.name, i.type.tensor_type.shape) for i in m.graph.input]"`
  If false: adjust tensor shapes in `preprocess_image()` and `decode()` before proceeding.
  Owner: Task 2

- `A3` — The vocabulary file from RapidLaTeX-OCR is a plain text file with one token per line, index = line number, with special tokens: index 0 = `<PAD>`, index 1 = `<BOS>`, index 2 = `<EOS>`, index 3 = `<UNK>`.
  Type: external
  Source: RapidLaTeX-OCR repo `vocab.txt`
  Check: `head -5 <model-dir>/vocab.txt`
  If false: adjust `load_vocab()` and `decode_ids()` token index constants.
  Owner: Task 3

- `A4` — `ort` 2.0 is available on crates.io and compiles against the ORT dynamic library (`.so`/`.dll`) installed separately. The crate re-exports `ndarray` as `ort::tensor::ndarray`.
  Type: external
  Source: docs.rs/ort — version 2.0 API research
  Check: `cargo add ort --features download-binaries --optional` in a scratch project
  If false: use `ort` with `load-dynamic` feature and document the `ORT_DYLIB_PATH` env var.
  Owner: Task 1

- `A5` — The `image` crate is not currently a dependency in `Cargo.toml`; Stage 6 must add it for preprocessing tests and ONNX crop loading.
  Type: repo-state
  Source: Stage 4 follow-up check of `Cargo.toml`
  Check: `grep "^image" Cargo.toml`
  If false: add `image = { version = "0.25", optional = true }` and include it in `onnx-ocr`; if non-feature tests need it, add it as a dev-dependency too.
  Owner: Task 2

- `A6` — Existing Stage 3 users may already pass a bare executable path/string to `--formula-sidecar`.
  Type: policy
  Source: Stage 3 implementation and tests
  Check: `grep -n "formula_sidecar" src/cli.rs src/pipeline.rs tests/formula_ocr.rs`
  If false: only support the explicit `cmd:` and `onnx:` prefixes.
  Owner: Task 4

---

## File Map

| File | Change |
|------|--------|
| `Cargo.toml` | Add `ort` (optional), `ndarray` (optional); add `[features] onnx-ocr` |
| `src/formula/ocr_onnx.rs` (new) | `OnnxFormulaSidecar` struct + `FormulaSidecar` impl; `preprocess_image`, `decode` |
| `src/formula/mod.rs` | Add `#[cfg(feature = "onnx-ocr")] pub mod ocr_onnx;` |
| `src/cli.rs` | Extend `--formula-sidecar` to accept `onnx:<path>` prefix; construct `OnnxFormulaSidecar` |
| `tests/formula_onnx.rs` (new) | Unit tests for preprocessing, vocab loading, token decode logic |

---

### Task 1: Add `ort` dependency behind `onnx-ocr` feature

**Files:**
- Modify: `Cargo.toml`
- Modify: `src/formula/mod.rs`

**Ownership:**
- In scope: Cargo feature declaration, feature-gated module declaration
- Out of scope: actual ONNX inference code

**Assumption refs:** `A1`, `A4`

- [ ] **Step 1: Write a failing compilation test**

Create `tests/formula_onnx.rs` with:

```rust
#[cfg(feature = "onnx-ocr")]
mod onnx_tests {
    #[test]
    fn onnx_ocr_module_accessible() {
        // If this compiles, the feature gate and module declaration are correct.
        use pdf_processor::formula::ocr_onnx::OnnxFormulaSidecar;
        let _ = std::marker::PhantomData::<OnnxFormulaSidecar>;
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features onnx-ocr --test formula_onnx 2>&1 | tail -10`
Expected: compile error — `ocr_onnx` module not found.

- [ ] **Step 3: Add dependencies and feature to Cargo.toml**

In `[dependencies]`, add:

```toml
ort = { version = "2.0", features = ["load-dynamic"], optional = true }
ndarray = { version = "0.16", optional = true }
image = { version = "0.25", optional = true }
```

In `[features]`, add:

```toml
onnx-ocr = ["dep:ort", "dep:ndarray", "dep:image"]
```

In `src/formula/mod.rs`, add:

```rust
#[cfg(feature = "onnx-ocr")]
pub mod ocr_onnx;
```

Create `src/formula/ocr_onnx.rs` with a stub:

```rust
//! Native ONNX formula OCR via ort + RapidLaTeX-OCR models.
//! Enable with: cargo build --features onnx-ocr
//! Model files: download from github.com/RapidAI/RapidLaTeXOCR/releases
//!   encoder.onnx, decoder.onnx, vocab.txt → place in <model-dir>

use crate::formula::ocr::FormulaSidecar;
use std::path::{Path, PathBuf};

pub struct OnnxFormulaSidecar {
    model_dir: PathBuf,
}

impl OnnxFormulaSidecar {
    pub fn new(model_dir: &Path) -> anyhow::Result<Self> {
        Ok(Self { model_dir: model_dir.to_path_buf() })
    }
}

impl FormulaSidecar for OnnxFormulaSidecar {
    fn recognize(&self, _crop: &Path) -> Option<String> {
        None // stub — inference wired in Tasks 2-4
    }
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features onnx-ocr --test formula_onnx onnx_ocr_module_accessible`
Expected: PASS

Run: `cargo test 2>&1 | tail -5`
Expected: default build still passes (feature-gated code must not affect default build).

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml src/formula/mod.rs src/formula/ocr_onnx.rs tests/formula_onnx.rs
git commit -m "feat(onnx-ocr): add ort dependency and OnnxFormulaSidecar stub"
```

---

### Task 2: Implement image preprocessing

**Files:**
- Modify: `src/formula/ocr_onnx.rs`
- Test: `tests/formula_onnx.rs`

**Ownership:**
- In scope: `preprocess_image(path: &Path) -> anyhow::Result<ndarray::Array4<f32>>`
- Out of scope: ONNX session loading, decode loop

**Assumption refs:** `A2`, `A5`

- [ ] **Step 1: Write a failing test for preprocessing**

In `tests/formula_onnx.rs`, inside `onnx_tests`:

```rust
#[cfg(feature = "onnx-ocr")]
mod preprocess_tests {
    use pdf_processor::formula::ocr_onnx::preprocess_image;
    use std::path::Path;

    #[test]
    fn preprocess_returns_correct_shape() {
        // Create a minimal 10×20 white PNG in /tmp
        let png_path = std::path::PathBuf::from("/tmp/test_formula_crop.png");
        image::GrayImage::new(20, 10)
            .save(&png_path)
            .expect("write test PNG");
        let tensor = preprocess_image(&png_path).expect("preprocess");
        // Expected shape: [1, 1, 192, 672]
        assert_eq!(tensor.shape(), &[1, 1, 192, 672]);
        // Values must be in [0.0, 1.0]
        assert!(tensor.iter().all(|&v| (0.0..=1.0).contains(&v)));
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --features onnx-ocr --test formula_onnx preprocess_returns_correct_shape 2>&1 | tail -15`
Expected: compile error — `preprocess_image` not public or not found.

- [ ] **Step 3: Implement `preprocess_image`**

In `src/formula/ocr_onnx.rs`:

```rust
use image::{GrayImage, imageops::FilterType};
use ndarray::{Array4, s};

/// Load a formula crop PNG, resize to 192×672 (H×W), normalise pixel values
/// to [0.0, 1.0], and return a float32 tensor of shape [1, 1, 192, 672].
pub fn preprocess_image(path: &Path) -> anyhow::Result<Array4<f32>> {
    let img = image::open(path)
        .map_err(|e| anyhow::anyhow!("failed to open crop {}: {}", path.display(), e))?
        .to_luma8();

    // Invert (white background → black background expected by model)
    let img: GrayImage = image::imageops::colorops::invert(&img);

    // Resize to [H=192, W=672] using Lanczos3
    let resized = image::imageops::resize(&img, 672, 192, FilterType::Lanczos3);

    let mut tensor = Array4::<f32>::zeros([1, 1, 192, 672]);
    for (y, row) in resized.rows().enumerate() {
        for (x, px) in row.enumerate() {
            tensor[[0, 0, y, x]] = px[0] as f32 / 255.0;
        }
    }
    Ok(tensor)
}
```

Export `preprocess_image` as `pub` at the top of `ocr_onnx.rs`.

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test --features onnx-ocr --test formula_onnx preprocess_returns_correct_shape`
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/formula/ocr_onnx.rs tests/formula_onnx.rs
git commit -m "feat(onnx-ocr): implement formula crop preprocessing (192×672 tensor)"
```

---

### Task 3: Implement vocabulary loading and token decoding

**Files:**
- Modify: `src/formula/ocr_onnx.rs`
- Test: `tests/formula_onnx.rs`

**Ownership:**
- In scope: `load_vocab(path: &Path) -> Vec<String>`, `decode_ids(ids: &[i64], vocab: &[String]) -> String`
- Out of scope: ONNX session inference

**Assumption refs:** `A3`

- [ ] **Step 1: Write failing tests for vocab load and decode**

In `tests/formula_onnx.rs`:

```rust
#[cfg(feature = "onnx-ocr")]
mod vocab_tests {
    use pdf_processor::formula::ocr_onnx::{load_vocab, decode_ids};
    use std::io::Write;

    fn write_vocab_file(lines: &[&str]) -> tempfile::NamedTempFile {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        for line in lines {
            writeln!(f, "{}", line).unwrap();
        }
        f
    }

    #[test]
    fn load_vocab_reads_lines_as_tokens() {
        let f = write_vocab_file(&["<PAD>", "<BOS>", "<EOS>", "<UNK>", "\\frac", "x", "+"]);
        let vocab = load_vocab(f.path()).expect("load");
        assert_eq!(vocab.len(), 7);
        assert_eq!(vocab[4], "\\frac");
    }

    #[test]
    fn decode_ids_joins_tokens_and_strips_special() {
        let vocab = vec![
            "<PAD>".to_string(), "<BOS>".to_string(), "<EOS>".to_string(),
            "<UNK>".to_string(), "\\frac".to_string(), "{x}".to_string(),
            "{y}".to_string(),
        ];
        // BOS + tokens + EOS
        let ids: Vec<i64> = vec![1, 4, 5, 6, 2];
        let result = decode_ids(&ids, &vocab);
        assert_eq!(result, "\\frac {x} {y}");
    }

    #[test]
    fn decode_ids_stops_at_eos() {
        let vocab = vec![
            "<PAD>".to_string(), "<BOS>".to_string(), "<EOS>".to_string(),
            "<UNK>".to_string(), "a".to_string(), "b".to_string(),
        ];
        let ids: Vec<i64> = vec![1, 4, 2, 5]; // stops at EOS (index 2), ignores "b"
        let result = decode_ids(&ids, &vocab);
        assert_eq!(result, "a");
    }
}
```

Add `tempfile = "3"` to `[dev-dependencies]` in `Cargo.toml` if not already present.

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --features onnx-ocr --test formula_onnx load_vocab decode_ids 2>&1 | tail -15`
Expected: compile errors — functions not found.

- [ ] **Step 3: Implement `load_vocab` and `decode_ids`**

In `src/formula/ocr_onnx.rs`:

```rust
const PAD_ID: i64 = 0;
const BOS_ID: i64 = 1;
const EOS_ID: i64 = 2;

/// Load vocabulary: one token per line, index = line number.
pub fn load_vocab(path: &Path) -> anyhow::Result<Vec<String>> {
    let content = std::fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("vocab load error {}: {}", path.display(), e))?;
    Ok(content.lines().map(|l| l.to_string()).collect())
}

/// Decode token IDs to a LaTeX string, skipping BOS/EOS/PAD and stopping at EOS.
pub fn decode_ids(ids: &[i64], vocab: &[String]) -> String {
    let mut tokens: Vec<&str> = Vec::new();
    for &id in ids {
        if id == EOS_ID { break; }
        if id == BOS_ID || id == PAD_ID { continue; }
        let idx = id as usize;
        if idx < vocab.len() {
            tokens.push(&vocab[idx]);
        }
    }
    tokens.join(" ")
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --features onnx-ocr --test formula_onnx vocab_tests`
Expected: all three tests PASS.

- [ ] **Step 5: Commit**

```bash
git add src/formula/ocr_onnx.rs tests/formula_onnx.rs Cargo.toml
git commit -m "feat(onnx-ocr): vocab loading and token decoding"
```

---

### Task 4: Wire ONNX sessions and greedy decode loop

**Files:**
- Modify: `src/formula/ocr_onnx.rs` — `OnnxFormulaSidecar::new()`, `recognize()`

**Ownership:**
- In scope: `OnnxFormulaSidecar::new()` loading encoder/decoder sessions, `recognize()` running inference
- Out of scope: preprocessing (Task 2), vocabulary (Task 3), CLI wiring (Task 5)

**Assumption refs:** `A2`, `A3`, `A4`

- [ ] **Step 1: Write a failing integration test for `recognize` (with mock sessions)**

Since loading real ONNX sessions requires model files not present in CI, test the logic path with a trait mock. In `tests/formula_onnx.rs`:

```rust
#[cfg(feature = "onnx-ocr")]
mod recognize_tests {
    use pdf_processor::formula::ocr::FormulaSidecar;
    use pdf_processor::formula::ocr_onnx::OnnxFormulaSidecar;
    use std::path::Path;

    /// If no model dir is provided, new() should return an error, not panic.
    #[test]
    fn new_with_missing_dir_returns_error() {
        let result = OnnxFormulaSidecar::new(Path::new("/nonexistent/dir"));
        assert!(result.is_err(), "missing model dir must return Err");
    }

    /// If model dir exists but has no encoder.onnx, new() returns an error.
    #[test]
    fn new_with_incomplete_model_dir_returns_error() {
        let dir = tempfile::tempdir().unwrap();
        let result = OnnxFormulaSidecar::new(dir.path());
        assert!(result.is_err(), "missing encoder.onnx must return Err");
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --features onnx-ocr --test formula_onnx recognize_tests 2>&1 | tail -15`
Expected: both FAILs or compile errors — `new()` currently returns `Ok` (stub).

- [ ] **Step 3: Implement `OnnxFormulaSidecar::new()` with session loading**

Update `OnnxFormulaSidecar` struct and `new()`:

```rust
use ort::{Environment, Session, SessionBuilder};
use std::sync::Arc;

pub struct OnnxFormulaSidecar {
    encoder: Session,
    decoder: Session,
    vocab: Vec<String>,
}

impl OnnxFormulaSidecar {
    pub fn new(model_dir: &Path) -> anyhow::Result<Self> {
        let encoder_path = model_dir.join("encoder.onnx");
        let decoder_path = model_dir.join("decoder.onnx");
        let vocab_path   = model_dir.join("vocab.txt");

        if !encoder_path.exists() {
            anyhow::bail!("encoder.onnx not found in {}", model_dir.display());
        }
        if !decoder_path.exists() {
            anyhow::bail!("decoder.onnx not found in {}", model_dir.display());
        }
        if !vocab_path.exists() {
            anyhow::bail!("vocab.txt not found in {}", model_dir.display());
        }

        let encoder = SessionBuilder::new()?.with_model_from_file(&encoder_path)?;
        let decoder = SessionBuilder::new()?.with_model_from_file(&decoder_path)?;
        let vocab   = load_vocab(&vocab_path)?;

        Ok(Self { encoder, decoder, vocab })
    }
}
```

- [ ] **Step 4: Implement `recognize()` with greedy decode loop**

```rust
const MAX_DECODE_STEPS: usize = 512;

impl FormulaSidecar for OnnxFormulaSidecar {
    fn recognize(&self, crop: &Path) -> Option<String> {
        let input_tensor = preprocess_image(crop).ok()?;
        
        // Run encoder: [1, 1, 192, 672] → memory tensor
        let enc_inputs = ort::inputs![input_tensor.view()].ok()?;
        let enc_outputs = self.encoder.run(enc_inputs).ok()?;
        let memory = enc_outputs[0].try_extract_tensor::<f32>().ok()?;

        // Greedy decode: start with BOS token
        let mut token_ids: Vec<i64> = vec![BOS_ID];
        for _ in 0..MAX_DECODE_STEPS {
            let ids_array = ndarray::Array2::from_shape_vec(
                [1, token_ids.len()],
                token_ids.clone(),
            ).ok()?;
            let dec_inputs = ort::inputs![
                ids_array.view(),
                memory.view()
            ].ok()?;
            let dec_outputs = self.decoder.run(dec_inputs).ok()?;
            let logits = dec_outputs[0].try_extract_tensor::<f32>().ok()?;

            // Argmax over last token's vocab dimension
            let last_step = logits.shape()[1] - 1;
            let last_logits = logits.slice(ndarray::s![0, last_step, ..]);
            let next_id = last_logits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())?
                .0 as i64;

            token_ids.push(next_id);
            if next_id == EOS_ID {
                break;
            }
        }

        let latex = decode_ids(&token_ids, &self.vocab);
        if latex.trim().is_empty() {
            None
        } else {
            Some(latex)
        }
    }
}
```

Note: the decoder input names (`ids`, `memory`) and output name (`logits`) must match the actual ONNX model. Verify after downloading models:
```bash
python -c "
import onnx
m = onnx.load('decoder.onnx')
print('inputs:', [i.name for i in m.graph.input])
print('outputs:', [o.name for o in m.graph.output])
"
```
If the names differ, replace the hardcoded names in `ort::inputs![...]` with the actual names.

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --features onnx-ocr --test formula_onnx recognize_tests`
Expected: both PASS (no model files needed for these error-path tests).

Run: `cargo test --features onnx-ocr 2>&1 | tail -10`
Expected: all tests pass; no regressions.

- [ ] **Step 6: Commit**

```bash
git add src/formula/ocr_onnx.rs tests/formula_onnx.rs
git commit -m "feat(onnx-ocr): greedy decode loop for formula LaTeX reconstruction"
```

---

### Task 5: Wire `onnx:<path>` into CLI

**Files:**
- Modify: `src/cli.rs` — extend `--formula-sidecar` parsing
- Modify: `src/pipeline.rs` or wherever `SubprocessSidecar` is constructed

**Ownership:**
- In scope: CLI flag parsing, sidecar construction dispatch
- Out of scope: ONNX inference internals

**Assumption refs:** `A1`, `A4`

- [ ] **Step 1: Write a failing test for CLI parsing**

In `tests/formula_onnx.rs`:

```rust
#[cfg(feature = "onnx-ocr")]
mod cli_tests {
    #[test]
    fn onnx_prefix_parses_to_model_dir() {
        use pdf_processor::cli::parse_formula_sidecar;
        let result = parse_formula_sidecar("onnx:/tmp/models").expect("parse");
        assert!(matches!(result, pdf_processor::cli::FormulaSidecarArg::Onnx(_)));
    }

    #[test]
    fn cmd_prefix_parses_to_command() {
        use pdf_processor::cli::parse_formula_sidecar;
        let result = parse_formula_sidecar("cmd:rapid-latex-ocr").expect("parse");
        assert!(matches!(result, pdf_processor::cli::FormulaSidecarArg::Command(_)));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --features onnx-ocr --test formula_onnx cli_tests 2>&1 | tail -10`
Expected: compile errors — `parse_formula_sidecar` and `FormulaSidecarArg` not found.

- [ ] **Step 3: Add `FormulaSidecarArg` enum and parser to `cli.rs`**

In `src/cli.rs`:

```rust
#[derive(Debug, Clone)]
pub enum FormulaSidecarArg {
    Command(String),
    #[cfg(feature = "onnx-ocr")]
    Onnx(std::path::PathBuf),
}

pub fn parse_formula_sidecar(s: &str) -> anyhow::Result<FormulaSidecarArg> {
    if let Some(path) = s.strip_prefix("onnx:") {
        #[cfg(feature = "onnx-ocr")]
        return Ok(FormulaSidecarArg::Onnx(std::path::PathBuf::from(path)));
        #[cfg(not(feature = "onnx-ocr"))]
        anyhow::bail!("onnx: prefix requires --features onnx-ocr build");
    }
    let cmd = s.strip_prefix("cmd:").unwrap_or(s);
    Ok(FormulaSidecarArg::Command(cmd.to_string()))
}
```

In the sidecar construction site (pipeline.rs or main.rs), dispatch on `FormulaSidecarArg`:

```rust
let sidecar: Box<dyn FormulaSidecar> = match arg {
    FormulaSidecarArg::Command(cmd) => Box::new(SubprocessSidecar::new(cmd)),
    #[cfg(feature = "onnx-ocr")]
    FormulaSidecarArg::Onnx(dir) => {
        Box::new(OnnxFormulaSidecar::new(&dir)
            .map_err(|e| VtvError::Config(format!("onnx sidecar: {e}")))?)
    }
};
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test --features onnx-ocr --test formula_onnx cli_tests`
Expected: PASS.

Run: `cargo test && cargo test --features onnx-ocr 2>&1 | tail -10`
Expected: all pass on both builds.

- [ ] **Step 5: Add model download instructions to README**

In `README.md`, under a new `## Formula OCR (ONNX)` section:

```markdown
## Formula OCR (ONNX, optional)

Enable native formula LaTeX reconstruction without Python:

1. Download models:
   ```bash
   mkdir -p ~/.local/share/pdfp/rapid-latex-ocr
   cd ~/.local/share/pdfp/rapid-latex-ocr
   wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/encoder.onnx
   wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/decoder.onnx
   wget https://huggingface.co/RapidAI/RapidLaTeXOCR/resolve/main/vocab.txt
   ```

2. Install ORT shared library:
   - Ubuntu/Debian: `sudo apt install libonnxruntime-dev`
   - Arch: `sudo pacman -S onnxruntime`

3. Build and run:
   ```bash
   cargo build --release --features onnx-ocr
   pdfp convert paper.pdf --formula-sidecar onnx:~/.local/share/pdfp/rapid-latex-ocr
   ```
```

- [ ] **Step 6: Commit**

```bash
git add src/cli.rs src/pipeline.rs README.md tests/formula_onnx.rs
git commit -m "feat(onnx-ocr): wire onnx:<path> CLI flag to OnnxFormulaSidecar"
```

---

### Task 6 (final): Spec Acceptance + Post-Implementation Review

**Files:**
- Read: this plan as acceptance authority

- [ ] **Step 1: Re-read the acceptance criteria**

Criteria from the goal: (1) default build passes all tests, (2) `--features onnx-ocr` build compiles and tests pass, (3) `--formula-sidecar onnx:<path>` with a valid model dir produces LaTeX in output instead of `<!-- formula-review -->`, (4) `--formula-sidecar cmd:...` still works (Stage 3 path not broken).

- [ ] **Step 2: Run every acceptance item in one batch**

```bash
# A: Default build
cargo build 2>&1 | tail -3
cargo test 2>&1 | tail -5

# B: onnx-ocr feature build
cargo build --features onnx-ocr 2>&1 | tail -3
cargo test --features onnx-ocr 2>&1 | tail -5

# C: Clippy clean on both builds
cargo clippy -- -D warnings 2>&1 | tail -5
cargo clippy --features onnx-ocr -- -D warnings 2>&1 | tail -5

# D: End-to-end (requires model files at MODEL_DIR)
# MODEL_DIR=~/.local/share/pdfp/rapid-latex-ocr
# pdfp convert sample_with_formulas.pdf \
#   --formula-sidecar onnx:$MODEL_DIR \
#   --features onnx-ocr -o /tmp/out/
# grep '\$\$' /tmp/out/*.md | head -5

# E: Stage 3 subprocess path still works
# pdfp convert sample.pdf --formula-sidecar cmd:rapid-latex-ocr -o /tmp/out2/
```

- [ ] **Step 3: Resolve every failure**

Fix or document as Known Limitation with root cause and ≥2 approaches tried.

- [ ] **Step 4: Fill Post-Implementation Review**

Three subsections: Acceptance results, Scope drift, Refactor proposals.

- [ ] **Step 5: Surface limitations to user**

Summarise any acceptance items that did not pass.

- [ ] **Step 6: Commit**

```bash
git add .
git commit -m "docs: stage 6 post-implementation review"
```
