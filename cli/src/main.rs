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
        /// Overwrite a Projection that carries changes of its own.
        #[arg(long)]
        force: bool,
    },
    /// Patch a Projection's edits back into the document it came from.
    Apply {
        /// The `.dxr` to apply. Its Original and sidecar are found beside it.
        file: PathBuf,
        /// Overwrite an existing output that is not what this run produces.
        #[arg(long)]
        force: bool,
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
        Command::Open { file, force } => open(&file, force),
        Command::Apply { file, force } => apply(&file, force),
    }
}

/// Writes through a temporary file in the same directory, then renames.
///
/// A plain write truncates its target before it fills it, so an interruption
/// leaves a shortened file where a document was. Both references do it this way
/// — docxengine `mkstemp` + `os.replace`, docx-cli write-then-rename — and
/// ADR-0006's premise is that a `.docx` is often the only copy of something.
/// Writes, refusing to destroy a file that is not already what we are about to
/// write.
///
/// Not "refuse if it exists": re-projecting is the documented recovery from a
/// Stale Projection, and re-applying is the ordinary edit loop, so a bare
/// existence check would refuse the two things a user is most often doing.
/// Comparing against the *content* costs nothing here — `open` and `apply` are
/// both deterministic, so a rerun that changed nothing writes identical bytes
/// and passes silently — and it refuses exactly the case worth refusing: a file
/// holding something this run did not produce.
///
/// What that protects, measured before the guard existed: `open report.docx`
/// replaced an edited `report.dxr` with a fresh Projection, and `apply` replaced
/// an unrelated 9241-byte `report.out.docx` with its output. Both under exit 0.
fn write_guarded(path: &Path, bytes: &[u8], force: bool, hint: &str) -> Result<(), String> {
    if !force
        && let Ok(existing) = std::fs::read(path)
        && existing != bytes
    {
        return Err(format!(
            "refusing to overwrite {}: it holds something this run did not produce ({hint}). Pass --force to replace it",
            path.display()
        ));
    }
    write_atomically(path, bytes)
}

fn write_atomically(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_os_string();
    tmp.push(".docxray-tmp");
    let tmp = PathBuf::from(tmp);

    std::fs::write(&tmp, bytes).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("cannot write {}: {e}", path.display())
    })
}

/// Where a document's sidecar lives, given its Projection. The coupling is
/// invisible from the `.dxr` itself, so every message about it names the path
/// (ADR-0006).
///
/// The `.json` is **appended** to the stem rather than swapped in with
/// `with_extension`, which would replace everything after the *last* dot and so
/// collapse `contract.v1` and `contract.v2` onto one sidecar. That is not a
/// hypothetical filename: `apply` writes `report.out.docx`, so projecting the
/// tool's own output would overwrite the sidecar of the document it came from.
/// One sidecar per document is the whole point (ADR-0006) — a Projection
/// carrying another document's Anchors is worse than one carrying none.
fn sidecar_path(dxr: &Path) -> Result<PathBuf, String> {
    let stem = dxr
        .file_stem()
        .ok_or_else(|| format!("{} has no file name", dxr.display()))?;
    let mut name = stem.to_os_string();
    name.push(".json");
    Ok(dxr
        .parent()
        .unwrap_or(Path::new("."))
        .join(SIDECAR_DIR)
        .join(name))
}

fn apply(dxr: &Path, force: bool) -> Result<(), String> {
    let projection =
        std::fs::read_to_string(dxr).map_err(|e| format!("cannot read {}: {e}", dxr.display()))?;

    // The Original sits beside its Projection under the same stem, which is
    // where `open` put it.
    let original_path = dxr.with_extension("docx");
    let original = std::fs::read(&original_path)
        .map_err(|e| format!("cannot read {}: {e}", original_path.display()))?;

    let sidecar_path = sidecar_path(dxr)?;
    let sidecar = match std::fs::read_to_string(&sidecar_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            // The core says what is wrong and what has to happen; the adapter
            // adds the path and the command, which are its own policy.
            return Err(format!(
                "{}\n  expected it at {}\n  run: docxray open {}",
                docxray::Error::SidecarMissing,
                sidecar_path.display(),
                original_path.display()
            ));
        }
        Err(e) => return Err(format!("cannot read {}: {e}", sidecar_path.display())),
    };

    let produced = docxray::apply(&original, &projection, &sidecar).map_err(|e| e.to_string())?;

    // Never over the Original: a `.docx` is often the only copy of something
    // (ADR-0006). Overwriting will need an explicit flag.
    let out = dxr.with_extension("out.docx");
    write_guarded(&out, &produced, force, "a different document")?;

    println!("{} -> {}", dxr.display(), out.display());
    Ok(())
}

fn open(file: &Path, force: bool) -> Result<(), String> {
    let bytes = std::fs::read(file).map_err(|e| format!("cannot read {}: {e}", file.display()))?;
    let projection = docxray::open(&bytes).map_err(|e| e.to_string())?;

    let dxr = file.with_extension("dxr");
    // Reading a document already named `.dxr` would otherwise write the
    // Projection straight over the document it was made from, and report
    // success doing it.
    if dxr == file {
        return Err(format!(
            "refusing to project {} onto itself: its Projection would be written to the same path. Rename it to .docx first",
            file.display()
        ));
    }
    write_guarded(
        &dxr,
        projection.text.as_bytes(),
        force,
        "an edited Projection, or one made from another document",
    )?;

    // One sidecar per document rather than a single shared file: two documents
    // projected in the same directory would otherwise overwrite each other's
    // Anchors, and a Projection whose sidecar belongs to another document is
    // worse than one with no sidecar at all.
    let sidecar = sidecar_path(&dxr)?;
    let dir = sidecar.parent().unwrap_or(Path::new("."));
    std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    write_guarded(
        &sidecar,
        projection.sidecar().as_bytes(),
        force,
        "another document's Anchors",
    )?;

    println!(
        "{} — {} blocks -> {}",
        file.display(),
        projection.anchors.len(),
        dxr.display()
    );
    Ok(())
}
