---
title: Workspace Dependency Management
slug: workspace-dependency-management
topic: build-config
summary: The `nostr = "0.44"` literal pin is left as a bare literal across approximately 10 crates rather than being workspace-inherited
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-03
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:c7805f5d-42c5-44b6-8eaa-ecd2453ed822
---

# Workspace Dependency Management

## Workspace Dependency Management

The `nostr = "0.44"` literal pin is left as a bare literal across approximately 10 crates rather than being workspace-inherited. This is an explicit, documented workspace-wide policy boundary driven by the split feature posture across the workspace: different crates require different feature sets of `nostr`, so a single inherited workspace definition would not suit all consumers. It is not a missed migration — the manifest ratchet enforces workspace inheritance only for `serde`, `serde_json`, `zeroize`, `rustls`, `tungstenite`, and `nmp-ownership`, not `nostr`.

<!-- citations: [^c7805-ca2a7] [^c7805-3af3a] -->
