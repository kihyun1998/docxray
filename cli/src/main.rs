//! Command-line adapter over the `docxray` core.
//!
//! Everything here is policy: argument parsing, file IO, where a sidecar is
//! placed, exit codes, terminal presentation. The mechanism lives in the core,
//! which knows nothing about paths (ADR-0002, ADR-0006).

use std::sync::LazyLock;

use clap::Parser;

/// The adapter's version alongside the core it is actually linked against.
/// The two are released together, but printing both makes the output evidence
/// of the link rather than a claim about it.
static VERSION: LazyLock<String> =
    LazyLock::new(|| format!("{} (core {})", env!("CARGO_PKG_VERSION"), docxray::VERSION));

#[derive(Parser)]
#[command(
    name = "docxray",
    version = VERSION.as_str(),
    about = "Surgical DOCX editing: project a Word document, edit the projection, patch the edits back."
)]
struct Cli {}

fn main() {
    let _cli = Cli::parse();
}
