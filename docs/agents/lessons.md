# Lessons

Concrete occasions a rule caught something real. A rule with no war story here is
still an abstraction. Referenced by `docs/agents/theflow.md`.

---

## Outline level is not enough to detect a heading

**2026-08-19, while inspecting the first real fixture — before #4 or #5 existed.**

`docs/agents/theflow.md` and the v1 spec both named `w:outlineLvl` the most
reliable heading signal, reasoning from documents saved by a localised Word: the
style *name* is translated, the style *id* is often meaningless, so the outline
level is what survives.

The first real document we opened — a Korean product manual, roughly 40 MB, 662
paragraphs, 285 images — carries **no `w:outlineLvl` anywhere**. Not on
`Heading1` through `Heading6`, not in the document body. It had been produced by
a generator rather than saved by Word: no `docProps`, `[Content_Types].xml` last
in the archive instead of first, a minimal `settings.xml` with no revision table.
Its style ids, meanwhile, were clean English `Heading1`…`Heading6`.

Had we implemented the stated rule, every heading in that document would have
rendered as body text, and the failure would have looked like a parser bug rather
than a wrong premise.

**What it changed.** Heading detection is a cascade, not a signal: outline level,
then a known built-in style id, then a normalised style name, then the `basedOn`
ancestor chain, then heuristics.

**The rule it earned.** *"Wild" is not one thing.* Word-edited documents and
generator output are strange in opposite directions — the first has revision
fragments and localised style names, the second is missing standard parts and
attributes entirely. A corpus assembled from one kind proves nothing about the
other, so fixtures are counted in *kinds of wild*, not in files.

**The second thing it showed.** The same document's runs turned out to be only 1%
mergeable once revision identifiers were ignored — its fragmentation was genuine
formatting, not the spurious splitting we had recorded as a certainty. That
gotcha is real, but it belongs to documents with editing history, and this
corpus cannot demonstrate it.

---

## A gate that cannot fail is not a gate — the wasm check did not enforce ADR-0002

**2026-08-19, #2.**

`docs/agents/theflow.md` had committed the gate matrix a few hours earlier, and
its wasm line carried a confident rationale: `cargo test --workspace` builds for
the host, so `cargo check -p docxray --target wasm32-unknown-unknown` was what
kept a `std::fs` from creeping into the IO-agnostic core (ADR-0002). The
reasoning was clean and the claim was false.

Probed it before trusting it — added a function to the core taking a
`std::path::Path` and calling `std::fs::read`, then ran the gate:

```
=== wasm check (should go red) ===
    Finished `dev` profile ... exit=0
```

`wasm32-unknown-unknown` ships a `std` where `std::fs` **compiles**; the
operations fail at runtime with `Unsupported`. So the gate we had written down as
the guardian of the project's central architectural promise would have passed a
core that read files.

**What it changed.** `scripts/check-core-io-free.sh` scans the core for
`std::fs`, `std::path`, `std::net`, `std::env`, `std::process` and `PathBuf`,
ignoring comments, and fails the build when any appear. Verified with the same
probe: red with it, green without. The wasm check stays in the matrix, with its
rationale corrected — it catches dependencies that genuinely cannot build for
wasm, which is a real failure, just not the one it was credited with.

**The rule it earned.** *Probe a new gate the way you probe a new test.* The
test-trust gate already says a test you never saw fail proves nothing; a gate is
a test of the codebase and inherits the rule. The failure mode is worse for
gates, because a green gate is read as evidence by everyone downstream and
nobody re-derives the reasoning behind it. Cost of checking: one probe. Cost of
not checking: an architectural promise that erodes invisibly until the first
WASM build, months later.

---

## The main document part is not always `word/document.xml`

**2026-08-19, #3.**

A fixture-integrity gate was added to catch truncated downloads — one fixture had
already arrived as a 14-byte 404 body while looking like a successful fetch. The
check was: is it a readable zip, and does it contain `word/document.xml`?

It failed on its first run, against `indent_word_online.docx`:

```
no word/document.xml: tests/fixtures/vendor/docx-rs/indent_word_online.docx
```

The file was fine. **The gate was wrong.** That package's main part is
`word/document2.xml`. The part name is not fixed by the specification — it is
resolved by following the `officeDocument` relationship in `_rels/.rels`, and
Word Online writes `document2.xml`. `[Content_Types].xml` declares the content
type independently of the name.

So the very first thing written against the corpus made exactly the assumption
docxray's parser must not make, and a real document caught it within minutes.

**What it changed.** `scripts/check-fixtures.sh` now resolves the main part
through the package relationships instead of by name. The fact went into the
hidden-state list in `docs/agents/theflow.md`, because part naming is state that
carries meaning and never appears in a Projection.

