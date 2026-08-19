# Patches are a typed operation list, not direct XML splicing

Computing what changes and performing the change are separated by a typed list of Patch Operations (`Replace`, `Insert`, `Delete`, `Move`, `Restyle`). Nothing writes XML without producing that list first.

The separation is needed for testability regardless, so naming the boundary costs nothing now. What it buys is that the same list can be consumed three ways: applied directly, printed as an `apply --dry-run` preview, or rendered as Word tracked changes (`w:ins`/`w:del`).

## Consequences

- `--dry-run` is close to free, and it is the only thing that makes the tool trustworthy — "show me what the AI is about to do to my document" before it does it.
- Tracked-changes output is deferred past v1 (the `w:ins`/`w:del` placement rules are their own problem) but stays reachable. Splicing XML directly would have made it permanently impossible without rewriting the patch engine.
