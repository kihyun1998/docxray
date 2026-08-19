//! Surgical DOCX editing for AI agents.
//!
//! A Word document (the **Original**) is projected into an editable text view
//! (a **Projection**); edits made there are patched back into the Original by
//! rewriting only the nodes that changed, so untouched content stays
//! byte-identical. See `CONTEXT.md` for the vocabulary and `docs/adr/` for the
//! decisions behind it.
//!
//! This crate is the product; the `docxray` command-line tool is one adapter
//! over it. The API deliberately knows nothing about files or paths — it takes
//! and returns bytes — so the same engine runs under WebAssembly and behind
//! foreign-language bindings (ADR-0002).

#![forbid(unsafe_code)]
#![warn(missing_docs)]

/// The version of this crate, as declared in its manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
