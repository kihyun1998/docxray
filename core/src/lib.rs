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

mod document;
mod package;

use document::Paragraph;
use package::Package;

/// The version of this crate, as declared in its manifest.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Anything that stops a document being projected.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The bytes are not a readable package.
    #[error("not a readable .docx package: {0}")]
    Package(#[from] zip::result::ZipError),

    /// A part the package declares is not in the archive.
    #[error("package declares a part that is missing: {0}")]
    MissingPart(String),

    /// No `officeDocument` relationship, so there is no main document part.
    #[error("package has no officeDocument relationship, so no main part")]
    NoMainPart,

    /// The package's directory declares more entries than are addressable, so
    /// at least one part name appears twice. OPC part names are unique, and
    /// resolving a duplicate is a guess about which one the producer meant.
    #[error(
        "package declares {declared} entries but only {addressable} can be addressed, so a part name appears more than once and neither copy can be trusted"
    )]
    DuplicatePart {
        /// What the central directory claims.
        declared: u64,
        /// How many distinct names that leaves.
        addressable: usize,
    },

    /// A part was offered for replacement that the package does not contain.
    #[error("cannot replace {0}: the package has no such part")]
    UnknownPart(String),

    /// The XML of a part could not be read.
    #[error("malformed XML: {0}")]
    Xml(#[from] quick_xml::Error),

    /// Reading bytes out of the package failed.
    #[error("could not read part: {0}")]
    Io(#[from] std::io::Error),

    /// The sidecar could not be read, so every Anchor binds to nothing.
    #[error("the sidecar cannot be read ({0}), so this Projection's Anchors bind to nothing")]
    Sidecar(String),

    /// There is no sidecar. Only a consumer can notice a file is absent, but
    /// what the agent reading the error should do next is the core's to say.
    #[error(
        "this Projection has no sidecar, so its Anchors bind to nothing. Project the Original again to make both"
    )]
    SidecarMissing,

    /// The Projection was not made from the Original it is being applied to.
    ///
    /// Both Fingerprints are carried, not just rendered, so an adapter can
    /// report them without parsing this sentence back apart.
    #[error(
        "this Projection was not made from the Original given: it records {recorded}, and the Original is {found}. Either the Original changed since it was projected, and the Projection must be re-opened, or this Projection has been paired with another document's sidecar. Nothing was written"
    )]
    Stale {
        /// The Fingerprint the sidecar carries.
        recorded: String,
        /// The Fingerprint of the Original actually given.
        found: String,
    },

    /// The Projection carries edits, and applying them is not implemented yet.
    #[error(
        "this Projection has been edited, and applying edits is not implemented in this version. Nothing was written, and nothing was discarded"
    )]
    EditNotSupported,

    /// The produced document differs from the Original where it should not.
    ///
    /// Every part is compared today, because with no edits every part is
    /// expected to be identical. Once edits are applied, the parts an edit
    /// legitimately rewrites have to be excluded, or this refuses exactly the
    /// writes it exists to permit.
    ///
    /// The differences are carried as themselves rather than as prose: an
    /// adapter that has to act on *which* parts moved should not have to read
    /// the sentence back (`docs/agents/theflow.md`, Step 2).
    #[error("refusing to hand back a document that changed where nothing was edited: {}", render_differences(.0))]
    UntouchedPartsChanged(Vec<PartDifference>),
}

/// Joins differences into the one sentence an error renders them as.
fn render_differences(differences: &[PartDifference]) -> String {
    differences
        .iter()
        .map(|d| d.to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

impl From<quick_xml::encoding::EncodingError> for Error {
    fn from(e: quick_xml::encoding::EncodingError) -> Self {
        Error::Xml(quick_xml::Error::Encoding(e))
    }
}

/// The identity a Block carries in a Projection.
///
/// Anchors are issued in Block order when the Projection is made, and are valid
/// for that Projection's lifetime only — re-opening an Original issues fresh
/// ones, which is correct rather than a defect (ADR-0007).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Anchor {
    /// The identifier as it appears in the Projection, e.g. `p0`.
    pub id: String,
    /// Position of the Block in the Original's body.
    pub block: usize,
}

/// A text view of an Original, written for an agent to read and edit.
///
/// Lossy by design and always disposable: it is a cache, never truth.
#[derive(Debug)]
pub struct Projection {
    /// The `.dxr` content.
    pub text: String,
    /// Every Anchor the text carries, in Block order.
    pub anchors: Vec<Anchor>,
    /// The Fingerprint of the Original this Projection was made from, as
    /// `sha256:<hex>`. Applying compares it and refuses a Stale Projection.
    pub original: String,
}

impl Projection {
    /// The sidecar content: every Anchor against the Block it came from.
    ///
    /// A Projection separated from its sidecar is meaningless, which is why
    /// applying one without it must fail loudly (ADR-0006).
    pub fn sidecar(&self) -> String {
        serde_json::to_string_pretty(&Sidecar {
            version: SIDECAR_VERSION,
            original: &self.original,
            anchors: &self.anchors,
        })
        .expect("anchors always serialise")
    }
}

/// The sidecar shape this build writes and understands.
const SIDECAR_VERSION: u32 = 1;

#[derive(serde::Serialize)]
struct Sidecar<'a> {
    version: u32,
    original: &'a str,
    anchors: &'a [Anchor],
}

