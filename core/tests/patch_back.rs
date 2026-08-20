//! Patch-back's central property, and the comparison harness it is proved with.
//!
//! Tests drive the core crate's public API, never the CLI (ADR-0008).

mod common;

use common::{every_fixture, fixture};
use docxray::PartDifference;

/// Rebuilds a package with one entry dropped, added or rewritten, so the
/// comparison harness can be shown to *fail* and not merely to pass. A harness
/// that reports "identical" for everything would make every test below green.
fn mutate(docx: &[u8], f: impl Fn(&str, &[u8]) -> Option<Vec<u8>>) -> Vec<u8> {
    use std::io::{Cursor, Read, Write};

    let mut archive = zip::ZipArchive::new(Cursor::new(docx.to_vec())).expect("a package");
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options: zip::write::FileOptions<'_, ()> =
        zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("an entry");
        let name = entry.name().to_owned();
        if entry.is_dir() {
            out.add_directory(&name, options).expect("directory");
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("readable entry");
        if let Some(bytes) = f(&name, &bytes) {
            out.start_file(&name, options).expect("start");
            out.write_all(&bytes).expect("write");
        }
    }
    out.finish().expect("finish").into_inner()
}

/// The harness's own floor: a document is identical to itself. Asserted across
/// the whole corpus, because "no differences" has to mean the same thing for a
/// package that is entirely STORED as for one that is entirely deflated.
#[test]
fn a_document_has_no_differences_from_itself() {
    for (name, bytes) in every_fixture() {
        let differences = docxray::compare_parts(&bytes, &bytes).expect("both are packages");
        assert!(
            differences.is_empty(),
            "{name} should be identical to itself, got: {differences:?}"
        );
    }
}

/// The discriminating half: each of the three kinds of difference is detected,
/// and named, so a report tells a reader which part to look at.
#[test]
fn a_dropped_part_is_named() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let produced = mutate(&original, |name, bytes| {
        (name != "word/styles.xml").then(|| bytes.to_vec())
    });

    let differences = docxray::compare_parts(&original, &produced).expect("both are packages");
    assert_eq!(
        differences,
        vec![PartDifference::Dropped("word/styles.xml".into())]
    );
}

#[test]
fn an_added_part_is_named() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let mut produced = mutate(&original, |_, bytes| Some(bytes.to_vec()));
    produced = {
        use std::io::Write;
        let mut out = zip::ZipWriter::new_append(std::io::Cursor::new(produced)).expect("append");
        out.start_file::<_, ()>("word/smuggled.xml", zip::write::FileOptions::default())
            .expect("start");
        out.write_all(b"<x/>").expect("write");
        out.finish().expect("finish").into_inner()
    };

    let differences = docxray::compare_parts(&original, &produced).expect("both are packages");
    assert_eq!(
        differences,
        vec![PartDifference::Added("word/smuggled.xml".into())]
    );
}

#[test]
fn a_rewritten_part_is_named() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let produced = mutate(&original, |name, bytes| {
        Some(if name == "word/document.xml" {
            let mut b = bytes.to_vec();
            b.extend_from_slice(b"<!-- -->");
            b
        } else {
            bytes.to_vec()
        })
    });

    let differences = docxray::compare_parts(&original, &produced).expect("both are packages");
    assert_eq!(
        differences,
        vec![PartDifference::Rewritten("word/document.xml".into())]
    );
}

/// Zip entry order, timestamps and compression level vary without meaning, so
/// they are not a comparison surface (ADR-0008). `mutate` re-deflates every
/// entry at a different level and rewrites its timestamp; the parts are the
/// same, so the documents compare equal.
#[test]
fn repacking_without_changing_a_part_is_not_a_difference() {
    let original = fixture("vendor/docx-rs/footnotes.docx");
    let produced = mutate(&original, |_, bytes| Some(bytes.to_vec()));

    assert_ne!(original, produced, "the zip bytes should genuinely differ");
    assert_eq!(
        docxray::compare_parts(&original, &produced).expect("both are packages"),
        vec![],
        "a repack that changes no part is not a difference"
    );
}

