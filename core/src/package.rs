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

        // OPC part names are unique. A zip that breaks that is not merely odd:
        // the reader keys entries by name, so a duplicate silently collapses to
        // one of them *before* anything here can see it — and a repack would
        // then drop an entry from the user's document while every check
        // downstream compared the same collapsed view on both sides and found
        // nothing wrong. Measured on a package built with two
        // `word/document.xml` entries: ten in, nine out, exit zero.
        //
        // Counting what the archive *declares* is the only view that survives
        // the collapse, which is why this reads the central directory's own
        // total rather than asking the reader.
        if let Some(declared) = declared_entry_count(bytes)
            && declared != archive.len() as u64
        {
            return Err(Error::DuplicatePart {
                declared,
                addressable: archive.len(),
            });
        }
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

    /// Every entry in archive order, decompressed: the surface a package is
    /// compared on (ADR-0008). Directory entries are included with no content,
    /// because dropping one changes the archive a consumer receives even
    /// though it carries nothing.
    ///
    /// Names are unique by the time they reach here — the reader keys entries
    /// by name, so duplicates have already collapsed. Checking for them *here*
    /// would be a guard that cannot fire; `Package::open` does it against the
    /// central directory's declared total instead, which is the only view that
    /// still sees both copies.
    pub(crate) fn parts(&mut self) -> Result<Vec<(String, Vec<u8>)>, Error> {
        let mut out: Vec<(String, Vec<u8>)> = Vec::with_capacity(self.archive.len());

        for i in 0..self.archive.len() {
            let mut entry = self.archive.by_index(i)?;
            let name = entry.name().to_owned();
            let mut bytes = Vec::new();
            if !entry.is_dir() {
                entry.read_to_end(&mut bytes)?;
            }
            out.push((name, bytes));
        }
        Ok(out)
    }

    /// Rewrites the package, taking each part's bytes from `replace` where it
    /// offers one and copying the original entry untouched where it does not.
    ///
    /// An untouched entry is copied **raw**: its already-compressed bytes are
    /// streamed through, so name, position, compression method, CRC and DOS
    /// timestamp all survive and media is never recompressed. Prior art
    /// normalises instead; see issue #6 for why that reasoning does not
    /// transfer.
    ///
    /// **The archive is not byte-identical, only its parts are.** Measured: a
    /// rewritten package loses each entry's local and central *extra fields*
    /// (Word's `0xA220` Growth Hint, Info-ZIP's `0x5455`/`0x7875`), the
    /// deflate-level hint in the general-purpose flags, and the version-made-by
    /// and external-attribute bytes — 1832 of 25412 bytes on
    /// `korean-generated-export-long.docx`. Two consequences are worth naming
    /// rather than filing under "metadata": a **directory entry comes back
    /// marked as a regular file** (`0x41ed0000` becomes `0x81ed0010`), and the
    /// attribute bytes are chosen from the *build host*, so the same document
    /// rewritten on Windows and on Linux does not produce the same archive.
    /// Neither reaches a reader — a directory is still a zero-length entry with
    /// a trailing slash — and none of it is reachable by `compare_parts`, which
    /// compares decompressed part content by design (ADR-0008). Say "untouched
    /// part content", not "untouched content", when quoting this guarantee.
    pub(crate) fn repack(
        &mut self,
        replace: &std::collections::HashMap<String, Vec<u8>>,
    ) -> Result<Vec<u8>, Error> {
        use std::io::Write;

        // Every replacement must name a part that exists, or a caller could ask
        // for a write, receive success, and get a document without it — and the
        // part-by-part check could not tell, because a part that was never
        // written is a part that never differed. Unreachable while `apply`
        // passes nothing; #7 is the first caller that fills this map.
        for name in replace.keys() {
            if self.archive.index_for_name(name).is_none() {
                return Err(Error::UnknownPart(name.clone()));
            }
        }

        let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));

        for i in 0..self.archive.len() {
            let entry = self.archive.by_index(i)?;
            let name = entry.name().to_owned();

            match replace.get(&name) {
                None => out.raw_copy_file(entry)?,
                Some(bytes) => {
                    let options = zip::write::SimpleFileOptions::default()
                        .compression_method(entry.compression())
                        .last_modified_time(entry.last_modified().unwrap_or_default());
                    drop(entry);
                    out.start_file(&name, options)?;
                    out.write_all(bytes)?;
                }
            }
        }

        Ok(out.finish()?.into_inner())
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

/// How many entries the archive's own directory claims, read straight from the
/// end-of-central-directory record.
///
/// `None` when the record cannot be located, which is left to the zip reader to
/// reject — refusing a document on the strength of a byte scan that found
/// nothing would turn a hardening check into a new way to lose a file.
fn declared_entry_count(bytes: &[u8]) -> Option<u64> {
    const EOCD: &[u8] = b"PK\x05\x06";
    const ZIP64_EOCD: &[u8] = b"PK\x06\x06";

    // The record is last, but a trailing comment of up to 65535 bytes may
    // follow it.
    let from = bytes.len().saturating_sub(65_557);
    let eocd = (from..bytes.len().saturating_sub(21))
        .rev()
        .find(|&i| bytes[i..].starts_with(EOCD))?;

    let total = u16::from_le_bytes([*bytes.get(eocd + 10)?, *bytes.get(eocd + 11)?]);
    if total != u16::MAX {
        return Some(u64::from(total));
    }

    // The 16-bit field is saturated, so the real count lives in the zip64
    // record. Find it the same way rather than trusting the locator's offset,
    // which is relative to a disk layout we do not model.
    let z64 = (0..eocd)
        .rev()
        .find(|&i| bytes[i..].starts_with(ZIP64_EOCD))?;
    let field = bytes.get(z64 + 32..z64 + 40)?;
    Some(u64::from_le_bytes(field.try_into().ok()?))
}

fn attr(tag: &str, name: &str) -> Option<String> {
    let key = format!("{name}=\"");
    let start = tag.find(&key)? + key.len();
    let rest = &tag[start..];
    let end = rest.find('"')?;
    Some(rest[..end].to_owned())
}
