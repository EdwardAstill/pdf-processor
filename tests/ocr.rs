//! Local OCR sidecar integration tests that do not require OCRmyPDF to be installed.

use std::path::PathBuf;
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdfp"))
}

fn missing_ocr_command() -> &'static str {
    "definitely-missing-pdfp-ocr-command"
}

fn fixture(name: &str) -> Option<PathBuf> {
    let path = root().join("example/pdf").join(name);
    if !path.exists() {
        eprintln!("SKIP: fixture missing {}", path.display());
        return None;
    }
    Some(path)
}

#[cfg(unix)]
fn fake_ocrmypdf(path: &std::path::Path) {
    use std::os::unix::fs::PermissionsExt;

    std::fs::write(
        path,
        "#!/usr/bin/env bash\nset -euo pipefail\ninput=\"${@: -2:1}\"\noutput=\"${@: -1}\"\ncp \"$input\" \"$output\"\n",
    )
    .unwrap();
    let mut perms = std::fs::metadata(path).unwrap().permissions();
    perms.set_mode(0o755);
    std::fs::set_permissions(path, perms).unwrap();
}

#[test]
fn clean_pdf_ocr_auto_skips_missing_ocr_tool() {
    let Some(input) = fixture("golden__lorem.pdf") else {
        return;
    };
    let root = root();
    let out = root.join("target/ocr-auto-clean");
    let _ = std::fs::remove_dir_all(&out);

    let output = Command::new(bin_path())
        .arg(&input)
        .arg("--ocr")
        .arg("auto")
        .arg("--ocr-command")
        .arg(missing_ocr_command())
        .arg("-o")
        .arg(&out)
        .output()
        .expect("pdfp should run");

    assert!(
        output.status.success(),
        "clean PDF should skip OCR even when OCR command is missing\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        out.join("golden__lorem.md").exists(),
        "conversion output should still be written"
    );
}

#[test]
fn scan_pdf_ocr_auto_reports_missing_ocr_tool() {
    let Some(input) = fixture("golden__chinese_scan.pdf") else {
        return;
    };
    let root = root();
    let out = root.join("target/ocr-auto-scan-missing-tool");
    let _ = std::fs::remove_dir_all(&out);

    let output = Command::new(bin_path())
        .arg(&input)
        .arg("--ocr")
        .arg("auto")
        .arg("--ocr-command")
        .arg(missing_ocr_command())
        .arg("-o")
        .arg(&out)
        .output()
        .expect("pdfp should run");

    assert!(
        !output.status.success(),
        "scan PDF should need OCR and fail when OCR command is missing"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("OCRmyPDF command") && stderr.contains(missing_ocr_command()),
        "missing-tool error should be actionable, got:\n{stderr}"
    );
}

#[test]
fn standalone_ocr_auto_copies_clean_pdf_when_tool_is_missing() {
    let Some(input) = fixture("golden__lorem.pdf") else {
        return;
    };
    let root = root();
    let out = root.join("target/standalone-ocr-clean.pdf");
    let _ = std::fs::remove_file(&out);

    let output = Command::new(bin_path())
        .arg("ocr")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--command")
        .arg(missing_ocr_command())
        .arg("--json")
        .output()
        .expect("pdfp ocr should run");

    assert!(
        output.status.success(),
        "standalone OCR should skip and copy clean PDF\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.exists(), "standalone OCR should write output PDF");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["mode"], "auto");
    assert_eq!(report["status"], "skipped");
    assert_eq!(report["output"], out.display().to_string());
}

#[test]
fn standalone_ocr_auto_reports_missing_tool_for_scan() {
    let Some(input) = fixture("golden__chinese_scan.pdf") else {
        return;
    };
    let root = root();
    let out = root.join("target/standalone-ocr-scan.pdf");
    let _ = std::fs::remove_file(&out);

    let output = Command::new(bin_path())
        .arg("ocr")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--command")
        .arg(missing_ocr_command())
        .output()
        .expect("pdfp ocr should run");

    assert!(!output.status.success(), "scan should require OCR command");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("OCRmyPDF command") && stderr.contains(missing_ocr_command()),
        "missing-tool error should be actionable, got:\n{stderr}"
    );
}

