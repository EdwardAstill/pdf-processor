//! Figure snapshot CLI regression tests.

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
    let out = std::env::temp_dir().join(format!("pdfp-figure-test-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&out);
    out
}

#[test]
fn snapshot_mode_writes_figure_pngs_not_embedded_image_links() {
    let Some(pdf) = fixture("attention.pdf") else {
        return;
    };
    let out = temp_out("snapshot");
    let output = Command::new(bin_path())
        .args([
            "convert",
            pdf.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--figures",
            "snapshot",
            "--figure-dpi",
            "96",
            "--debug-figures",
        ])
        .output()
        .expect("run pdfp convert");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let md = fs::read_to_string(out.join("attention").join("attention.md")).unwrap();
    assert!(md.contains("images/page"));
    assert!(md.contains("_fig"));
    assert!(!md.contains("_img"));

    let image_dir = out.join("attention").join("images");
    let figure_pngs: Vec<_> = fs::read_dir(&image_dir)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.file_name().unwrap().to_string_lossy().contains("_fig"))
        .collect();
    assert!(
        !figure_pngs.is_empty(),
        "no figure snapshots in {image_dir:?}"
    );

    let first = fs::read(&figure_pngs[0]).unwrap();
    assert!(first.starts_with(b"\x89PNG\r\n\x1a\n"));

    let debug = fs::read_to_string(
        out.join("attention")
            .join("debug")
            .join("figures")
            .join("page3.json"),
    )
    .unwrap();
    assert!(debug.contains("\"bbox\""));
    assert!(debug.contains("\"confidence\""));
}

#[test]
fn no_images_suppresses_snapshot_figures() {
    let Some(pdf) = fixture("attention.pdf") else {
        return;
    };
    let out = temp_out("none");
    let output = Command::new(bin_path())
        .args([
            "convert",
            pdf.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--figures",
            "snapshot",
            "--no-images",
        ])
        .output()
        .expect("run pdfp convert");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let md = fs::read_to_string(out.join("attention").join("attention.md")).unwrap();
    assert!(!md.contains("_fig"));
    assert!(!out.join("attention").join("images").exists());
}

#[test]
fn figures_none_suppresses_all_image_outputs() {
    let Some(pdf) = fixture("attention.pdf") else {
        return;
    };
    let out = temp_out("figures-none");
    let output = Command::new(bin_path())
        .args([
            "convert",
            pdf.to_str().unwrap(),
            "--output",
            out.to_str().unwrap(),
            "--figures",
            "none",
        ])
        .output()
        .expect("run pdfp convert");

    assert!(
        output.status.success(),
        "convert failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let md = fs::read_to_string(out.join("attention").join("attention.md")).unwrap();
    assert!(!md.contains("images/"));
    assert!(!out.join("attention").join("images").exists());
}
