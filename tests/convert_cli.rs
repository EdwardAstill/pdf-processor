//! Conversion CLI regression tests for the simple default command surface.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdfp"))
}

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("example")
        .join("pdf")
        .join(name)
}

fn temp_out(name: &str) -> PathBuf {
    let out = std::env::temp_dir().join(format!("pdfp-convert-cli-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    out
}

#[test]
fn bare_convert_writes_flat_markdown_without_asset_dirs() {
    let pdf = fixture("golden__lorem.pdf");
    if !pdf.exists() {
        eprintln!("SKIP: fixture missing {}", pdf.display());
        return;
    }

    let out = temp_out("flat");
    let output = Command::new(bin_path())
        .arg(&pdf)
        .arg("--output")
        .arg(&out)
        .arg("--ocr")
        .arg("off")
        .output()
        .expect("run pdfp");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(out.join("golden__lorem.md").exists());
    assert!(!out.join("golden__lorem").exists());
    assert!(!out.join("images").exists());
    assert!(!out.join("tables").exists());
    assert!(!out.join("equations").exists());
}

#[test]
fn optional_asset_flags_create_requested_folders() {
    let pdf = fixture("math-number-theory.pdf");
    if !pdf.exists() {
        eprintln!("SKIP: fixture missing {}", pdf.display());
        return;
    }

    let out = temp_out("assets");
    let output = Command::new(bin_path())
        .arg(&pdf)
        .arg("--output")
        .arg(&out)
        .arg("--ocr")
        .arg("off")
        .arg("--pages")
        .arg("1")
        .arg("--images")
        .arg("--tables")
        .arg("--equations")
        .output()
        .expect("run pdfp");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(out.join("math-number-theory.md").exists());
    assert!(out.join("images").is_dir());
    assert!(out.join("tables").is_dir());
    assert!(out.join("equations").is_dir());
}
