//! CLI integration tests for PDF document information metadata.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lopdf::{dictionary, text_string, Document, Object, Stream};
use serde_json::Value;
use tempfile::TempDir;

fn bin_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdfp"))
}

fn run_pdfp(args: &[String]) -> Output {
    Command::new(bin_path())
        .args(args)
        .output()
        .unwrap_or_else(|err| panic!("failed to run pdfp {args:?}: {err}"))
}

fn path_arg(path: &Path) -> String {
    path.display().to_string()
}

fn write_pdf(path: &Path, info: &[(&str, &str)], xmp: bool, signed: bool) {
    let mut doc = Document::with_version("1.7");
    let pages_id = doc.new_object_id();

    let font_id = doc.add_object(dictionary! {
        "Type" => "Font",
        "Subtype" => "Type1",
        "BaseFont" => "Helvetica",
    });
    let resources_id = doc.add_object(dictionary! {
        "Font" => dictionary! {
            "F1" => font_id,
        },
    });
    let content_id = doc.add_object(Stream::new(
        dictionary! {},
        b"BT /F1 12 Tf 72 720 Td (Hello metadata) Tj ET".to_vec(),
    ));
    let page_id = doc.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "Resources" => resources_id,
        "Contents" => content_id,
        "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
    });
    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => vec![page_id.into()],
            "Count" => 1,
        }),
    );

    let mut catalog = dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    };
    if xmp {
        let xmp_id = doc.add_object(Stream::new(
            dictionary! {
                "Type" => "Metadata",
                "Subtype" => "XML",
            },
            b"<x:xmpmeta></x:xmpmeta>".to_vec(),
        ));
        catalog.set("Metadata", xmp_id);
    }
    if signed {
        let sig_id = doc.add_object(dictionary! {
            "FT" => "Sig",
            "T" => text_string("Signature1"),
        });
        let acro_form_id = doc.add_object(dictionary! {
            "Fields" => vec![sig_id.into()],
        });
        catalog.set("AcroForm", acro_form_id);
    }

    let catalog_id = doc.add_object(catalog);
    doc.trailer.set("Root", catalog_id);

    if !info.is_empty() {
        let mut info_dict = lopdf::Dictionary::new();
        for (key, value) in info {
            info_dict.set(*key, text_string(value));
        }
        let info_id = doc.add_object(info_dict);
        doc.trailer.set("Info", info_id);
    }

    doc.save(path).unwrap();
}

fn show_json(path: &Path) -> Value {
    let output = run_pdfp(&[
        "metadata".to_string(),
        "show".to_string(),
        path_arg(path),
        "--json".to_string(),
    ]);
    assert!(
        output.status.success(),
        "metadata show failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn metadata_show_json_reports_full_info() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    write_pdf(
        &input,
        &[
            ("Title", "Original Title"),
            ("Author", "Ada"),
            ("Subject", "Metadata"),
            ("Keywords", "pdf,rust"),
            ("Creator", "fixture"),
            ("Producer", "lopdf"),
            ("CreationDate", "D:20260102030405Z"),
            ("ModDate", "D:20260103040506Z"),
        ],
        false,
        false,
    );

    let json = show_json(&input);
    assert_eq!(json["page_count"], 1);
    assert_eq!(json["info"]["title"], "Original Title");
    assert_eq!(json["info"]["author"], "Ada");
    assert_eq!(json["info"]["subject"], "Metadata");
    assert_eq!(json["info"]["keywords"], "pdf,rust");
    assert_eq!(json["info"]["creator"], "fixture");
    assert_eq!(json["info"]["producer"], "lopdf");
    assert_eq!(json["info"]["creation_date"], "D:20260102030405Z");
    assert_eq!(json["info"]["modification_date"], "D:20260103040506Z");
    assert_eq!(json["xmp"]["present"], false);
    assert_eq!(json["signatures"]["present"], false);
}

