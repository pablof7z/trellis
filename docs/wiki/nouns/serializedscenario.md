---
type: noun-entry
slug: serializedscenario
name: "SerializedScenario"
origin: extracted
source_refs:
  - transcript:104-104
  - transcript:155-155
  - transcript:430-430
---

# SerializedScenario

A serde-serializable Trellis script format (staged input changes as data, not closures) that carries TRACE_FORMAT_VERSION and is sufficient to reconstruct and re-run a transaction sequence against a rebuilt graph — delivered by PR #141 closing trellis#114.
