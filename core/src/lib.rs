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

    /// The XML of a part could not be read.
    #[error("malformed XML: {0}")]
    Xml(#[from] quick_xml::Error),

    /// Reading bytes out of the package failed.
    #[error("could not read part: {0}")]
    Io(#[from] std::io::Error),
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
}

impl Projection {
    /// The sidecar content: every Anchor against the Block it came from.
    ///
    /// A Projection separated from its sidecar is meaningless, which is why
    /// applying one without it must fail loudly (ADR-0006).
    pub fn sidecar(&self) -> String {
        serde_json::to_string_pretty(&Sidecar {
            version: 1,
            anchors: &self.anchors,
        })
        .expect("anchors always serialise")
    }
}

#[derive(serde::Serialize)]
struct Sidecar<'a> {
    version: u32,
    anchors: &'a [Anchor],
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

    Ok(render(&paragraphs, &headings))
}

/// The path of a part sitting beside the main one, e.g. `word/styles.xml`.
fn sibling(main: &str, name: &str) -> String {
    match main.rfind('/') {
        Some(i) => format!("{}/{name}", &main[..i]),
        None => name.to_owned(),
    }
}

fn render(paragraphs: &[Paragraph], headings: &document::HeadingLevels) -> Projection {
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

    Projection { text, anchors }
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
