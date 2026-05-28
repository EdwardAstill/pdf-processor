//! CLI integration tests for page-level PDF operations.

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

use lopdf::{dictionary, Document, Object, Stream};
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

fn write_two_page_pdf(path: &Path) {
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

    let mut kids = Vec::new();
    for page_num in 1..=2 {
        let content = format!("BT /F1 12 Tf 72 720 Td (Page {page_num}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, content.into_bytes()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => pages_id,
            "Resources" => resources_id,
            "Contents" => content_id,
            "MediaBox" => vec![0.into(), 0.into(), 612.into(), 792.into()],
        });
        kids.push(page_id.into());
    }

    doc.objects.insert(
        pages_id,
        Object::Dictionary(dictionary! {
            "Type" => "Pages",
            "Kids" => kids,
            "Count" => 2,
        }),
    );
    let catalog_id = doc.add_object(dictionary! {
        "Type" => "Catalog",
        "Pages" => pages_id,
    });
    doc.trailer.set("Root", catalog_id);
    doc.save(path).unwrap();
}

fn page_object(doc: &Document, page_num: u32) -> &lopdf::Dictionary {
    let page_id = doc.get_pages()[&page_num];
    doc.get_object(page_id).unwrap().as_dict().unwrap()
}

fn object_number(object: &Object) -> f32 {
    match object {
        Object::Integer(value) => *value as f32,
        Object::Real(value) => *value,
        other => panic!("expected numeric object, got {other:?}"),
    }
}

#[test]
fn pages_rotate_sets_rotation_on_selected_pages() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let output = temp.path().join("rotated.pdf");
    write_two_page_pdf(&input);

    let result = run_pdfp(&[
        "pages".to_string(),
        "rotate".to_string(),
        path_arg(&input),
        "--pages".to_string(),
        "1".to_string(),
        "--degrees".to_string(),
        "90".to_string(),
        "-o".to_string(),
        path_arg(&output),
    ]);

    assert!(
        result.status.success(),
        "pages rotate failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let doc = Document::load(&output).unwrap();
    assert_eq!(
        page_object(&doc, 1).get(b"Rotate").unwrap(),
        &Object::Integer(90)
    );
    assert!(page_object(&doc, 2).get(b"Rotate").is_err());
}

#[test]
fn page_crop_sets_crop_box_on_selected_pages() {
    let temp = TempDir::new().unwrap();
    let input = temp.path().join("input.pdf");
    let output = temp.path().join("cropped.pdf");
    write_two_page_pdf(&input);

    let result = run_pdfp(&[
        "page".to_string(),
        "crop".to_string(),
        path_arg(&input),
        "--pages".to_string(),
        "2".to_string(),
        "--box".to_string(),
        "10".to_string(),
        "20".to_string(),
        "300".to_string(),
        "400".to_string(),
        "-o".to_string(),
        path_arg(&output),
    ]);

    assert!(
        result.status.success(),
        "page crop failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&result.stdout),
        String::from_utf8_lossy(&result.stderr)
    );

    let doc = Document::load(&output).unwrap();
    assert!(page_object(&doc, 1).get(b"CropBox").is_err());
    let crop_box = page_object(&doc, 2)
        .get(b"CropBox")
        .unwrap()
        .as_array()
        .unwrap();
    let values: Vec<f32> = crop_box.iter().map(object_number).collect();
    assert_eq!(values, vec![10.0, 20.0, 300.0, 400.0]);
}
