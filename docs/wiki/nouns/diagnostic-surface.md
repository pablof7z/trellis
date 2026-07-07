---
type: noun-entry
slug: diagnostic-surface
name: "diagnostic surface"
origin: extracted
source_refs:
  - transcript:3-4
  - transcript:102-102
  - transcript:153-153
  - transcript:160-160
---

# diagnostic surface

A dev-build-only surface distinct from the app surface that exposes Trellis trace/audit data (receipts) via an nmp-devtools crate / MCP server; it is the ADR-0075 carve-out that allows Trellis vocabulary to cross a boundary only in dev builds, never in product-facing APIs.