#[test]
fn doctor_reports_ocr_status() {
    let output = Command::new(bin_path())
        .arg("doctor")
        .arg("--json")
        .output()
        .expect("pdfp doctor should run");

    assert!(
        output.status.success(),
        "doctor should succeed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["pdfp"]["version"].is_string());
    assert!(report["ocr"]["available"].is_boolean());
    assert!(report["ocr"]["searched"].is_array());
}

#[test]
#[cfg(unix)]
fn standalone_ocr_uses_fake_command_and_writes_pdf() {
    let Some(input) = fixture("golden__chinese_scan.pdf") else {
        return;
    };
    let root = root();
    let work = root.join("target/standalone-ocr-fake");
    let out = work.join("scan.ocr.pdf");
    let command = work.join("fake-ocrmypdf");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    fake_ocrmypdf(&command);

    let output = Command::new(bin_path())
        .arg("ocr")
        .arg(&input)
        .arg("-o")
        .arg(&out)
        .arg("--command")
        .arg(&command)
        .arg("--json")
        .output()
        .expect("pdfp ocr should run");

    assert!(
        output.status.success(),
        "fake standalone OCR failed\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(out.exists(), "fake OCR should write requested output");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["status"], "ran");
    assert_eq!(report["output"], out.display().to_string());
}

#[test]
fn inspect_json_reports_ocr_decision() {
    let Some(input) = fixture("golden__lorem.pdf") else {
        return;
    };
    let output = Command::new(bin_path())
        .arg("inspect")
        .arg(&input)
        .arg("--ocr")
        .arg("auto")
        .arg("--ocr-command")
        .arg(missing_ocr_command())
        .arg("--json")
        .output()
        .expect("pdfp inspect should run");

    assert!(
        output.status.success(),
        "inspect should skip OCR for clean PDF\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ocr"]["mode"], "auto");
    assert_eq!(report["ocr"]["status"], "skipped");
}

#[test]
fn search_ocr_auto_skips_clean_pdf() {
    let Some(input) = fixture("attention.pdf") else {
        return;
    };
    let output = Command::new(bin_path())
        .arg("search")
        .arg(&input)
        .arg("Attention")
        .arg("--ocr")
        .arg("auto")
        .arg("--ocr-command")
        .arg(missing_ocr_command())
        .arg("--json")
        .arg("--verbose")
        .output()
        .expect("pdfp search should run");

    assert!(
        output.status.success(),
        "search should skip OCR for born-digital PDF\nstderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains(missing_ocr_command()),
        "born-digital search should not try to invoke OCR, got:\n{stderr}"
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["ocr"]["status"], "skipped");
    assert!(report["matches"]
        .as_array()
        .is_some_and(|matches| !matches.is_empty()));
}

#[test]
#[cfg(unix)]
fn scan_pdf_ocr_cache_hits_on_second_run() {
    let Some(input) = fixture("golden__chinese_scan.pdf") else {
        return;
    };
    let root = root();
    let work = root.join("target/ocr-cache-hit-test");
    let out1 = work.join("out1");
    let out2 = work.join("out2");
    let cache = work.join("cache");
    let command = work.join("fake-ocrmypdf");
    let _ = std::fs::remove_dir_all(&work);
    std::fs::create_dir_all(&work).unwrap();
    fake_ocrmypdf(&command);

    let first = Command::new(bin_path())
        .arg(&input)
        .arg("--ocr")
        .arg("auto")
        .arg("--ocr-cache-dir")
        .arg(&cache)
        .arg("--ocr-command")
        .arg(&command)
        .arg("--verbose")
        .arg("-o")
        .arg(&out1)
        .output()
        .expect("first pdfp OCR run should start");
    assert!(
        first.status.success(),
        "first fake OCR run failed\nstderr:\n{}",
        String::from_utf8_lossy(&first.stderr)
    );

    let second = Command::new(bin_path())
        .arg(&input)
        .arg("--ocr")
        .arg("auto")
        .arg("--ocr-cache-dir")
        .arg(&cache)
        .arg("--ocr-command")
        .arg(&command)
        .arg("--verbose")
        .arg("-o")
        .arg(&out2)
        .output()
        .expect("second pdfp OCR run should start");
    assert!(
        second.status.success(),
        "second fake OCR run failed\nstderr:\n{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("ocr: cache hit"),
        "expected second run to report OCR cache hit\nstderr:\n{}",
        String::from_utf8_lossy(&second.stderr)
    );
}
