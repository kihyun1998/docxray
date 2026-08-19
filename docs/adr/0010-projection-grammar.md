# The Projection grammar

ADR-0003 settled that a Projection is hybrid Markdown anchored at Block level. This fixes the syntax.

Every decision below was checked by rendering real fixtures — `grid_after.docx` (Word) and `table_merged_libre_office.docx` (LibreOffice) — into the proposed form and confirming the OOXML geometry could be reconstructed from the result. The worked examples are that output, not illustrations written by hand.

## Anchors are trailing HTML comments

An Anchor is an HTML comment, and it is the last thing on its Block's line:

```markdown
# Quarterly report <!--p11-->
Revenue rose **12%** against the prior year. <!--p12-->
- First item <!--p13-->
- Second item <!--p14-->
```

HTML comments are invisible when the Projection is rendered, cannot collide with Markdown syntax, and are what the prior art uses — a design convention, where the tie-breaker gives prior art the vote.

Trailing rather than on its own line, because an anchor line between list items interrupts the list in CommonMark, and a prefix before `#` stops a heading being a heading. Trailing is the one position that works for every Block type with a single rule, and ADR-0003's premise is that the agent will eventually get the syntax wrong, so one rule with no exceptions beats a shorter file.

**Paragraphs are never hard-wrapped in a Projection.** One Block, one line — which is what makes "last thing on the line" unambiguous.

A Block with no Anchor is an `Insert` (ADR-0007). An agent writing a new paragraph writes plain Markdown and nothing else.

## Style handles are deviation-only

A Block carries formatting information only where it deviates from what its style already implies:

```markdown
Figure 3 — quarterly revenue <!--p31 style=Caption-->
```

Same comment as the Anchor, so there is one marker per Block. Prior art again: a Projection that restates every Block's full formatting is mostly noise, and noise is what the agent spends attention on instead of the text.

Run-level formatting has no handles at all. Bold, italic and underline round-trip through Markdown; everything else falls to the Block's Dominant Format (ADR-0003).

## Tables come in two forms

Real tables are overwhelmingly simple. In the vendored corpus and in the first real document we inspected — 3 tables, 42 cells — the count of merged cells was zero. So the common case keeps ordinary Markdown, and only tables that need more pay for it.

### Pipe form

Used when every row occupies the full grid, no cell spans more than one column or row, and every cell holds at most one paragraph and no nested table. A GFM table with Anchors trailing inside each cell:

```markdown
<!--t0-->
| Hello <!--t0:r0c0--> |  <!--t0:r0c1--> |  <!--t0:r0c2--> |
| --- | --- | --- |
|  <!--t0:r1c0--> |  <!--t0:r1c1--> |  <!--t0:r1c2--> |
```

*(rendered from `table_docx.docx`)*

### Grid form

Used for everything else. Still a pipe table, so the two-dimensional shape survives, but each cell Anchor carries the geometry: `span=`, `vmerge=start|cont`, and the row's `before=`/`after=` ride the row's first and last cell.

```markdown
<!--t0 cols=5-->
|  <!--t0:r0c0 span=5--> |
|  <!--t0:r1c0 span=3 vmerge=start--> |  <!--t0:r1c3 vmerge=start after=1--> |
|  <!--t0:r2c0 span=3 vmerge=cont--> |  <!--t0:r2c3 vmerge=cont after=1--> |
|  <!--t0:r3c0 span=2--> |  <!--t0:r3c2 after=2--> |
|  <!--t0:r4c0--> |  <!--t0:r4c1 span=2 after=2--> |
|  <!--t0:r5c0--> |  <!--t0:r5c1--> |  <!--t0:r5c2 after=2--> |
```

*(rendered from `grid_after.docx`; rows 6–8 repeat row 5)*

```markdown
<!--t0 cols=3-->
|  <!--t0:r0c0 span=2--> |  <!--t0:r0c2 vmerge=start--> |
|  <!--t0:r1c0 vmerge=start--> |  <!--t0:r1c1--> |  <!--t0:r1c2 vmerge=cont--> |
|  <!--t0:r2c0 vmerge=cont--> |  <!--t0:r2c1--> |  <!--t0:r2c2 vmerge=cont--> |
```

*(rendered from `table_merged_libre_office.docx`)*

**`c` is the grid column, not the cell's position in the row.** In the first example row 1 has two cells, addressed `c0` and `c3`, because the first spans three columns. Numbering cells sequentially would make an Anchor change meaning when a neighbouring span changes.

**Ragged rows are real and had to be designed for.** `grid_after.docx` has a five-column grid where most rows occupy three, declaring the shortfall with `w:gridAfter`. Neither a Markdown table nor a naive "cells plus spans" model can express that; `before=`/`after=` exist because a real Word document required them.

The form is information-complete: grid width, each cell's column, span, vertical-merge state, and each row's leading and trailing gap all reconstruct the original geometry.

### Detached cells

A cell that cannot be written inside a pipe cell — more than one paragraph, or a nested table — is marked and its content follows the table as ordinary anchored Blocks:

```markdown
|  <!--t0:r1c0 detached--> |  <!--t0:r1c1--> |

Opening line of that cell. <!--t0:r1c0:p0-->
Second paragraph of the same cell. <!--t0:r1c0:p1-->
```

A nested table is emitted as its own anchored table block in the same position, and the containing cell carries `contains=t1`.

This exists because rendering `nested_table.docx` through an earlier draft of this grammar silently flattened its nested table into two sibling tables. Nothing in the draft could express a table inside a cell, and a real fixture found it within minutes.

## Considered options

- **Pandoc-style `{#p12}` attributes** — rejected. Some Markdown renderers interpret them, and they are not what the prior art uses.
- **A single table form, always explicit** — rejected. It would tax every ordinary table for a case that, in the corpus we have, never occurs.
- **A non-Markdown block language for tables** — rejected. It discards the agent's table-editing instincts, which is the accuracy ADR-0003 exists to keep.
- **Sequential cell numbering** — rejected. An Anchor would change meaning when a neighbouring cell's span changed.

## Consequences

- The Projection parser must accept both table forms, and the renderer must choose between them per table. The choice is a pure function of the table's geometry, so it is not a decision the agent ever makes.
- Grid form is where a span mismatch becomes possible, so it is where validation has to be strictest — an edit whose `span` values no longer sum to the grid width is refused, never repaired (ADR-0003).
- One line per Block means a Projection's line count is its Block count, which is what makes `outline`'s line numbers usable for a ranged read (ADR-0006).
- Nothing here gives an agent a way to name a run, a section, or an unmodelled element. That is deliberate: what the grammar cannot address, an edit cannot damage.
