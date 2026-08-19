//! Reading the main document part into Blocks.

use quick_xml::NsReader;
use quick_xml::events::Event;
use quick_xml::name::ResolveResult;

use crate::Error;
use crate::package::W_NS;

/// One run of text with the formatting docxray represents inline.
#[derive(Debug, Default, PartialEq)]
pub(crate) struct Run {
    pub(crate) text: String,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
}

impl Run {
    /// Two runs are mergeable when they carry identical formatting. Word splits
    /// a single word into several runs for reasons that have nothing to do with
    /// formatting — spell-check state, revision identifiers — so runs are
    /// merged on read or the Projection would be full of arbitrary seams.
    fn same_format(&self, other: &Run) -> bool {
        self.bold == other.bold && self.italic == other.italic && self.underline == other.underline
    }
}

/// A paragraph: the unit a Projection line corresponds to.
#[derive(Debug, Default)]
pub(crate) struct Paragraph {
    pub(crate) style: Option<String>,
    pub(crate) runs: Vec<Run>,
}

/// Parses the main document part's body into paragraphs.
///
/// Anything not modelled is skipped rather than represented, which is what
/// makes it an Opaque Node: it never reaches a Projection, so an edit cannot
/// name it and therefore cannot damage it.
pub(crate) fn read_body(xml: &[u8]) -> Result<Vec<Paragraph>, Error> {
    let mut reader = NsReader::from_reader(xml);
    reader.config_mut().trim_text(false);

    let mut buf = Vec::new();
    let mut paragraphs = Vec::new();

    // Depth of drawing/pict content we are inside. Text within a drawing is in
    // another namespace and belongs to an Opaque Node either way.
    let mut opaque_depth = 0usize;
    // Depth of table content: tables are Blocks of their own and are not part
    // of this slice.
    let mut table_depth = 0usize;

    let mut para: Option<Paragraph> = None;
    let mut run: Option<Run> = None;
    let mut in_run_props = false;
    let mut in_text = false;

    loop {
        let (ns, event) = reader.read_resolved_event_into(&mut buf)?;
        let w = matches!(ns, ResolveResult::Bound(n) if n.as_ref() == W_NS);

        match event {
            Event::Eof => break,

            Event::Start(ref e) => {
                let local = e.local_name();
                let local = local.as_ref();
                if !w {
                    continue;
                }
                match local {
                    b"drawing" | b"pict" | b"object" => opaque_depth += 1,
                    b"tbl" => table_depth += 1,
                    _ if opaque_depth > 0 || table_depth > 0 => {}
                    b"p" => para = Some(Paragraph::default()),
                    b"r" => run = Some(Run::default()),
                    b"rPr" => in_run_props = true,
                    b"t" => in_text = true,
                    _ => {}
                }
            }

            Event::Empty(ref e) => {
                if !w || opaque_depth > 0 || table_depth > 0 {
                    continue;
                }
                let local = e.local_name();
                match local.as_ref() {
                    b"pStyle" => {
                        if let (Some(p), Some(v)) = (para.as_mut(), attr(e, b"val")) {
                            p.style = Some(v);
                        }
                    }
                    b"b" if in_run_props => set(&mut run, |r| r.bold = on(e)),
                    b"i" if in_run_props => set(&mut run, |r| r.italic = on(e)),
                    b"u" if in_run_props => {
                        let val = attr(e, b"val");
                        set(&mut run, |r| {
                            r.underline = val.as_deref().unwrap_or("single") != "none"
                        });
                    }
                    _ => {}
                }
            }

            Event::Text(ref t) if in_text && opaque_depth == 0 && table_depth == 0 => {
                if let Some(r) = run.as_mut() {
                    r.text.push_str(&t.decode()?);
                }
            }

            Event::End(ref e) => {
                let local = e.local_name();
                let local = local.as_ref();
                if !w {
                    continue;
                }
                match local {
                    b"drawing" | b"pict" | b"object" => {
                        opaque_depth = opaque_depth.saturating_sub(1)
                    }
                    b"tbl" => table_depth = table_depth.saturating_sub(1),
                    _ if opaque_depth > 0 || table_depth > 0 => {}
                    b"t" => in_text = false,
                    b"rPr" => in_run_props = false,
                    b"r" => {
                        if let (Some(p), Some(r)) = (para.as_mut(), run.take()) {
                            push_run(&mut p.runs, r);
                        }
                    }
                    b"p" => {
                        if let Some(p) = para.take() {
                            paragraphs.push(p);
                        }
                    }
                    _ => {}
                }
            }

            _ => {}
        }
        buf.clear();
    }

    Ok(paragraphs)
}