#[derive(serde::Deserialize)]
struct StoredSidecar {
    version: u32,
    original: String,
    anchors: Vec<Anchor>,
}

/// The Fingerprint of an Original: what a Projection records so that applying
/// can tell the document has not moved underneath it.
///
/// Taken over the raw bytes rather than over normalised part content, and the
/// asymmetry is the reason. A Word save with no visible edit still rewrites
/// `document.xml` with a fresh revision session, so "the bytes changed" and
/// "the Blocks may have moved" are nearly the same claim. A Fingerprint that is
/// too eager costs one `open`; one that is too forgiving patches a document
/// that moved.
fn fingerprint(docx: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("sha256:{:x}", Sha256::digest(docx))
}

/// One way a produced document differs from the Original it came from.
///
/// The comparison surface is the package's decompressed parts, not its zip
/// bytes: entry order, timestamps and compression level vary without meaning
/// and would report differences nobody made (ADR-0008).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PartDifference {
    /// A part the Original has and the produced document does not.
    Dropped(String),
    /// A part the produced document has and the Original does not.
    Added(String),
    /// A part in both, whose bytes differ.
    Rewritten(String),
}

impl std::fmt::Display for PartDifference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Dropped(name) => write!(f, "{name} is missing"),
            Self::Added(name) => write!(f, "{name} was added"),
            Self::Rewritten(name) => write!(f, "{name} was rewritten"),
        }
    }
}

/// Compares two documents part by part, reporting every part that differs.
///
/// This is how fidelity is asserted in tests *and* how a write is guarded at
/// run time: `apply` runs it over its own output before returning, so a
/// document damaged in a way no test anticipated is refused rather than handed
/// back (ADR-0006). The two being the same code is the point — the check is
/// nearly free, and it reaches documents the fixture corpus does not.
///
/// Differences are reported in the Original's part order, with parts the
/// Original never had last.
pub fn compare_parts(original: &[u8], produced: &[u8]) -> Result<Vec<PartDifference>, Error> {
    let before = Package::open(original)?.parts()?;
    let after = Package::open(produced)?.parts()?;

    let after_by_name: std::collections::HashMap<&str, &Vec<u8>> =
        after.iter().map(|(n, b)| (n.as_str(), b)).collect();
    let before_names: std::collections::HashSet<&str> =
        before.iter().map(|(n, _)| n.as_str()).collect();

    let mut out = Vec::new();
    for (name, bytes) in &before {
        match after_by_name.get(name.as_str()) {
            None => out.push(PartDifference::Dropped(name.clone())),
            Some(other) if *other != bytes => out.push(PartDifference::Rewritten(name.clone())),
            Some(_) => {}
        }
    }
    for (name, _) in &after {
        if !before_names.contains(name.as_str()) {
            out.push(PartDifference::Added(name.clone()));
        }
    }
    Ok(out)
}

