# docxray

Surgical DOCX editing for AI agents. A Word document is projected into an editable text view; edits made there are patched back into the original file, leaving untouched content byte-identical.

## Language

### The document and its views

**Original**:
The `.docx` file the user brought. It is the single source of truth at all times and is never regenerated from anything else.
_Avoid_: source file, input

**Projection**:
The `.dxr` text view of an Original, written for an AI agent to read and edit. Lossy by design and always disposable — it is a cache, never truth.
_Avoid_: conversion, export, markdown output, intermediate format

**Patch-back**:
Applying the edits made in a Projection to the Original by rewriting only the nodes that changed.
_Avoid_: round-trip, reverse conversion, re-export

**Stale**:
The state of a Projection whose Original has changed since the Projection was made. Patch-back on a Stale Projection is refused.
_Avoid_: outdated, dirty, out-of-sync

### Addressing

**Block**:
The unit a Projection is addressable by — a paragraph, a table cell, or a table. Nothing smaller carries an identity.
_Avoid_: node, element, chunk, segment

**Anchor**:
The identity a Block carries in a Projection, binding it to the specific node it came from in the Original.
_Avoid_: id, ref, pointer, marker

**Outline**:
The line-numbered map of a Projection's Blocks, which lets an agent read only the region it needs instead of the whole document.
_Avoid_: table of contents, index, summary

### Editing

**Patch Operation**:
One typed unit of intent — `Replace`, `Insert`, `Delete`, `Move`, or `Restyle` — naming what an edit does to a Block. A set of edits is always described as Patch Operations before anything is written.
_Avoid_: diff, change, mutation, edit op

**Style Catalogue**:
The closed set of formatting an edit is allowed to use, drawn from what the Original already contains. Formatting outside the Catalogue cannot be requested.
_Avoid_: theme, stylesheet, style registry

**Neighbour Inheritance**:
The rule that a newly inserted Block takes its formatting from the Block adjacent to the insertion point — mirroring what Word does when a person presses Enter.
_Avoid_: default styling, format guessing

**Dominant Format**:
The formatting used across the widest part of a Block, applied to text that no longer has formatting of its own after an edit.
_Avoid_: base style, fallback format

**Opaque Node**:
Anything in the Original the parser does not model. It is preserved verbatim and never surfaced in a Projection, so an edit cannot damage it.
_Avoid_: unknown element, passthrough, unsupported content

### Projection syntax

**Pipe Form**:
The Projection's rendering of a table whose geometry fits an ordinary Markdown table — full-width rows, no spans, no vertical merges, one paragraph per cell.
_Avoid_: simple table, markdown table

**Grid Form**:
The Projection's rendering of a table whose geometry does not fit an ordinary Markdown table. Still a pipe table, with each Anchor carrying the cell's span, vertical-merge state, and its row's leading and trailing gap.
_Avoid_: complex table, extended table

**Detached Cell**:
A cell whose content cannot be written inside a pipe cell — more than one paragraph, or a nested table — and so appears as anchored Blocks following the table.
_Avoid_: overflow cell, external cell
