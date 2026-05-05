//! Quality-harness integration tests.

use std::path::PathBuf;
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn quality_report_skips_cleanly_when_corpus_is_absent() {
    let root = project_root();
    let missing_corpus = root.join("target/quality-test-missing-corpus");
    let output_dir = root.join("target/quality-test-output");
    let _ = std::fs::remove_dir_all(&missing_corpus);
    let _ = std::fs::remove_dir_all(&output_dir);

    let result = Command::new("bash")
        .arg(root.join("scripts/quality-report.sh"))
        .env("PDFP_QUALITY_CORPUS", &missing_corpus)
        .env("PDFP_QUALITY_OUT", &output_dir)
        .output()
        .expect("quality report script should be runnable with bash");

    assert!(
        result.status.success(),
        "quality report should exit 0 when fixtures are absent\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let stdout = String::from_utf8_lossy(&result.stdout);
    assert!(
        stdout.contains("SKIP") && stdout.contains("missing corpus"),
        "expected clear skip summary, got:\n{stdout}"
    );

    let report = std::fs::read_to_string(output_dir.join("report.json"))
        .expect("quality report should still write report.json for skipped corpus");
    assert!(report.contains("\"status\":\"skipped\""));
}
