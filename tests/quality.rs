//! Quality-harness integration tests.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdfp"))
}

fn fixture(name: &str) -> Option<PathBuf> {
    let path = project_root().join("example/pdf").join(name);
    if !path.exists() {
        eprintln!("SKIP: fixture missing {}", path.display());
        return None;
    }
    Some(path)
}

fn fake_formula_sidecar(name: &str, latex: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("{name}-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let script = dir.join("fake-formula-sidecar");
    let mut file = std::fs::File::create(&script).unwrap();
    writeln!(file, "#!/bin/sh").unwrap();
    writeln!(file, "printf '{}\\n'", latex).unwrap();
    drop(file);

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(&script).unwrap().permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&script, permissions).unwrap();
    }

    script
}

fn run_quality_report(corpus: &Path, output_dir: &Path, recursive: bool) -> std::process::Output {
    let root = project_root();
    Command::new("bash")
        .arg(root.join("scripts/quality-report.sh"))
        .env("PDFP_QUALITY_CORPUS", corpus)
        .env("PDFP_QUALITY_OUT", output_dir)
        .env("PDFP_QUALITY_RECURSIVE", if recursive { "1" } else { "0" })
        .output()
        .expect("quality report script should be runnable with bash")
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

#[test]
fn quality_report_distinguishes_top_level_from_recursive_corpus() {
    let Some(fixture) = fixture("golden__lorem.pdf") else {
        return;
    };
    let root = project_root();
    let corpus = root.join("target/quality-test-corpus");
    let nested = corpus.join("nested");
    let top_output = root.join("target/quality-test-top-output");
    let recursive_output = root.join("target/quality-test-recursive-output");
    let _ = std::fs::remove_dir_all(&corpus);
    let _ = std::fs::remove_dir_all(&top_output);
    let _ = std::fs::remove_dir_all(&recursive_output);
    std::fs::create_dir_all(&nested).unwrap();

    std::fs::copy(&fixture, corpus.join("top.pdf")).unwrap();
    std::fs::copy(&fixture, nested.join("nested.pdf")).unwrap();

    let top = run_quality_report(&corpus, &top_output, false);
    assert!(
        top.status.success(),
        "top-level quality report failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&top.stdout),
        String::from_utf8_lossy(&top.stderr)
    );

    let recursive = run_quality_report(&corpus, &recursive_output, true);
    assert!(
        recursive.status.success(),
        "recursive quality report failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&recursive.stdout),
        String::from_utf8_lossy(&recursive.stderr)
    );

    let top_report: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(top_output.join("report.json")).unwrap())
            .unwrap();
    let recursive_report: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(recursive_output.join("report.json")).unwrap(),
    )
    .unwrap();

    assert_eq!(top_report["corpus_mode"], "top-level");
    assert_eq!(top_report["case_count"], 1);
    assert_eq!(top_report["summary"]["total"], 1);
    assert_eq!(recursive_report["corpus_mode"], "recursive");
    assert_eq!(recursive_report["case_count"], 2);
    assert_eq!(recursive_report["summary"]["total"], 2);
    assert!(top_report["cases"][0]["heading_density"].is_number());
    assert!(top_report["cases"][0]["images_per_page"].is_number());
    assert!(top_report["quality_warnings"].is_array());
}

#[test]
fn scan_only_fixture_is_image_only_without_ocr() {
    let Some(pdf) = fixture("golden__chinese_scan.pdf") else {
        return;
    };
    let root = project_root();
    let output_dir = root.join("target/quality-test-chinese-scan");
    let _ = std::fs::remove_dir_all(&output_dir);

    let output = Command::new(bin_path())
        .arg(&pdf)
        .arg("-o")
        .arg(&output_dir)
        .arg("--ocr")
        .arg("off")
        .output()
        .expect("pdfp should run on the scan fixture");

    assert!(
        output.status.success(),
        "pdfp failed on scan fixture\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("scan-heavy"),
        "expected scan-heavy warning in stderr"
    );

    let markdown = std::fs::read_to_string(output_dir.join("golden__chinese_scan.md"))
        .expect("scan fixture markdown should exist");

    let meaningful_lines: Vec<&str> = markdown
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty()
                && !trimmed.starts_with("<!-- page:")
                && !trimmed.starts_with("<!-- WARNING:")
                && !trimmed.starts_with("![image]")
        })
        .collect();

    assert!(
        meaningful_lines.is_empty(),
        "scan fixture should be image-only before OCR, got meaningful lines: {meaningful_lines:?}"
    );
}

#[test]
fn formula_baseline_skips_when_standards_absent() {
    let standards = PathBuf::from("/home/eastill/projects/literature/standards");
    if standards.exists() {
        return;
    }

    let root = project_root();
    let output_dir = root.join("target/formula-baseline-missing-standards");
    let _ = std::fs::remove_dir_all(&output_dir);
    std::fs::create_dir_all(&output_dir).unwrap();
    std::fs::write(
        output_dir.join("formula-baseline.json"),
        r#"{"status":"skipped","reason":"missing standards corpus"}"#,
    )
    .unwrap();

    let report = std::fs::read_to_string(output_dir.join("formula-baseline.json")).unwrap();
    assert!(report.contains("\"status\":\"skipped\""));
}