/// Directory entries (`word/`, `_rels/`) carry no content but are part of the
/// archive a consumer receives. Seven of the corpus's seventeen fixtures have
/// them, so dropping them is a real way to change a document.
#[test]
fn a_dropped_directory_entry_is_named() {
    use std::io::{Cursor, Read, Write};

    let original = fixture("vendor/docx-rs/footnotes.docx");
    let mut archive = zip::ZipArchive::new(Cursor::new(original.clone())).expect("a package");
    let mut out = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut dropped = Vec::new();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).expect("an entry");
        let name = entry.name().to_owned();
        if entry.is_dir() {
            dropped.push(name);
            continue;
        }
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).expect("readable");
        out.start_file::<_, ()>(&name, zip::write::FileOptions::default())
            .expect("start");
        out.write_all(&bytes).expect("write");
    }
    let produced = out.finish().expect("finish").into_inner();

    assert!(!dropped.is_empty(), "this fixture should carry directories");
    let differences = docxray::compare_parts(&original, &produced).expect("both are packages");
    assert_eq!(
        differences,
        dropped
            .into_iter()
            .map(PartDifference::Dropped)
            .collect::<Vec<_>>()
    );
}

// -- the central property ---------------------------------------------------

/// Identity: patch back with no edits and the document is unchanged (ADR-0008).
///
/// Asserted across the whole corpus rather than one chosen document, because
/// the fixtures differ in exactly the ways a repack can get wrong — entirely
/// STORED archives, directory entries, `[Content_Types].xml` in an unusual
/// position.
#[test]
fn patching_back_an_unedited_projection_changes_nothing() {
    for (name, original) in every_fixture() {
        let projection = docxray::open(&original).expect("should project");
        let produced = docxray::apply(&original, &projection.text, &projection.sidecar())
            .unwrap_or_else(|e| panic!("{name} should patch back: {e}"));

        assert_eq!(
            docxray::compare_parts(&original, &produced).expect("both are packages"),
            vec![],
            "{name} changed under an empty patch"
        );
    }
}

/// The Original's Fingerprint is recorded when the Projection is made, so that
/// applying can tell it is the same document (ADR-0006).
#[test]
fn the_sidecar_records_the_originals_fingerprint() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let projection = docxray::open(&original).expect("should project");
    let sidecar = projection.sidecar();

    assert!(
        sidecar.contains(&projection.original),
        "the sidecar should carry the Fingerprint: {sidecar}"
    );
    assert!(
        projection.original.starts_with("sha256:"),
        "the Fingerprint names its algorithm: {}",
        projection.original
    );

    let other = docxray::open(&fixture("vendor/docx-rs/paragraph.docx")).expect("should project");
    assert_ne!(
        projection.original, other.original,
        "different documents should not share a Fingerprint"
    );
}

/// A Stale Projection is refused and nothing is produced, rather than patched
/// against a document that moved underneath it (ADR-0006).
#[test]
fn a_projection_whose_original_moved_is_refused() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let projection = docxray::open(&original).expect("should project");
    let moved = fixture("vendor/docx-rs/paragraph.docx");

    let error = docxray::apply(&moved, &projection.text, &projection.sidecar())
        .expect_err("a moved Original should be refused");

    let said = error.to_string();
    assert!(
        said.contains("re-open"),
        "the error should say how to recover: {said}"
    );
    assert!(
        said.contains("sidecar"),
        "and name the other cause — a Projection paired with the wrong sidecar: {said}"
    );
}

/// The wording of a sidecar failure belongs to the core even though only a
/// consumer can notice a file is absent: its reader is an agent, and what it
/// says decides what the agent does next.
///
/// It says it in domain terms and never in paths or commands. Where a sidecar
/// lives is the consumer's policy, so a core error naming `.docxray/` would put
/// a CLI convention inside an API that has no filesystem (ADR-0002).
#[test]
fn a_sidecar_that_cannot_be_read_is_refused_with_a_reason() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let projection = docxray::open(&original).expect("should project");

    let error = docxray::apply(&original, &projection.text, "{ not json")
        .expect_err("an unreadable sidecar should be refused");
    let said = error.to_string();
    assert!(
        said.contains("sidecar") && said.contains("Anchors"),
        "the error should say what is lost when a sidecar cannot be read: {said}"
    );

    let missing = docxray::Error::SidecarMissing.to_string();
    assert!(
        missing.contains("Project the Original again"),
        "and a missing one should say how to get it back: {missing}"
    );
    for leak in [".docxray", "docxray open", ".dxr"] {
        assert!(
            !missing.contains(leak),
            "core error text must not carry consumer policy ({leak}): {missing}"
        );
    }
}

