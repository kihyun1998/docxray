//! Command-line adapter over the `docxray` core.
//!
//! Everything here is policy: argument parsing, file IO, where a sidecar is
//! placed, exit codes, terminal presentation. The mechanism lives in the core,
//! which knows nothing about paths (ADR-0002, ADR-0006).

use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::LazyLock;

use clap::{Parser, Subcommand};

/// The adapter's version alongside the core it is actually linked against.
/// The two are released together, but printing both makes the output evidence
/// of the link rather than a claim about it.
static VERSION: LazyLock<String> =
    LazyLock::new(|| format!("{} (core {})", env!("CARGO_PKG_VERSION"), docxray::VERSION));

/// Where sidecars live, relative to the working directory.
const SIDECAR_DIR: &str = ".docxray";

#[derive(Parser)]
#[command(
    name = "docxray",
    version = VERSION.as_str(),
    about = "Surgical DOCX editing: project a Word document, edit the projection, patch the edits back."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Project a document into an editable text view.
    Open {
        /// The `.docx` to project.
        file: PathBuf,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let Some(command) = cli.command else {
        // No subcommand is not an error; clap has already had its chance at
        // --help and --version.
        return ExitCode::SUCCESS;
    };

    match run(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("docxray: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), String> {
    match command {
        Command::Open { file } => open(&file),
    }
}

fn open(file: &Path) -> Result<(), String> {
    let bytes = std::fs::read(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let projection = docxray::open(&bytes).map_err(|e| e.to_string())?;

    let stem = file
        .file_stem()
        .ok_or_else(|| format!("{} has no file name", file.display()))?;

    let dxr = file.with_extension("dxr");
    std::fs::write(&dxr, &projection.text)
        .map_err(|e| format!("cannot write {}: {e}", dxr.display()))?;

    // One sidecar per document rather than a single shared file: two documents
    // projected in the same directory would otherwise overwrite each other's
    // Anchors, and a Projection whose sidecar belongs to another document is
    // worse than one with no sidecar at all.
    let dir = dxr.parent().unwrap_or(Path::new(".")).join(SIDECAR_DIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    let sidecar = dir.join(stem).with_extension("json");
    std::fs::write(&sidecar, projection.sidecar())
        .map_err(|e| format!("cannot write {}: {e}", sidecar.display()))?;

    println!(
        "{} — {} blocks -> {}",
        file.display(),
        projection.anchors.len(),
        dxr.display()
    );
    Ok(())
}