**The rule it earned.** *A naive assumption written into a check is still a naive
assumption.* Test scaffolding gets written quickly and reviewed loosely, and it
encodes beliefs about the domain just as firmly as the parser does — the
difference is that nobody thinks of it as domain code. This one was cheap because
the corpus contained a counterexample; had the corpus been synthesised from the
same assumption, it would have agreed with the gate all the way into production.
Which is the argument for vendoring real documents, arriving unprompted.

---

## Re-serialising XML destroyed 21 of 24 namespaces, and Word refused the file

**2026-08-19, #3.**

Trimming a 40 MB real document down to a publishable 9 KB fixture. The first
script parsed `word/document.xml` with a standard XML library, removed the
blocks it did not want, replaced the text, and wrote the tree back. Every check
we had passed: the zip was intact, the `officeDocument` relationship resolved,
the main part existed, no original text survived.

Word:

> 이 파일의 일부가 없거나 잘못되었으므로 해당 파일을 열 수 없습니다.
> 위치: 부분: /word/document.xml, 줄: 0, 열: 0

Two causes, both found by probing rather than guessing.

**The original declares 24 namespaces; the round-trip kept 3.** An XML library
preserves only the prefixes it has been told about, so `w14:paraId` came back as
`ns1:paraId` and `r:id` as `ns2:id` — 27 attributes renamed, and `standalone="yes"`
dropped from the declaration for good measure.

**The body ended with a table.** The kept blocks happened to end with `</w:tbl>`,
followed directly by `<w:sectPr>`. The schema permits it. Word does not.

**What it changed.** The script now splices the original XML as text and never
re-serialises it, and appends a paragraph when the body would otherwise end with
a table. Namespaces: 24 of 24. Auto-generated prefixes: zero. Word opens it.

**The rules it earned.** Two, and the second is the sharper one.

*The failure was ADR-0001, reproduced in a fixture script.* "Never round-trip,
splice in place" was written about the product, and a helper script violated it
within a day. What Word did to that script is exactly what it would do to a user's
document if the writer ever re-serialises a part.

*Schema-legal and Word-opens-it are different claims.* Nothing about the
table-at-end-of-body rule is in the specification, so no validator would have
caught it. Six gates were green; the seventh check was a person double-clicking a
file. That is why opening output in Word is a release-gate item in
`docs/agents/theflow.md` and not an aspiration.

---

## Seven green gates could not see that the fixture looked wrong

**2026-08-19, #3.**

The trimmed fixture passed every check we had — valid package, main part
resolving, no original text surviving, Word opening it without complaint. Then a
screenshot arrived. Several paragraphs rendered with their words stretched right
across the page:

> 구조&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;검증을&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;위한&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;문장.

Not a rendering quirk — a data defect. 104 of that document's paragraphs are
`w:jc="both"`, and the scrubbing step had replaced every `<w:t>` with a whole
sentence regardless of the original's size. Original text nodes had a median
length of 21 characters; the replacements ran 9 to 33, and a paragraph with
several runs grew until it wrapped. Justification then stretched every line but
the last.

**What it changed.** Replacement text now matches the original's length exactly —
1,129 characters in the small fixture, 9,594 in the long one. Line breaking
returns to where the real document had it, which matters beyond appearance: line
breaking is part of what decides how Word splits runs, and run splitting is
precisely what the parser has to cope with.

**The rule it earned.** *A corpus has a property no gate can check: whether it
still looks like the thing it was taken from.* Every check we ran reads XML, and
from XML "short text in a justified paragraph" is unremarkable. The defect was
only visible to someone looking at a rendered page — which is the same reason
opening output in Word is a release-gate item, arriving here from the other
direction: not "does it open" but "does it still look like a document".

---

## A byte-perfect round trip that quietly dropped 1832 bytes

**2026-08-20, #6.**

`apply` copies every untouched entry with `zip`'s `raw_copy_file`, which streams
the already-compressed bytes rather than re-deflating them. The part-by-part
comparison came back clean across all seventeen fixtures, and the runtime
pre-write check agreed. Then the output of a real CLI round trip turned out to
be **1832 bytes smaller** than its input.

Every compressed stream was byte-identical. Every CRC matched. Entry names,
order and timestamps all survived. The missing bytes were entirely in the
**local-header extra fields**:

| Field | What it is | Fixtures carrying it |
| ----- | ---------- | -------------------- |
| `0xA220` | Word's OPC **Growth Hint** — padding reserved so a part can grow without rewriting the archive | 3 |
| `0x5455`, `0x7875` | Info-ZIP extended timestamp and Unix uid/gid, left by LibreOffice | 5 |

Neither is content, and neither is load-bearing — the first is a hint by
definition and the second is meaningless in a `.docx` (and carries the author's
uid). So this is not a defect. **It is a limit, and the reason to write it down
is that no gate in this project can see it**: ADR-0008 makes decompressed parts
the comparison surface precisely so that archive metadata cannot produce false
failures, which means archive metadata also cannot produce true ones.

**The clearance and its condition.** Dropping these is fine *as long as* nothing
downstream reads them. That holds today. It stops holding the day something
needs a Growth Hint preserved, and nothing in the test suite would say so.

