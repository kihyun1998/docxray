# Tests sit at the core crate API, comparing parts rather than bytes

There is one seam: the public API of the core crate, where a document goes in as bytes and a Projection or a rewritten document comes out. Tests drive that boundary. The CLI gets a handful of smoke checks for argument parsing and exit codes, which is not thick enough to call a seam.

The CLI is technically a higher seam, but ADR-0002 made the core IO-agnostic, which leaves the CLI holding nothing but argument parsing and file reads. Testing through a spawned process would add fixture and execution cost for no additional coverage, and it would not run under WASM, where the core is expected to work unchanged.

The central test the seam exists to express is fidelity:

- project an Original, patch back with no edits, and the document is unchanged
- project an Original, make one edit, patch back, and only that Block differs

## Consequences

- **Equality is asserted on decompressed parts, not on zip bytes.** Zip entry order, timestamps and compression level vary without meaning, so byte comparison produces false failures. The seam must therefore expose documents in a form that can be compared part by part.
- Fixture documents are a prerequisite for every test at this seam, and they cannot be produced by an agent — real files made in Korean Word, converted from HWP, containing merged cells and tracked changes, have to come from a person. Assembling the corpus gates the rest of the work.
- Internal stages (parser, Patch Operation planner, writer) are exercised through the seam rather than tested directly, so their shape stays free to change.
