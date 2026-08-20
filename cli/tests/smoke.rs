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

/// A scratch directory holding a copy of a fixture, so a test never writes
/// anywhere near the corpus.
fn scratch(name: &str, fixture: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("docxray-smoke-{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    let mut source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source.pop();
    source.push("tests/fixtures");
    source.push(fixture);
    std::fs::copy(&source, dir.join("report.docx")).expect("copy fixture");
    dir
}

fn docxray(args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_docxray"))
        .args(args)
        .output()
        .expect("binary should run")
}

/// The whole loop through the adapter: project a document, patch it back with
/// no edits, and the Original is still sitting there untouched beside a new
/// file (ADR-0006 — `apply` never overwrites).
#[test]
fn apply_writes_a_new_document_and_leaves_the_original_alone() {
    let dir = scratch("apply", "vendor/docx-rs/hello_libre_office.docx");
    let docx = dir.join("report.docx");
    let before = std::fs::read(&docx).expect("the original");

    let opened = docxray(&["open", docx.to_str().expect("utf-8 path")]);
    assert!(
        opened.status.success(),
        "open failed: {}",
        String::from_utf8_lossy(&opened.stderr)
    );

    let dxr = dir.join("report.dxr");
    let applied = docxray(&["apply", dxr.to_str().expect("utf-8 path")]);
    assert!(
        applied.status.success(),
        "apply failed: {}",
        String::from_utf8_lossy(&applied.stderr)
    );

    let produced = std::fs::read(dir.join("report.out.docx")).expect("a new document is written");
    assert_eq!(
        docxray::compare_parts(&before, &produced).expect("both are packages"),
        vec![],
        "the new document should be identical to the Original, part by part"
    );
    assert_eq!(
        std::fs::read(&docx).expect("the original"),
        before,
        "the Original itself must not be touched"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// The sidecar coupling is invisible from the `.dxr`, so losing it has to fail
/// loudly and say where the file should have been (ADR-0006). The core supplies
/// the sentence; the adapter supplies the path and the command, because both
/// are its policy.
#[test]
fn apply_refuses_loudly_when_the_sidecar_is_gone() {
    let dir = scratch("nosidecar", "vendor/docx-rs/hello_libre_office.docx");
    let docx = dir.join("report.docx");

    let opened = docxray(&["open", docx.to_str().expect("utf-8 path")]);
    assert!(opened.status.success(), "open should succeed");
    std::fs::remove_dir_all(dir.join(".docxray")).expect("remove the sidecar");

    let dxr = dir.join("report.dxr");
    let applied = docxray(&["apply", dxr.to_str().expect("utf-8 path")]);

    assert!(!applied.status.success(), "should exit non-zero");
    let said = String::from_utf8_lossy(&applied.stderr).to_string();
    assert!(
        said.contains("no sidecar"),
        "should say what is wrong: {said}"
    );
    assert!(
        said.contains(".docxray") && said.contains("docxray open"),
        "should name where it looked and how to get it back: {said}"
    );
    assert!(
        !dir.join("report.out.docx").exists(),
        "nothing should be written when applying is refused"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Two documents whose names differ only after the first dot must not share a
/// sidecar. Swapping in a `.json` extension replaces everything after the *last*
/// dot, which silently collapsed `contract.v1` and `contract.v2` onto one file —
/// and `apply` writes `report.out.docx`, so projecting the tool's own output
/// destroyed the sidecar of the document it came from.
///
/// ADR-0006 calls a Projection carrying another document's Anchors worse than
/// one carrying none, because those Anchors resolve — to the wrong nodes.
#[test]
fn documents_with_dots_in_their_names_keep_separate_sidecars() {
    let dir = std::env::temp_dir().join("docxray-smoke-dotted");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    let mut corpus = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    corpus.pop();
    corpus.push("tests/fixtures/vendor/docx-rs");

    // Deliberately different documents: if they shared a sidecar, the second
    // `open` would overwrite the first's Anchors and the first `apply` would
    // refuse as Stale.
    std::fs::copy(
        corpus.join("hello_libre_office.docx"),
        dir.join("contract.v1.docx"),
    )
    .expect("copy fixture");
    std::fs::copy(corpus.join("paragraph.docx"), dir.join("contract.v2.docx"))
        .expect("copy fixture");

    for name in ["contract.v1.docx", "contract.v2.docx"] {
        let out = docxray(&["open", dir.join(name).to_str().expect("utf-8 path")]);
        assert!(
            out.status.success(),
            "open {name} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    for name in ["contract.v1.json", "contract.v2.json"] {
        assert!(
            dir.join(".docxray").join(name).exists(),
            "each document should get its own sidecar; {name} is missing"
        );
    }

    // The consequence, not just the filename: both must still apply. Sharing a
    // sidecar is caught by the Fingerprint, so the symptom is a refusal here.
    for name in ["contract.v1.dxr", "contract.v2.dxr"] {
        let out = docxray(&["apply", dir.join(name).to_str().expect("utf-8 path")]);
        assert!(
            out.status.success(),
            "apply {name} failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    let _ = std::fs::remove_dir_all(&dir);
}

/// The same collision reached through the tool's own naming: `apply` produces
/// `report.out.docx`, and projecting that must not disturb `report.docx`.
#[test]
fn projecting_our_own_output_does_not_disturb_the_first_sidecar() {
    let dir = scratch("selfoutput", "vendor/docx-rs/hello_libre_office.docx");
    let docx = dir.join("report.docx");

    assert!(
        docxray(&["open", docx.to_str().expect("utf-8 path")])
            .status
            .success(),
        "open should succeed"
    );
    let first = std::fs::read_to_string(dir.join(".docxray/report.json")).expect("a sidecar");

    assert!(
        docxray(&[
            "apply",
            dir.join("report.dxr").to_str().expect("utf-8 path")
        ])
        .status
        .success(),
        "apply should succeed"
    );
    assert!(
        docxray(&[
            "open",
            dir.join("report.out.docx").to_str().expect("utf-8 path")
        ])
        .status
        .success(),
        "projecting the output should succeed"
    );

    assert_eq!(
        std::fs::read_to_string(dir.join(".docxray/report.json")).expect("still there"),
        first,
        "the first document's sidecar must be untouched"
    );
    assert!(
        dir.join(".docxray/report.out.json").exists(),
        "the output should get a sidecar of its own"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// `open` writes its Projection to the input's path with the extension swapped
/// for `.dxr`. Handed a document already called `.dxr`, that is the same path —
/// so it overwrote the document with its own Projection and exited zero.
/// Measured: 4132 bytes of Word document became 16 bytes of Markdown.
#[test]
fn open_refuses_to_project_a_document_onto_itself() {
    let dir = std::env::temp_dir().join("docxray-smoke-selfwrite");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("temp dir");

    let mut source = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    source.pop();
    source.push("tests/fixtures/vendor/docx-rs/hello_libre_office.docx");
    let victim = dir.join("mydoc.dxr");
    std::fs::copy(&source, &victim).expect("copy fixture");
    let before = std::fs::read(&victim).expect("the document");

    let out = docxray(&["open", victim.to_str().expect("utf-8 path")]);

    assert!(!out.status.success(), "should exit non-zero");
    assert_eq!(
        std::fs::read(&victim).expect("the document"),
        before,
        "the document must not have been overwritten by its own Projection"
    );

    let _ = std::fs::remove_dir_all(&dir);
}

/// Outputs are written through a temporary file and renamed, so an interrupted
/// write cannot leave a shortened file where a document was. The observable
/// part is that no temporary survives a successful run.
#[test]
fn writing_leaves_no_temporary_behind() {
    let dir = scratch("atomic", "vendor/docx-rs/hello_libre_office.docx");
    let docx = dir.join("report.docx");

    assert!(
        docxray(&["open", docx.to_str().expect("utf-8 path")])
            .status
            .success(),
        "open should succeed"
    );
    assert!(
        docxray(&[
            "apply",
            dir.join("report.dxr").to_str().expect("utf-8 path")
        ])
        .status
        .success(),
        "apply should succeed"
    );

    let mut strays = Vec::new();
    for base in [dir.clone(), dir.join(".docxray")] {
        for entry in std::fs::read_dir(&base).expect("readable") {
            let path = entry.expect("an entry").path();
            if path.to_string_lossy().contains("docxray-tmp") {
                strays.push(path);
            }
        }
    }
    assert!(strays.is_empty(), "temporaries left behind: {strays:?}");

    let _ = std::fs::remove_dir_all(&dir);
}
