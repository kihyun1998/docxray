# Anchors are scoped to a single Projection

Anchors are issued when a Projection is created, in Block order, as monotonically increasing identifiers recorded in the sidecar against the node each one came from. They are valid for that Projection's lifetime and no longer. Re-opening an Original produces a fresh Projection with freshly issued Anchors, and that is correct rather than a defect.

We spent some time on "how do we keep Anchor identity stable across runs" — stable numbering breaks when a Block is inserted, content hashing breaks the moment the content is edited, which is the one thing we expect to happen. The requirement itself was invented. ADR-0001 already established that a Projection is a disposable cache and never truth; nothing outlives it, so nothing needs an Anchor to outlive it either.

## Consequences

- Inserting a Block cannot disturb existing Anchors, because identifiers are issued rather than derived from position.
- **A Block with no Anchor is an `Insert`.** No syntax is needed to mark new content: an agent writing a new paragraph has no Anchor to write, so the correct thing happens when the agent does nothing special. This removes a surface the agent could get wrong.
- A Projection separated from its sidecar is meaningless, which is why `apply` must fail loudly on a missing sidecar (ADR-0006).
- Anchors must not be presented to users as durable references — no "see p42" in any output that outlives the Projection.