**The second half, found by the same probe.** Turning `raw_copy_file` off and
re-deflating instead left every test green — because compression is not a
comparison surface either. The raw-copy decision was documented and *unenforced*.
`an_untouched_part_keeps_its_compression` now pins it, using `footnotes.docx`,
the corpus's only entirely-STORED package. Red with the defect, green without,
verified both ways.

**The rule it earned.** *A comparison surface chosen to suppress false failures
suppresses true ones on the same axis.* ADR-0008 excluded zip metadata for a good
reason and the reason has not changed — but "the tests are blind here" is a
property of that choice, not an accident, and it has to be stated where the
choice is, or every later reader takes a green suite as coverage it never was.

---

## The duplicate-part guard could not see a duplicate part

**2026-08-20, #6, found by the completeness pass.**

`Package::parts()` walked the archive keeping a `HashSet` of names it had seen
and returned `Error::DuplicatePart` on a repeat, with a doc comment saying "OPC
part names are unique, so a package with two entries of the same name is refused
rather than silently resolved to one of them."

The check can never fire. `zip::ZipArchive` stores entries in an `IndexMap` keyed
by raw name, so a duplicate has already collapsed by the time `parts()` runs —
`archive.len()` *is* the deduplicated count. Every name it sees is unique by
construction.

Built a package with two `word/document.xml` entries and measured what actually
happened:

```
dup.docx      10 entries, the second word/document.xml holding "EVIL!"
docxray open  -> EVIL! <!--p0-->      the last entry silently won
docxray apply -> exit 0
dup.out.docx   9 entries
```

An entry present in the user's document was gone from the output, ADR-0006's
pre-write comparison reported no differences, and the exit code said success.
The comparison could not catch it because it compares the same already-collapsed
view on both sides.

**What it changed.** `Package::open` now reads the **end-of-central-directory
record's own entry total** and refuses when it exceeds the number of addressable
names. That is the only view that still sees both copies, because it is the
archive's own declaration rather than the reader's interpretation of it. Verified
both ways: red with the guard disabled, green with it, and green on all
seventeen real fixtures — a hardening check that refuses real documents would be
worse than the hole it closes.

**The rule it earned.** *A guard written in terms of a library's view of the data
inherits that view's blind spots.* The `seen` HashSet was not wrong about OPC —
part names really are unique, and a violation really should be refused. It was
wrong about **where** it stood: downstream of the exact normalisation that
destroys the evidence. Ask what the layer below already collapsed before writing
a check that depends on seeing it.

This is the third entry here with the same shape — the wasm check that could not
detect `std::fs`, the fixture gate that assumed `word/document.xml`, and now
this. The pattern is not carelessness; it is that a check reads as correct
exactly when its reasoning is plausible, and plausibility is what survives
review. Only the probe distinguishes them.

---

## The fixture gate failed one run in twelve, on a different fixture each time

**2026-08-20, #6, found while running the gate matrix.**

`scripts/check-fixtures.sh` reported:

```
main part 'word/document.xml' declared but missing: tests/fixtures/korean-generated-export-long.docx
```

The fixture was fine — unmodified since #3, `git status` clean, and the entry
plainly present. Run it again and it failed on `footnotes.docx`. Again, and it
failed on `paragraph.docx`. **A different file every time**, which is the tell
that the subject is not the fixture.

The check was `unzip -l "$f" | grep -qF " $target"` under `set -o pipefail`.
`grep -q` exits the instant it matches; `unzip` then writes into a closed pipe,
takes SIGPIPE and exits 141; `pipefail` reports the pipeline as failed. Whether
`unzip` had already finished writing when `grep` left is a race, so the check
passed or failed by timing.

Measured rather than argued, because an isolated one-line repro of the same
pipeline came back clean six times out of six and would have exonerated it:

| Form | Failures |
| ---- | -------- |
| `unzip -l \| grep -qF` (as written) | **1 of 12 runs** |
| capture the names, match with `case` | 0 of 12 runs |

**What it changed.** The check captures `unzip -Z1` output into a variable and
matches with a shell `case` — no pipeline, so no SIGPIPE, and no `pipefail`
interaction. Matching a whole line rather than a substring came along for free,
so a part named `notword/document.xml` can no longer satisfy a check for
`word/document.xml`. Probed both ways: still red for a package whose declared
main part is absent, still red for a file that is not a zip, green on all
seventeen real fixtures.

**The rule it earned.** *A flaky gate is worse than a missing one, because it
teaches people to re-run.* This repo already had two war stories about gates
that could not fail; this is the mirror image — a gate that failed when nothing
was wrong. Both erode the same thing. The first response to a red gate was
"which fixture did I break?", and the answer was none: the correct first
question is whether the **check** can be trusted, and the cheapest way to know
is to run it again and see whether it accuses someone else.

The bug shipped in #3 and survived every run since, because a gate that passes
eleven times out of twelve looks like a gate that passes.
