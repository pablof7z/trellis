---
title: X-Ray Epic Parallel Workflow
slug: x-ray-epic-workflow
topic: x-ray-devtools
summary: Parallel work on the NMP X-Ray epic uses multiple subagents in isolated worktrees so they don't collide with codex1's dirty tree or each other.
tags:
  - capture
volatility: warm
confidence: medium
created: 2026-07-03
updated: 2026-07-03
verified: 2026-07-03
compiled-from: conversation
sources:
  - session:f940bd78-c4e8-413d-82a8-53aa459f690c
---

# X-Ray Epic Parallel Workflow

## Parallel Worktree Strategy

Parallel work on the NMP X-Ray epic uses multiple subagents in isolated worktrees so they don't collide with codex1's dirty tree or each other. <!-- [^f940b-74192] -->

Subagents report to the channel before and after PRing to coordinate with others. Subagents draft branches and PRs but hold off on actually opening any PR or issue until sign-off. <!-- [^f940b-f23b8] -->