#[test]
fn metadata_set_round_trips_selected_fields() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let output_path = temp.path().join("output.pdf");
    write_pdf(&input, &[("Title", "Original")], false, false);

    let output = run_pdfp(&[
        "metadata".to_string(),
        "set".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&output_path),
        "--title".to_string(),
        "Updated".to_string(),
        "--author".to_string(),
        "Grace Hopper".to_string(),
        "--keywords".to_string(),
        "pdf,metadata".to_string(),
        "--creation-date".to_string(),
        "2026-05-19T12:30:00Z".to_string(),
        "--no-touch-mod-date".to_string(),
        "--json".to_string(),
    ]);

    assert!(
        output.status.success(),
        "metadata set failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(report["changed"]
        .as_array()
        .unwrap()
        .contains(&Value::from("title")));

    let json = show_json(&output_path);
    assert_eq!(json["info"]["title"], "Updated");
    assert_eq!(json["info"]["author"], "Grace Hopper");
    assert_eq!(json["info"]["keywords"], "pdf,metadata");
    assert_eq!(json["info"]["creation_date"], "D:20260519123000Z");
    assert_eq!(json["info"]["modification_date"], Value::Null);
}

#[test]
fn metadata_set_preserves_unicode_title() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let output_path = temp.path().join("unicode.pdf");
    write_pdf(&input, &[], false, false);

    let output = run_pdfp(&[
        "metadata".to_string(),
        "set".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&output_path),
        "--title".to_string(),
        "Resume Ω".to_string(),
        "--no-touch-mod-date".to_string(),
    ]);

    assert!(
        output.status.success(),
        "metadata set failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json = show_json(&output_path);
    assert_eq!(json["info"]["title"], "Resume Ω");
}

#[test]
fn metadata_clear_removes_selected_fields() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let output_path = temp.path().join("cleared.pdf");
    write_pdf(
        &input,
        &[
            ("Title", "Original"),
            ("Author", "Ada"),
            ("Subject", "Keep"),
        ],
        false,
        false,
    );

    let output = run_pdfp(&[
        "metadata".to_string(),
        "clear".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&output_path),
        "--fields".to_string(),
        "title,author".to_string(),
        "--json".to_string(),
    ]);

    assert!(
        output.status.success(),
        "metadata clear failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let json = show_json(&output_path);
    assert_eq!(json["info"]["title"], Value::Null);
    assert_eq!(json["info"]["author"], Value::Null);
    assert_eq!(json["info"]["subject"], "Keep");
}

#[test]
fn metadata_refuses_same_input_output() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    write_pdf(&input, &[], false, false);

    let output = run_pdfp(&[
        "metadata".to_string(),
        "set".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&input),
        "--title".to_string(),
        "Updated".to_string(),
    ]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite input PDF"),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn metadata_refuses_equivalent_same_input_output() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let equivalent_output = temp.path().join(".").join("input.pdf");
    write_pdf(&input, &[], false, false);

    let output = run_pdfp(&[
        "metadata".to_string(),
        "set".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&equivalent_output),
        "--title".to_string(),
        "Updated".to_string(),
    ]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("refusing to overwrite input PDF"),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn metadata_warns_when_xmp_present() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let output_path = temp.path().join("output.pdf");
    write_pdf(&input, &[], true, false);

    let output = run_pdfp(&[
        "metadata".to_string(),
        "set".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&output_path),
        "--title".to_string(),
        "Updated".to_string(),
        "--no-touch-mod-date".to_string(),
        "--json".to_string(),
    ]);

    assert!(
        output.status.success(),
        "metadata set failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: Value = serde_json::from_slice(&output.stdout).unwrap();
    let warnings = report["warnings"].as_array().unwrap();
    assert!(
        warnings.iter().any(|warning| warning
            .as_str()
            .unwrap()
            .contains("XMP metadata is present")),
        "{warnings:?}"
    );
}

#[test]
fn metadata_refuses_signed_pdf_without_force() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("signed.pdf");
    let output_path = temp.path().join("output.pdf");
    write_pdf(&input, &[], false, true);

    let output = run_pdfp(&[
        "metadata".to_string(),
        "set".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&output_path),
        "--title".to_string(),
        "Updated".to_string(),
    ]);

    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("signature fields") && stderr.contains("--force-signed"),
        "stderr:\n{stderr}"
    );
}

#[test]
fn metadata_rejects_invalid_pdf_date() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let output_path = temp.path().join("output.pdf");
    write_pdf(&input, &[], false, false);

    let output = run_pdfp(&[
        "metadata".to_string(),
        "set".to_string(),
        path_arg(&input),
        "-o".to_string(),
        path_arg(&output_path),
        "--creation-date".to_string(),
        "D:20260519123000+0800".to_string(),
    ]);

    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("valid PDF date"),
        "stderr:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
}
