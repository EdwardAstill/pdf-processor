//! Installer script regression tests.

use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use std::process::Command;

fn write_executable(path: &Path, contents: &str) {
    fs::write(path, contents).unwrap();
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

#[test]
fn arch_ocr_install_does_not_request_aur_only_ocrmypdf_from_pacman() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let work = tempfile::tempdir().unwrap();
    let bin = work.path().join("bin");
    fs::create_dir(&bin).unwrap();

    std::os::unix::fs::symlink("/usr/bin/mktemp", bin.join("mktemp")).unwrap();
    std::os::unix::fs::symlink("/usr/bin/rm", bin.join("rm")).unwrap();

    write_executable(&bin.join("pacman"), "#!/usr/bin/bash\nexit 0\n");
    write_executable(
        &bin.join("sudo"),
        "#!/usr/bin/bash\nprintf '%s\\n' \"$*\" > \"$PDFP_TEST_SUDO_LOG\"\nexit 0\n",
    );

    let sudo_log = work.path().join("sudo.log");
    let script = root.join("scripts/install.sh");
    let output = Command::new("/usr/bin/bash")
        .arg("-c")
        .arg(format!(
            "source {:?}; install_ocr_deps",
            script.to_string_lossy()
        ))
        .env("PATH", &bin)
        .env("PDFP_TEST_SUDO_LOG", &sudo_log)
        .env("PDFP_INSTALL_SCRIPT_TESTING", "1")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "install_ocr_deps should not fail when Arch OCRmyPDF is absent\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let sudo_args = fs::read_to_string(&sudo_log).unwrap();
    assert!(
        sudo_args.contains("pacman -S --needed --noconfirm"),
        "expected pacman install command, got: {sudo_args}"
    );
    assert!(
        !sudo_args.contains("ocrmypdf"),
        "pacman should not be asked to install AUR-only ocrmypdf: {sudo_args}"
    );
    for package in ["tesseract", "tesseract-data-eng", "qpdf", "ghostscript"] {
        assert!(
            sudo_args.contains(package),
            "expected pacman install command to include {package}: {sudo_args}"
        );
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("ocrmypdf AUR package"),
        "expected Arch AUR guidance, got: {stderr}"
    );
}
