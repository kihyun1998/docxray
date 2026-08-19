# CLI surface shaped around what an agent already does well

The CLI is the only adapter in v1, with four commands:

```
docxray open    report.docx          -> report.dxr + .docxray/anchors.json
docxray apply   report.dxr           -> rewrites report.docx
docxray apply   report.dxr --dry-run -> prints the Patch Operations
docxray check   report.dxr           -> exit code and errors only
docxray outline report.dxr           -> line numbers, heading tree, Block summary
```

Designing a CLI for an agent is not a matter of adding commands; it is cutting the joint to fit capabilities the agent already has. Agents are already excellent at editing files, so handing one a real file on disk costs nothing to teach — and the same file is one a human can open and `git diff`. `outline` exists for the same reason: emitting line numbers lets the agent use its own ranged read to pull only the region it needs, so large documents work without us implementing partial reading at all. Without it an agent must load an entire Projection to fix one typo, which makes the tool useless on real documents.

## Considered options

- **MCP server first** — rejected for v1, not forever (see ADR-0002). Per-Anchor tool calls explode on large edits, failures are swallowed on the far side of stdio, and it forfeits the file-editing ability the agent already has.

## Consequences

- `check` stays separate from `--dry-run` even though both validate. `check` returns an exit code and errors; `--dry-run` prints the Patch Operations. Cheap self-validation before applying costs fewer round trips than a failed `apply` followed by a retry.
- `apply` compares a hash of the Original and refuses a Stale Projection rather than patching against a document that moved underneath it.
- The sidecar is a hidden coupling: a `.dxr` copied or moved without its `.docxray/` directory is dead. `apply` must fail loudly and say why, since the agent cannot see the coupling from the Projection alone.
- The Projection and its sidecar are working files, not deliverables. They live in the working directory and are best gitignored.