#[test]
fn formula_candidate_report_contains_page_and_status() {
    let Some(pdf) = fixture("math-number-theory.pdf") else {
        return;
    };
    let root = project_root();
    let out = root.join("target/formula-quality-report");
    let _ = std::fs::remove_dir_all(&out);

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
        .expect("pdfp should run on math fixture");

    assert!(
        output.status.success(),
        "formula baseline command failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let debug_dir = out.join("debug").join("formulas");
    let index = std::fs::read_to_string(debug_dir.join("index.json"))
        .expect("expected aggregate formula index");
    assert!(index.contains("\"schema_version\": 1"));
    assert!(index.contains("\"candidate_count\""));
    assert!(index.contains("\"pages_with_candidates\""));
    assert!(index.contains("\"emitted_count\""));
    assert!(index.contains("\"review_block_count\""));

    let page_report_path = std::fs::read_dir(&debug_dir)
        .unwrap_or_else(|err| panic!("expected formula debug dir {debug_dir:?}: {err}"))
        .map(|entry| entry.unwrap().path())
        .find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("page") && name.ends_with(".json"))
        })
        .expect("expected at least one page formula JSON report");
    let page_report = std::fs::read_to_string(page_report_path).unwrap();
    assert!(page_report.contains("\"page_num\""));
    assert!(page_report.contains("\"status\""));
}

#[test]
fn formula_eval_harness_runs_with_no_optional_providers() {
    let Some(pdf) = fixture("math-number-theory.pdf") else {
        return;
    };
    let root = project_root();
    let output_dir = root.join("target/formula-eval-no-optional-test");
    let _ = std::fs::remove_dir_all(&output_dir);

    let result = Command::new("bash")
        .arg(root.join("scripts/formula-eval.sh"))
        .arg(&pdf)
        .arg(&output_dir)
        .env(
            "PDFP_FORMULA_EVAL_PROVIDERS",
            "native rapid-latex-ocr docling onnx",
        )
        .env("PDFP_FORMULA_EVAL_DOCLING_URL", "http://127.0.0.1:9")
        .env(
            "PDFP_FORMULA_EVAL_SIDECAR_COMMAND",
            "pdfp-missing-rapid-latex-ocr",
        )
        .output()
        .expect("formula eval script should be runnable with bash");

    assert!(
        result.status.success(),
        "formula eval should skip unavailable optional providers\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let summary = std::fs::read_to_string(output_dir.join("summary.json"))
        .expect("formula eval should write summary.json");
    assert!(summary.contains("native"));
    assert!(summary.contains("skipped"));
}

#[test]
fn formula_eval_harness_runs_with_fake_sidecar_provider() {
    let Some(pdf) = fixture("math-number-theory.pdf") else {
        return;
    };
    let root = project_root();
    let output_dir = root.join("target/formula-eval-fake-sidecar-test");
    let _ = std::fs::remove_dir_all(&output_dir);
    let sidecar = fake_formula_sidecar("pdfp-formula-eval-fake", "E = mc^2");

    let result = Command::new("bash")
        .arg(root.join("scripts/formula-eval.sh"))
        .arg(&pdf)
        .arg(&output_dir)
        .env("PDFP_FORMULA_EVAL_PROVIDERS", "native sidecar")
        .env("PDFP_FORMULA_EVAL_SIDECAR_COMMAND", &sidecar)
        .output()
        .expect("formula eval script should be runnable with bash");

    assert!(
        result.status.success(),
        "formula eval fake sidecar run failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let summary: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(output_dir.join("summary.json")).unwrap())
            .unwrap();
    let sidecar_row = summary["providers"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["provider"] == "sidecar")
        .expect("sidecar row should exist");
    assert_eq!(sidecar_row["status"], "ok");
    assert!(sidecar_row["recovered"].as_u64().unwrap() > 0);
}

#[test]
fn sidecar_audit_skips_missing_optional_backends_cleanly() {
    let Some(_) = fixture("math-number-theory.pdf") else {
        return;
    };
    let root = project_root();
    let output_dir = root.join("target/sidecar-audit-test");
    let _ = std::fs::remove_dir_all(&output_dir);

    let result = Command::new("bash")
        .arg(root.join("scripts/sidecar-audit.sh"))
        .env("PDFP_SIDECAR_OUT", &output_dir)
        .env("PDFP_SIDECAR_FIXTURES", "math-number-theory.pdf")
        .env(
            "PDFP_SIDECAR_BACKENDS",
            "native docling gmft img2table unimernet",
        )
        .env("PDFP_SIDECAR_DOCLING_URL", "http://127.0.0.1:9")
        .output()
        .expect("sidecar audit script should be runnable with bash");

    assert!(
        result.status.success(),
        "sidecar audit should skip unavailable optional backends\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let summary = std::fs::read_to_string(output_dir.join("summary.md"))
        .expect("sidecar audit should write a summary");
    assert!(summary.contains("native"));
    assert!(summary.contains("unavailable") || summary.contains("skipped"));
}
