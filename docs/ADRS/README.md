# Architecture Decision Records

This directory contains Architecture Decision Records (ADRs) for Trellis.

ADRs are required for changes that alter the semantic contract of the graph. The goal is to keep implementation decisions explicit and reviewable.

## When an ADR is required

An ADR is required for any change affecting:

- transaction phase order;
- graph ownership model;
- whether graph propagation may perform side effects;
- dependency declaration or discovery;
- resource plan semantics;
- scope teardown semantics;
- materialized output semantics;
- revision semantics;
- full-recompute requirements;
- error and rollback behavior;
- runtime or async ownership;
- public API concepts that change the core vocabulary.

An ADR is also required when introducing a new abstraction that cannot be described using the glossary.

## When an ADR is not required

An ADR is usually not required for:

- typo fixes;
- documentation clarification that does not change semantics;
- private implementation refactoring;
- tests that encode already-documented semantics;
- example-only domain concepts;
- optional adapter code that does not change core semantics.

## ADR format

Use this format:

```markdown
# ADR NNNN: Title

Status: Proposed | Accepted | Superseded | Rejected
Date: YYYY-MM-DD

## Context

What problem forces a decision?

## Decision

What are we deciding?

## Consequences

What improves? What gets worse? What constraints follow?

## Alternatives considered

What did we reject and why?

## Required tests or documentation changes

What must be added so the decision is enforceable?
```

## Status meanings

`Proposed` means the decision is under review.

`Accepted` means the decision is part of the semantic contract.

`Superseded` means a later ADR replaces this decision.

`Rejected` means the decision was considered and explicitly not adopted.

## Current ADRs

- [ADR 0001: Effects are data, not closures](0001-effects-are-data-not-closures.md)
- [ADR 0002: Resource identity is separate from command payload](0002-resource-identity-separate-from-payload.md)
