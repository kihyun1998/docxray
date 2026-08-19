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
