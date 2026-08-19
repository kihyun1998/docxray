# Projection and patch-back instead of conversion

The obvious shape for this tool is a converter: docx → markdown → docx. We rejected it, because everything markdown has no slot for (`spacing`, `rFonts/eastAsia`, `shd`, `sz`) is destroyed at conversion time — it is not hard to restore, it is already gone. Instead the Original stays the source of truth forever, the Projection is a disposable view, and only the Blocks that changed are rewritten. Fidelity comes from never round-tripping, not from inventing a more expressive format.

## Consequences

- Untouched content is byte-identical after a patch-back, which is the property the whole product rests on.
- Features we do not model (tracked changes, comments, fields, footnotes, floating images) survive for free: they never appear in a Projection, so an edit cannot reach them. A converter would have destroyed all of them.
- The Projection must never become the source of truth. Storing it as a file is fine; treating it as the document is the failure mode that turns this project into a worse re-implementation of OOXML.
