---
type: noun-entry
slug: surfacemode
name: "SurfaceMode"
origin: extracted
source_refs:
  - transcript:3370-3386
  - transcript:3780-3798
---

# SurfaceMode

An enum classifying how much Trellis leads a surface: Imperative (host decides, no graph), Shadow (graph computes beside host, host authoritative), Advisory (graph derives the value but is not long-lived), Authoritative (single long-lived daemon-held graph is the one decider, all writers routed through it), ProjectionOwned (graph additionally leads a DB projection as a summarized output-frame intent).
