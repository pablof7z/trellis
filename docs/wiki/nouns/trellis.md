---
type: noun-entry
slug: trellis
name: "Trellis"
origin: extracted
source_refs:
  - transcript:7-8
  - transcript:43-43
  - transcript:87-93
---

# Trellis

A deterministic reconciliation engine for Rust: state changes go in, effect plans and receipts come out. It is a control plane (not a data plane) designed for graphs of hundreds to thousands of nodes coordinating effects that each cost more than a graph clone. Its design goal is to be legible to tooling and AI agents that need to reason about why a system did what it did, without parsing logs.
