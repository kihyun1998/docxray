//! The OPC layer: a `.docx` is a zip of parts wired together by relationships.

use std::io::{Cursor, Read};

use crate::Error;

/// The wordprocessingml namespace. Everything docxray models lives here, and
/// checking it is not pedantry: a drawing embeds `a:t` text elements from the
/// drawingml namespace, which a parser matching on local names alone would
/// happily emit into a Projection.
pub(crate) const W_NS: &[u8] = b"http://schemas.openxmlformats.org/wordprocessingml/2006/main";

const OFFICE_DOCUMENT: &str =
    "http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument";

/// A `.docx` opened for reading.
pub(crate) struct Package {
    archive: zip::ZipArchive<Cursor<Vec<u8>>>,
}

impl Package {
    pub(crate) fn open(bytes: &[u8]) -> Result<Self, Error> {
        let archive = zip::ZipArchive::new(Cursor::new(bytes.to_vec()))?;
        Ok(Self { archive })
    }

    pub(crate) fn part(&mut self, name: &str) -> Result<Vec<u8>, Error> {
        let mut file = self
            .archive
            .by_name(name)
            .map_err(|_| Error::MissingPart(name.to_owned()))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        Ok(buf)
    }

    pub(crate) fn part_opt(&mut self, name: &str) -> Option<Vec<u8>> {
        self.part(name).ok()
    }

    /// The main document part, found the way the specification says: follow the
    /// `officeDocument` relationship. It is **not** always `word/document.xml`
    /// — Word Online writes `word/document2.xml`, and a parser that hardcodes
    /// the name opens most documents and fails on those.
    pub(crate) fn main_part_name(&mut self) -> Result<String, Error> {
        let rels = self.part("_rels/.rels")?;
        let rels = String::from_utf8_lossy(&rels);

        for tag in rels.split('<') {
            if !tag.starts_with("Relationship ") {
                continue;
            }
            if attr(tag, "Type").as_deref() != Some(OFFICE_DOCUMENT) {
                continue;
            }
            if let Some(target) = attr(tag, "Target") {
                return Ok(target.trim_start_matches('/').to_owned());
            }
        }
        Err(Error::NoMainPart)
    }
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}
