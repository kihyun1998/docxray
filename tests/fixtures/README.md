# Fixture corpus

Real `.docx` files the test seam runs against (ADR-0008). Every file here must be
safe to publish.

Fixtures come in two kinds, because one file cannot be both minimal and
realistic:

- **Minimal fixtures** isolate one axis each, so a failing test points at one cause.
- **Wild fixtures** carry the mess a parser actually breaks on — revision-id
  fragments, meaningless style identifiers, unused styles, missing standard parts.

## `vendor/docx-rs/`

Fifteen fixtures taken from [docx-rs](https://github.com/bokuweb/docx-rs)'s test
corpus (MIT). Produced by real Word, LibreOffice and Word Online, so they are
ground truth in a way a document we generated ourselves could never be — see that
directory's README for the reasoning, the verified inventory, and the list of
what it does **not** cover.

## Ours

| File | What it exercises | Origin |
| ---- | ----------------- | ------ |
| `korean-generated-export.docx` | Generated-export package shape: `[Content_Types].xml` last in the archive, no `docProps`, and **`Heading1`–`Heading6` defined with no `w:outlineLvl` at all** — the document behind the outline-level war story. Korean run fonts, 95 KB of `numbering.xml`, one table, highlight and shading, 226 `rsid` attributes. 26 paragraphs | Trimmed from a real Korean product manual, 40 MB → 9 KB. Opens in Word |
| `korean-generated-export-long.docx` | The same package at working scale: **498 paragraphs, 54 headings across five levels**, 3 tables, 59 list items, 33 highlights, 43 shadings, 4344 `rsid` attributes. This is what `outline` and ranged reading are for; nothing else in the corpus is bigger than 26 paragraphs | Same original, every non-image block kept. 16 KB. Opens in Word |

Still needed:

| Needed | Why nothing above covers it |
| ------ | --------------------------- |
| A document converted from HWP | Absent from every public corpus checked. Needs a person with HWP |

East-Asian typography is **not** a gap, contrary to what this file claimed twice
before anyone counted: thirteen of the vendored fixtures carry east-Asian font
state, `del_in_ins.docx` pairs `w:ascii="Arial"` with `w:eastAsia="Malgun Gothic"`,
and six carry the meaningless style identifiers a localised Word emits. See
`vendor/docx-rs/README.md`.

### How `korean-generated-export.docx` was derived

The recipe matters, because the first attempt produced a file Word refused.

1. **Copy the original**; never trim in place.
2. **Splice the XML as text.** Do not parse and re-serialise `document.xml` — see the war story in `docs/agents/lessons.md`.
3. Keep one paragraph per distinct style, a few list items, a couple carrying highlight and shading, and one table. Drop every paragraph containing an image.
4. **Append a paragraph if the body would otherwise end with a table.** Word rejects a body ending in `</w:tbl>` even though the schema permits it.
5. Replace the text of every `<w:t>` **with the same number of characters**, keeping run boundaries. Filling each node with a whole sentence regardless of its original size inflates paragraphs until they wrap, and 104 of this document's paragraphs are justified — so the earlier lines of a wrapped paragraph get stretched across the page. Preserving length keeps line breaking, and therefore run splitting, close to the real document.
6. Prune relationships the trimmed document no longer references, then the media they pointed at.
7. **Open the result in Word.** Nothing before this step is evidence.

## Sanitising, before anything of ours is committed

This repository is public, and a `.docx` carries more than it shows on screen.

1. Turn tracked changes off and accept all changes **before** trimming — deleted
   text otherwise survives inside deletion markup and gets published.
2. Remove comments unless the fixture exists to exercise comments.
3. Clear author, company and other document properties.
4. Check the **unzipped XML**, not the rendered document.

Untrimmed originals live in `tests/fixtures-local/`, which is gitignored. Trim
and sanitise there; save the result here.

## The inventory

Every fixture is listed with what it exercises and where it came from — vendored
ones in `vendor/docx-rs/README.md`, ours in the table above once they exist.
State what a file actually contains, verified by unzipping it, rather than what
its name suggests.
