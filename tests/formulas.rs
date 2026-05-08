//! Formula detection and debug-ledger CLI regression tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdfp"))
}

fn fixture(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("example")
        .join("pdf")
        .join(name);
    if !path.exists() {
        eprintln!("SKIP: fixture missing {}", path.display());
        return None;
    }
    Some(path)
}

fn temp_out(name: &str) -> PathBuf {
    let out = std::env::temp_dir().join(format!("pdfp-formula-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    out
}

#[test]
fn debug_formulas_writes_json_for_candidate_page() {
    let Some(pdf) = fixture("math-number-theory.pdf") else {
        return;
    };
    let out = temp_out("json");
    let output = Command::new(bin_path())
        .args([
            "convert",
            pdf.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-images",
            "--debug-formulas",
        ])
        .output()
        .expect("run pdfp convert");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let debug_dir = out
        .join("math-number-theory")
        .join("debug")
        .join("formulas");
    let json_files: Vec<_> = fs::read_dir(&debug_dir)
        .unwrap_or_else(|err| panic!("expected debug dir {debug_dir:?}: {err}"))
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "json"))
        .collect();
    assert!(!json_files.is_empty(), "expected formula debug JSON");

    let first_json = fs::read_to_string(&json_files[0]).unwrap();
    assert!(first_json.contains("\"page_num\""));
    assert!(first_json.contains("\"status\""));
    assert!(first_json.contains("\"confidence\""));
}

#[test]
fn debug_formulas_writes_crop_for_candidate_page() {
    let Some(pdf) = fixture("math-number-theory.pdf") else {
        return;
    };
    let out = temp_out("crop");
    let output = Command::new(bin_path())
        .args([
            "convert",
            pdf.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-images",
            "--debug-formulas",
        ])
        .output()
        .expect("run pdfp convert");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let debug_dir = out
        .join("math-number-theory")
        .join("debug")
        .join("formulas");
    let crop_files: Vec<_> = fs::read_dir(&debug_dir)
        .unwrap_or_else(|err| panic!("expected debug dir {debug_dir:?}: {err}"))
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.file_name()
                .unwrap()
                .to_string_lossy()
                .contains("_formula")
                && path.extension().is_some_and(|ext| ext == "png")
        })
        .collect();
    assert!(!crop_files.is_empty(), "expected formula crop PNGs");

    let first = fs::read(&crop_files[0]).unwrap();
    assert!(first.starts_with(b"\x89PNG\r\n\x1a\n"));
}

#[test]
fn formulas_auto_without_hybrid_audits_without_injecting_heuristic_math() {
    let Some(pdf) = fixture("math-number-theory.pdf") else {
        return;
    };
    let out = temp_out("auto-local");
    let output = Command::new(bin_path())
        .args([
            "convert",
            pdf.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-images",
            "--formulas",
            "auto",
            "--debug-formulas",
        ])
        .output()
        .expect("run pdfp convert");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("formula candidate"),
        "expected formula warning in stderr"
    );

    let md =
        fs::read_to_string(out.join("math-number-theory").join("math-number-theory.md")).unwrap();
    assert!(
        !md.trim().is_empty(),
        "local markdown should still be written"
    );
    assert!(
        !md.contains("$$"),
        "auto mode should audit formula candidates without rendering heuristic formula blocks"
    );
}

#[test]
fn conservative_mode_does_not_render_local_formula_candidates() {
    let Some(pdf) = fixture("math-number-theory.pdf") else {
        return;
    };
    let out = temp_out("conservative");
    let output = Command::new(bin_path())
        .args([
            "convert",
            pdf.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--no-images",
            "--conservative",
            "--formulas",
            "local",
            "--debug-formulas",
        ])
        .output()
        .expect("run pdfp convert");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let md =
        fs::read_to_string(out.join("math-number-theory").join("math-number-theory.md")).unwrap();
    assert!(
        !md.contains("$$"),
        "conservative mode should not render heuristic formula blocks even if --formulas local is present"
    );
}
