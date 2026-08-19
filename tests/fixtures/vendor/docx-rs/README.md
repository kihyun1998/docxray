# Vendored fixtures — docx-rs

Real `.docx` files taken from [bokuweb/docx-rs](https://github.com/bokuweb/docx-rs)'s
test corpus, MIT licensed. `LICENSE` in this directory is theirs, kept alongside
the files as the licence requires.

## Why vendor rather than author

Two reasons, and the second is the one that matters.

Authoring these by hand in Word is hours of tedious work — that is merely the
cheap reason. The real one is that **a fixture we generate ourselves cannot serve
as ground truth.** If we wrote a merged-cell document by emitting the XML we
believe Word emits, the round-trip test would pass against our own
misunderstanding, and the parser would inherit exactly the assumption the test
was supposed to challenge. These files were produced by real Word, real
LibreOffice and real Word Online, and a working project's suite depends on them,
so they open.

Selection was made against docxray's own acceptance criteria (#3), not by copying
the corpus wholesale. Inheriting another project's coverage decisions would mean
inheriting its blind spots, and accuracy is the thing docxray claims (ADR-0009).

## What is here

Contents verified by unzipping each file and counting elements, not inferred from
the file names.

| File | Size | Producer | Verified to contain |
| ---- | ---- | -------- | ------------------- |
| `comment.docx` | 5 KB | LibreOffice 6.0.7.3 (Linux) | comments part |
| `comment_in_delete_in_insert.docx` | 17 KB | Microsoft Office Word | ins 1, del 2, comments part, outlineLvl 9 in styles |
| `del_in_ins.docx` | 14 KB | Microsoft Office Word | ins 2, del 1 |
| `footnotes.docx` | 18 KB | — | comments part, footnotes part |
| `grid_after.docx` | 12 KB | Microsoft Office Word | 1 table, gridSpan 5, vMerge 4, shd 20 |
| `hello_libre_office.docx` | 4 KB | LibreOffice 6.0.7.3 (Linux) | prose only |
| `highlight_and_underline.docx` | 4 KB | LibreOffice 6.0.7.3 (Linux) | highlight 1 |
| `image_inline_and_anchor.docx` | 117 KB | Microsoft Office Word | 1 inline image, 1 anchored image |
| `indent_word_online.docx` | 11 KB | Word Online | **main part is `word/document2.xml`**, not `word/document.xml` |
| `nested_table.docx` | 11 KB | Microsoft Office Word | 2 tables, nested |
| `numbering.docx` | 5 KB | LibreOffice 6.0.7.3 (Linux) | numPr 5 |
| `outline_lvl.docx` | 15 KB | Microsoft Office Word | outlineLvl 7 in styles, numPr 2 |
| `paragraph.docx` | 5 KB | LibreOffice 6.2.8.2 (Linux) | numPr 2 |
| `table_docx.docx` | 6 KB | — | 1 table, footnotes part |
| `table_merged_libre_office.docx` | 4 KB | LibreOffice 6.0.7.3 (Linux) | 1 table, gridSpan 1, vMerge 5, shd 8 |

`grid_after.docx` and `table_merged_libre_office.docx` between them give merged
cells from **two different producers**, which matters because a merged cell is
the reason the Projection is not plain Markdown (ADR-0003) and Word and
LibreOffice do not lay out `gridSpan`/`vMerge` identically.

`outline_lvl.docx` is the counterpart to the war story in
`docs/agents/lessons.md`: a document that *does* carry outline levels, so
heading detection can be tested from both sides.

**East-Asian typography is covered, contrary to what this file first claimed.**
Thirteen of the sixteen fixtures carry east-Asian font state, and the claim that
none did was written twice before anyone checked. Verified by unzipping every
file:

- `del_in_ins.docx` sets `w:ascii="Arial"` with `w:eastAsia="Malgun Gothic"` —
  the Latin/Hangul split, in direct run formatting.
- `grid_after.docx` pairs `Century` with `ＭＳ 明朝` and carries `w:hint`.
- Six fixtures carry the meaningless style identifiers a localised Word emits —
  `a`, `a0`, `a3`, `1`, `10`, `20` — and eight carry localised style *names*
  (`見出し 1`, `索引`, `コメント文字列`).
- `outline_lvl.docx` has all three at once: `styleId="1"` with
  `w:name="見出し 1"` and an outline level. It is a Japanese Word document, and
  the mechanism a localised Word uses is the same whichever language it is.

`indent_word_online.docx` turned out to be the most valuable file here for a
reason its name does not suggest. Its main document part is `word/document2.xml`.
That is legal — the part name is not fixed by the specification; it is resolved
through the `officeDocument` relationship in `_rels/.rels` — and Word Online
writes it that way. Any parser that hardcodes `word/document.xml` opens every
other fixture in this directory and fails on this one.

## What this corpus does NOT cover

Named explicitly, because a gap nobody wrote down is a gap nobody fills:

- **No HWP-converted document.**
- **No generator export of the kind that omits `docProps` entirely** — the shape
  that produced the outlineLvl war story. `indent_word_online.docx` is
  generator-produced but keeps the standard parts.
- **Highlight appears only in a LibreOffice file.** Word-produced `w:highlight`
  is untested.
- These are small, single-purpose documents. Nothing here is a hundred pages
  long, so nothing here exercises `outline` at the scale that motivates it.

The gaps above are what remains of #3, and they need a person with Word.