/// A sidecar this build does not understand is refused by version, rather than
/// read hopefully and resolved against the wrong shape.
#[test]
fn a_sidecar_from_another_version_is_refused() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let projection = docxray::open(&original).expect("should project");
    let future = projection
        .sidecar()
        .replace(r#""version": 1"#, r#""version": 99"#);

    let error = docxray::apply(&original, &projection.text, &future)
        .expect_err("an unknown sidecar version should be refused");
    assert!(
        error.to_string().contains("99"),
        "the error should name the version it found: {error}"
    );
}

/// Without this, an agent could edit a Projection, apply it, and be handed the
/// Original back with its work silently discarded under a success exit code.
/// Extracting edits is #7; refusing to pretend is this ticket's.
#[test]
fn an_edited_projection_is_refused_rather_than_quietly_ignored() {
    let original = fixture("vendor/docx-rs/hello_libre_office.docx");
    let projection = docxray::open(&original).expect("should project");
    let edited = projection.text.replace("Hello", "Goodbye");

    let error = docxray::apply(&original, &edited, &projection.sidecar())
        .expect_err("an edited Projection should be refused, not silently dropped");

    // Named, not sniffed for a substring: eight of the twelve error variants
    // contain the word "not" (counted, 2026-08-20), so `contains("not")` would
    // be satisfied by every refusal `apply` can produce — including the ones
    // this test exists to rule out.
    assert!(
        matches!(error, docxray::Error::EditNotSupported),
        "expected EditNotSupported, got: {error:?}"
    );
    let said = error.to_string();
    assert!(
        said.contains("edited") && said.contains("nothing was discarded"),
        "and it must promise the edits survive, or an agent will re-do the work: {said}"
    );
}

/// A Projection that made a round trip through an editor which rewrites line
/// endings has not been edited. Refusing it would be a refusal nobody earned.
#[test]
fn rewritten_line_endings_are_not_an_edit() {
    let original = fixture("vendor/docx-rs/paragraph.docx");
    let projection = docxray::open(&original).expect("should project");
    let crlf = projection.text.replace('\n', "\r\n");

    assert_ne!(crlf, projection.text, "the text should genuinely differ");
    let produced =
        docxray::apply(&original, &crlf, &projection.sidecar()).expect("should still apply");
    assert_eq!(
        docxray::compare_parts(&original, &produced).expect("both are packages"),
        vec![]
    );
}

/// Untouched entries are copied raw, keeping their compression method, and this
/// is asserted because nothing else can see it: part-by-part comparison is
/// deliberately blind to compression (ADR-0008), so re-deflating every entry
/// passes the identity test above. Measured by probe.
///
/// `footnotes.docx` is entirely STORED — the whole corpus's only such package —
/// so a repack that normalises to DEFLATE shows up here and nowhere else.
#[test]
fn an_untouched_part_keeps_its_compression() {
    use std::io::Cursor;

    let original = fixture("vendor/docx-rs/footnotes.docx");
    let projection = docxray::open(&original).expect("should project");
    let produced =
        docxray::apply(&original, &projection.text, &projection.sidecar()).expect("should apply");

    let before = zip::ZipArchive::new(Cursor::new(original)).expect("a package");
    let mut after = zip::ZipArchive::new(Cursor::new(produced)).expect("a package");

    assert_eq!(before.len(), after.len(), "entry count should not move");
    for i in 0..after.len() {
        let entry = after.by_index(i).expect("an entry");
        assert_eq!(
            entry.compression(),
            zip::CompressionMethod::Stored,
            "{} was recompressed",
            entry.name()
        );
    }
}

