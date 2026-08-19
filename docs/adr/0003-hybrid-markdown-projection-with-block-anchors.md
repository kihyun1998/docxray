# Hybrid markdown Projection, anchored at Block level only

The Projection keeps prose, headings and lists as plain markdown, and uses escape syntax only for what markdown cannot express — tables, Anchors, and styles. Anchors are attached to Blocks and nothing finer; runs carry no identity.

Markdown was not enough on its own: merged cells (`gridSpan`/`vMerge`) have no slot in markdown table syntax, and that is a grammar problem no parser can fix. But losing tables is not a reason to lose the whole grammar. Agents almost never get markdown wrong — there are hundreds of millions of examples in their training data — and that inherited accuracy is this tool's win rate on the 95% of a document that is prose.

## Considered options

- **Full custom block language** — consistent and easy to parse, but forfeits the markdown accuracy on every line of prose.
- **XML-lite** — maximum expressiveness, worst readability; the agent spends its attention on syntax instead of content.
- **Run-level anchors** — rejected. They wreck prose readability, cost tokens, and duplicate themselves whenever an agent copies a paragraph with its editor, which is a certainty rather than a risk.

## Consequences

- A custom grammar means the agent *will* eventually emit invalid syntax, so validation is mandatory rather than optional — and the reader of those error messages is the agent, not a human. Recovery is allowed only where meaning cannot change: a missing Anchor can be inferred from position, a table span mismatch must be refused. Silently repairing meaning corrupts the Original.
- Block-level anchoring promotes run formatting redistribution into a real v1 problem: when a Block's text is rewritten wholesale, the original runs' formatting has nothing to attach to. We resolve it by round-tripping through markdown inline syntax — bold, italic and underline are already in the Projection and survive for free — and dropping everything else (font, colour, shading) to the Block's Dominant Format. Text-alignment diffing would be more faithful but fails in ways a human cannot explain.