fn push_run(runs: &mut Vec<Run>, run: Run) {
    if run.text.is_empty() {
        return;
    }
    match runs.last_mut() {
        Some(last) if last.same_format(&run) => last.text.push_str(&run.text),
        _ => runs.push(run),
    }
}

fn set(run: &mut Option<Run>, f: impl FnOnce(&mut Run)) {
    if let Some(r) = run.as_mut() {
        f(r);
    }
}

/// A toggle property is on unless it explicitly says otherwise.
fn on(e: &quick_xml::events::BytesStart<'_>) -> bool {
    !matches!(attr(e, b"val").as_deref(), Some("0" | "false"))
}

fn attr(e: &quick_xml::events::BytesStart<'_>, name: &[u8]) -> Option<String> {
    e.attributes().flatten().find_map(|a| {
        (a.key.local_name().as_ref() == name)
            .then(|| String::from_utf8_lossy(a.value.as_ref()).into_owned())
    })
}

/// What `styles.xml` says about which styles are headings, and at what level.
#[derive(Debug, Default)]
pub(crate) struct HeadingLevels {
    styles: std::collections::HashMap<String, StyleInfo>,
}

#[derive(Debug, Default)]
struct StyleInfo {
    name: Option<String>,
    outline: Option<u8>,
    based_on: Option<String>,
}

impl HeadingLevels {
    /// Resolves a style identifier to a heading level, 0 being the top.
    ///
    /// A cascade rather than a single signal, because no single signal holds.
    /// `w:outlineLvl` is the most reliable and a real generated document turned
    /// out to carry none at all; `w:styleId` is usually English but a localised
    /// Word emits `a3` or `2`; `w:name` is translated (`見出し 1`, `제목 1`).
    /// Each rung catches documents the one above it misses.
    pub(crate) fn level_of(&self, style_id: &str) -> Option<u8> {
        let mut current = style_id;
        for _ in 0..16 {
            let info = self.styles.get(current)?;
            if let Some(level) = info.outline {
                return (level <= 8).then_some(level);
            }
            if let Some(level) = builtin_level(current) {
                return Some(level);
            }
            if let Some(level) = info.name.as_deref().and_then(name_level) {
                return Some(level);
            }
            current = info.based_on.as_deref()?;
        }
        None
    }
}

/// `Heading1`..`Heading9`, the identifiers an English Word writes.
fn builtin_level(style_id: &str) -> Option<u8> {
    let n = style_id
        .strip_prefix("Heading")
        .or_else(|| style_id.strip_prefix("heading"))?;
    n.parse::<u8>()
        .ok()
        .filter(|n| (1..=9).contains(n))
        .map(|n| n - 1)
}

/// The English style *name* survives in documents whose identifiers do not.
fn name_level(name: &str) -> Option<u8> {
    let n = name.trim().strip_prefix("heading ")?;
    n.parse::<u8>()
        .ok()
        .filter(|n| (1..=9).contains(n))
        .map(|n| n - 1)
}

/// Reads the style definitions a Projection needs to recognise headings.
pub(crate) fn heading_levels(xml: &[u8]) -> Result<HeadingLevels, Error> {
    let mut out = HeadingLevels::default();
    if xml.is_empty() {
        return Ok(out);
    }

    let mut reader = NsReader::from_reader(xml);
    let mut buf = Vec::new();
    let mut current: Option<(String, StyleInfo)> = None;

    loop {
        let (ns, event) = reader.read_resolved_event_into(&mut buf)?;
        let w = matches!(ns, ResolveResult::Bound(n) if n.as_ref() == W_NS);

        match event {
            Event::Eof => break,
            Event::Start(ref e) if w && e.local_name().as_ref() == b"style" => {
                if let Some(id) = attr(e, b"styleId") {
                    current = Some((id, StyleInfo::default()));
                }
            }
            Event::Empty(ref e) if w => {
                let Some((_, info)) = current.as_mut() else {
                    buf.clear();
                    continue;
                };
                match e.local_name().as_ref() {
                    b"name" => info.name = attr(e, b"val"),
                    b"basedOn" => info.based_on = attr(e, b"val"),
                    b"outlineLvl" => info.outline = attr(e, b"val").and_then(|v| v.parse().ok()),
                    _ => {}
                }
            }
            Event::End(ref e) if w && e.local_name().as_ref() == b"style" => {
                if let Some((id, info)) = current.take() {
                    out.styles.insert(id, info);
                }
            }
            _ => {}
        }
        buf.clear();
    }

    Ok(out)
}