/// Patches a Projection's edits back into the Original, returning the bytes of
/// the resulting document.
///
/// The Original is never modified: this returns a document, and where it lands
/// is the consumer's decision (ADR-0006).
///
/// Applying is refused rather than half-done. A Projection whose Original has
/// moved is Stale; a sidecar that cannot be read leaves every Anchor bound to
/// nothing; and the produced document is compared part by part against the
/// Original before it is handed back, so a defect in docxray cannot return a
/// damaged file.
///
/// **This version applies no edits.** Extracting Patch Operations from an
/// edited Projection is the next slice; until it exists an edited Projection is
/// refused, because handing back the Original under a success would discard the
/// agent's work without saying so.
pub fn apply(original: &[u8], projection: &str, sidecar: &str) -> Result<Vec<u8>, Error> {
    let stored: StoredSidecar =
        serde_json::from_str(sidecar).map_err(|e| Error::Sidecar(e.to_string()))?;
    if stored.version != SIDECAR_VERSION {
        return Err(Error::Sidecar(format!(
            "it is version {}, and this build understands version {SIDECAR_VERSION}",
            stored.version
        )));
    }

    // Re-projecting is what makes everything below comparable: the same
    // Original always yields the same text and the same Anchors (ADR-0007).
    let current = open(original)?;

    if stored.original != current.original {
        return Err(Error::Stale {
            recorded: stored.original,
            found: current.original,
        });
    }
    if stored.anchors != current.anchors {
        // Not "it belongs to another document" — a foreign sidecar would have
        // failed the Fingerprint check above, so that cause is already ruled
        // out and naming it would send an agent to check its paths instead of
        // re-projecting. What is left is a sidecar that was damaged in place.
        return Err(Error::Sidecar(
            "it records different Anchors from the ones this Original produces, so it has been edited or truncated since it was written"
                .to_owned(),
        ));
    }

    // A Projection that went through an editor which rewrites line endings has
    // not been edited, and refusing it would be a refusal nobody earned.
    if projection.replace("\r\n", "\n") != current.text {
        return Err(Error::EditNotSupported);
    }

    let produced = Package::open(original)?.repack(&std::collections::HashMap::new())?;

    // ADR-0006's verify-before-write. With nothing replaced it compares raw
    // copies against themselves and cannot fire — it is the right shape holding
    // its place, not evidence, and it starts earning its keep the moment a part
    // is actually rewritten.
    let differences = compare_parts(original, &produced)?;
    if !differences.is_empty() {
        return Err(Error::UntouchedPartsChanged(differences));
    }
    Ok(produced)
}

/// Projects a `.docx` into a Projection.
pub fn open(docx: &[u8]) -> Result<Projection, Error> {
    let mut package = Package::open(docx)?;
    let main = package.main_part_name()?;
    let body = package.part(&main)?;
    let paragraphs = document::read_body(&body)?;

    let styles = package
        .part_opt(&sibling(&main, "styles.xml"))
        .unwrap_or_default();
    let headings = document::heading_levels(&styles)?;

    Ok(render(&paragraphs, &headings, fingerprint(docx)))
}

/// The path of a part sitting beside the main one, e.g. `word/styles.xml`.
fn sibling(main: &str, name: &str) -> String {
    match main.rfind('/') {
        Some(i) => format!("{}/{name}", &main[..i]),
        None => name.to_owned(),
    }
}

fn render(
    paragraphs: &[Paragraph],
    headings: &document::HeadingLevels,
    original: String,
) -> Projection {
    let mut text = String::new();
    let mut anchors = Vec::with_capacity(paragraphs.len());

    for (i, para) in paragraphs.iter().enumerate() {
        let id = format!("p{i}");

        if let Some(level) = para.style.as_deref().and_then(|s| headings.level_of(s)) {
            text.push_str(&"#".repeat(level as usize + 1));
            text.push(' ');
        }

        for run in &para.runs {
            text.push_str(&emphasise(run));
        }

        // The Anchor is the last thing on the line, separated by one space
        // (ADR-0010). Trailing whitespace in the content would put two there
        // and is not meaningful in Markdown anyway.
        while text.ends_with(' ') {
            text.pop();
        }
        text.push_str(" <!--");
        text.push_str(&id);
        text.push_str("-->\n");

        anchors.push(Anchor { id, block: i });
    }

    Projection {
        text,
        anchors,
        original,
    }
}

/// Bold, italic and underline round-trip through Markdown; everything else
/// falls to the Block's Dominant Format (ADR-0003).
fn emphasise(run: &document::Run) -> String {
    let mut open = String::new();
    let mut close = String::new();
    if run.bold {
        open.push_str("**");
        close.insert_str(0, "**");
    }
    if run.italic {
        open.push('*');
        close.insert(0, '*');
    }
    if run.underline {
        open.push_str("__");
        close.insert_str(0, "__");
    }
    format!("{open}{}{close}", run.text)
}
