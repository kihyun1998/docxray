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