/// A package with two entries of the same name loses one of them the moment it
/// is read: the reader keys entries by name, so the collapse happens before any
/// check here can see it. Repacking then writes the survivor, `compare_parts`
/// compares the same collapsed view on both sides and reports nothing wrong,
/// and a part of the user's document is gone under a success.
///
/// Measured before this guard existed: ten entries in, nine out, exit zero, and
/// the *second* `word/document.xml` was the one the Projection showed.
#[test]
fn a_package_with_a_duplicated_part_name_is_refused() {
    use std::io::{Cursor, Write};

    let original = fixture("vendor/docx-rs/hello_libre_office.docx");

    // The zip writer refuses to emit a duplicate name outright, so the second
    // entry is written under a decoy of the *same length* and renamed at the
    // byte level afterwards. Equal length keeps every stored offset valid, so
    // the result is a genuine archive with two identically named entries rather
    // than a corrupted one.
    const DECOY: &[u8] = b"word/document.xmX";
    const REAL: &[u8] = b"word/document.xml";
    let smuggled = {
        let mut out =
            zip::ZipWriter::new_append(Cursor::new(original.clone())).expect("append to it");
        out.start_file::<_, ()>(
            std::str::from_utf8(DECOY).expect("ascii"),
            zip::write::FileOptions::default(),
        )
        .expect("start");
        out.write_all(b"<w:document/>").expect("write");
        let bytes = out.finish().expect("finish").into_inner();

        let mut renamed = Vec::with_capacity(bytes.len());
        let mut rest = bytes.as_slice();
        while let Some(at) = rest.windows(DECOY.len()).position(|w| w == DECOY) {
            renamed.extend_from_slice(&rest[..at]);
            renamed.extend_from_slice(REAL);
            rest = &rest[at + DECOY.len()..];
        }
        renamed.extend_from_slice(rest);
        renamed
    };

    // Sanity: the smuggled entry is genuinely there and genuinely invisible.
    let declared = zip::ZipArchive::new(Cursor::new(smuggled.clone())).expect("a package");
    assert_eq!(
        declared.len(),
        zip::ZipArchive::new(Cursor::new(original.clone()))
            .expect("a package")
            .len(),
        "the reader should collapse the duplicate, which is exactly the problem"
    );

    let error = docxray::open(&smuggled).expect_err("a duplicated part name should be refused");
    assert!(
        matches!(error, docxray::Error::DuplicatePart { .. }),
        "expected DuplicatePart, got: {error:?}"
    );
}

/// The guard reads the archive's declared entry count, so it must not fire on
/// any real document. Seventeen of them, including one whose trailing comment
/// and zip64 handling could confuse a byte scan.
#[test]
fn the_duplicate_guard_does_not_fire_on_any_real_document() {
    for (name, bytes) in every_fixture() {
        docxray::open(&bytes).unwrap_or_else(|e| panic!("{name} should still project: {e}"));
    }
}

/// The duplicate guard reads the end-of-central-directory record by scanning
/// bytes, which is a parser on the path that decides whether writing is allowed
/// at all. It must refuse malformed input, never panic on it — a slice index
/// off the end would abort the process instead of returning an error an agent
/// can act on.
#[test]
fn the_entry_count_scan_survives_malformed_input() {
    let valid = fixture("vendor/docx-rs/hello_libre_office.docx");

    let mut cases: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"PK".to_vec(),
        b"PK\x05\x06".to_vec(),
        vec![0xFF; 64],
        b"PK\x06\x06".to_vec(),
    ];
    // Every truncation of a real package, at a stride that lands inside headers,
    // payloads and the directory alike.
    for cut in (0..valid.len()).step_by(97) {
        cases.push(valid[..cut].to_vec());
    }
    // A package whose directory claims the maximum, sending the scan down the
    // zip64 path with no zip64 record present.
    let mut saturated = valid.clone();
    if let Some(at) = saturated.windows(4).rposition(|w| w == b"PK\x05\x06") {
        saturated[at + 10] = 0xFF;
        saturated[at + 11] = 0xFF;
    }
    cases.push(saturated);

    for (i, bytes) in cases.iter().enumerate() {
        // Any Result is fine; not returning at all is not.
        let _ = docxray::open(bytes);
        let _ = docxray::compare_parts(bytes, &valid);
        assert!(i < cases.len(), "unreachable, keeps the loop honest");
    }
}
