//! CLI help smoke tests.

use std::path::PathBuf;
use std::process::Command;

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdfp"))
}

#[test]
fn every_command_path_prints_help() {
    let commands: &[&[&str]] = &[
        &[],
        &["convert"],
        &["inspect"],
        &["search"],
        &["pages"],
        &["pages", "extract"],
        &["pages", "delete"],
        &["pages", "split"],
        &["pages", "reorder"],
        &["pages", "merge"],
        &["impose"],
        &["impose", "2up"],
        &["impose", "booklet"],
        &["page"],
        &["page", "resize"],
    ];

    for command in commands {
        let output = Command::new(bin_path())
            .args(*command)
            .arg("--help")
            .output()
            .unwrap_or_else(|err| panic!("failed to run pdfp {command:?} --help: {err}"));

        assert!(
            output.status.success(),
            "pdfp {command:?} --help failed\nstdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains("Usage:") && stdout.contains("--help"),
            "pdfp {command:?} --help did not look like clap help:\n{stdout}"
        );
    }
}
