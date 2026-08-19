use std::process::Command;

/// `docxray --version` reports both the binary's version and the core library's,
/// so the output is evidence that the CLI is genuinely linked against the core
/// rather than merely sitting next to it in the workspace (ADR-0002).
#[test]
fn version_reports_cli_and_core() {
    let out = Command::new(env!("CARGO_BIN_EXE_docxray"))
        .arg("--version")
        .output()
        .expect("binary should run");

    assert!(out.status.success(), "--version should exit 0");
    let stdout = String::from_utf8(out.stdout).expect("output should be utf-8");

    assert!(
        stdout.contains(env!("CARGO_PKG_VERSION")),
        "should report the CLI version, got: {stdout:?}"
    );
    assert!(
        stdout.contains(&format!("core {}", docxray::VERSION)),
        "should report the linked core version, got: {stdout:?}"
    );
    assert!(
        out.stderr.is_empty(),
        "--version is not an error path, but wrote to stderr: {:?}",
        String::from_utf8_lossy(&out.stderr)
    );
}

/// The CLI is a thin adapter, so it gets smoke checks rather than a seam of its
/// own (ADR-0008): the Projection and its sidecar land where they are promised.
#[test]
fn open_writes_a_projection_and_its_sidecar() {
    let dir = std::env::temp_dir().join("docxray-smoke-open");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    let mut fixture = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    fixture.pop();
    fixture.push("tests/fixtures/vendor/docx-rs/hello_libre_office.docx");
    let copy = dir.join("hello.docx");
    std::fs::copy(&fixture, &copy).expect("copy fixture");

    let out = Command::new(env!("CARGO_BIN_EXE_docxray"))
        .args(["open", copy.to_str().expect("utf-8 path")])
        .output()
        .expect("binary should run");

    assert!(
        out.status.success(),
        "open failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let dxr = std::fs::read_to_string(dir.join("hello.dxr")).expect("projection written");
    assert_eq!(dxr, "Hello <!--p0-->\n");

    let sidecar =
        std::fs::read_to_string(dir.join(".docxray/hello.json")).expect("sidecar written");
    assert!(
        sidecar.contains("\"p0\""),
        "sidecar records the Anchor: {sidecar}"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// A file that is not a package fails loudly, with a non-zero exit code an
/// agent or a script can act on.
#[test]
fn open_refuses_something_that_is_not_a_package() {
    let dir = std::env::temp_dir().join("docxray-smoke-bad");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");
    let bad = dir.join("not-a.docx");
    std::fs::write(&bad, b"this is not a zip").expect("write");

    let out = Command::new(env!("CARGO_BIN_EXE_docxray"))
        .args(["open", bad.to_str().expect("utf-8 path")])
        .output()
        .expect("binary should run");

    assert!(!out.status.success(), "should exit non-zero");
    assert!(
        !out.stderr.is_empty(),
        "should say what went wrong on stderr"
    );
    assert!(
        !dir.join("not-a.dxr").exists(),
        "nothing should be written when projection fails"
    );

    let _ = std::fs::remove_dir_all(&dir);
}
