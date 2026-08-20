//! Tests drive the core crate's public API, never the CLI (ADR-0008).

mod common;

use common::fixture;

/// The smallest possible whole path: a one-paragraph document becomes one
/// anchored line of Markdown.
#[test]
fn a_single_paragraph_becomes_one_anchored_line() {
    let p = docxray::open(&fixture("vendor/docx-rs/hello_libre_office.docx"))
        .expect("fixture should project");

    let lines: Vec<&str> = p.text.lines().collect();
    assert_eq!(lines[0], "Hello <!--p0-->");

    // The Fingerprint rides the *last* line, so Block N stays line N and
    // `outline`'s ranged reads are unaffected (ADR-0010).
    assert_eq!(
        lines.len(),
        p.anchors.len() + 1,
        "one line per Block, then the Fingerprint"
    );
    assert_eq!(
        lines[lines.len() - 1],
        format!("<!--docxray original={}-->", p.original)
    );
    assert_eq!(p.anchors.len(), 1, "one Block, one Anchor");
    assert_eq!(p.anchors[0].id, "p0");
}

/// Anchors are issued in Block order, and every Block gets one (ADR-0007).
#[test]
fn anchors_are_issued_in_block_order() {
    let p = docxray::open(&fixture("vendor/docx-rs/paragraph.docx")).expect("should project");

    let ids: Vec<_> = p.anchors.iter().map(|a| a.id.as_str()).collect();
    assert_eq!(ids, ["p0", "p1", "p2", "p3"]);
    assert!(
        p.text.starts_with("Hello <!--p0-->\nWorld <!--p1-->\n"),
        "got: {:?}",
        p.text
    );
}

/// Outline level is the most reliable heading signal, and this document's style
/// identifiers are meaningless (`2`, `4`, `5`) so nothing else would resolve it.
#[test]
fn headings_resolve_through_outline_level() {
    let p = docxray::open(&fixture("vendor/docx-rs/outline_lvl.docx")).expect("should project");
    let lines: Vec<&str> = p.text.lines().collect();

    assert_eq!(lines[0], "## <!--p0-->", "styleId 2 carries outlineLvl 1");
    assert_eq!(
        lines[1], "#### A <!--p1-->",
        "styleId 4 carries outlineLvl 3"
    );
    assert_eq!(
        lines[3], "##### B <!--p3-->",
        "styleId 5 carries outlineLvl 4"
    );
}

/// The counterpart: a real generated export whose Heading styles carry no
/// outline level at all, so the cascade has to fall through to the identifier.
#[test]
fn headings_resolve_when_the_document_has_no_outline_levels() {
    let p = docxray::open(&fixture("korean-generated-export.docx")).expect("should project");

    for level in 1..=5 {
        let prefix = format!("{} ", "#".repeat(level));
        assert!(
            p.text.lines().any(|l| l.starts_with(&prefix)),
            "expected a level-{level} heading, got:\n{}",
            p.text
        );
    }
}

/// Word splits a run for reasons unrelated to formatting — spell-check state,
/// revision identifiers. This document's first paragraph is five separate bold
/// runs; unmerged they would emit their markers five times over.
#[test]
fn adjacent_runs_with_the_same_format_are_merged() {
    let p = docxray::open(&fixture("korean-generated-export-long.docx")).expect("should project");
    let first = p.text.lines().next().expect("a first line");

    assert_eq!(first, "**예시 예시 문** <!--p0-->");
    assert_eq!(
        first.matches("**").count(),
        2,
        "five bold runs should become one emphasis span, not five"
    );
}

/// Bold, italic and underline round-trip through Markdown (ADR-0003).
#[test]
fn emphasis_appears_as_markdown() {
    let p = docxray::open(&fixture("korean-generated-export-long.docx")).expect("should project");
    assert!(p.text.contains("**"), "bold should render as Markdown");
}

/// The main part is resolved through the `officeDocument` relationship, not by
/// name: Word Online writes `word/document2.xml`.
#[test]
fn the_main_part_is_found_by_relationship_not_by_name() {
    let p = docxray::open(&fixture("vendor/docx-rs/indent_word_online.docx"))
        .expect("a document2.xml package should project");
    assert!(!p.anchors.is_empty(), "should have found Blocks");
}

/// A drawing carries text in another namespace. It is an Opaque Node: it must
/// never reach a Projection, because what an edit cannot name it cannot damage.
#[test]
fn drawing_content_never_reaches_the_projection() {
    let p = docxray::open(&fixture("vendor/docx-rs/image_inline_and_anchor.docx"))
        .expect("should project");

    assert!(
        !p.text.contains("<w:"),
        "no raw markup should leak: {:?}",
        p.text
    );
    for anchor in &p.anchors {
        assert!(anchor.id.starts_with('p'), "only paragraphs in this slice");
    }
}
