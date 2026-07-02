# ADR 0007: Resource Keys Are Structured Segments

Status: Accepted
Date: 2026-07-03

## Context

Resource commands expose `ResourceKey` separately from application command
payloads so Trellis can track ownership, sharing, teardown, stale statuses, and
audit causes.

The original `ResourceKey` was a single opaque string. That made graph ownership
visible, but it still forced hosts to encode product identity into strings and
parse it back out when handling `Close` commands, which do not carry the open
payload. Example code used slash-separated keys and `splitn`, which is lossy as
soon as a real account, route, file path, URL, or source id contains the chosen
separator.

## Decision

`ResourceKey` stores an ordered non-empty list of identity segments.

`ResourceKey::new` creates a single-segment key. `ResourceKey::from_segments`
creates a structured key for product identifiers with multiple parts.

Hosts that receive `Close` commands recover identity through `segments()` or
`segment(index)`, not by parsing `as_str()`. The string view is only a
deterministic encoded representation for diagnostics and existing
single-segment trace data.

Serde preserves current single-segment trace JSON as a string and serializes
multi-segment keys as a JSON array of strings.

## Consequences

Close commands can remain payload-free while still carrying enough structured
identity for hosts to close the right external resource.

Examples must not teach separator-based resource key parsing.

Applications can use human-readable segments without choosing escaping rules or
hashing product identity before Trellis can track it.

Resource key ordering remains deterministic because segment lists are ordered.

## Alternatives considered

Echo the open payload on close. Rejected because it duplicates app payloads in a
graph-owned lifecycle command and still leaves graph-visible identity stringly.

Keep opaque strings and document escaping. Rejected because every host would
need to get the same escaping rules right forever.

Keep opaque strings and recommend hashed keys. Rejected because hashes hide
product identity from traces and human review.

## Required tests or documentation changes

- Core tests must prove structured keys preserve segments that contain common
  separators.
- Example code must use structured segments and stop parsing flattened keys.
- Serialized trace tests must cover legacy string keys and structured key arrays.
- Semantic documentation must say resource identity is structured graph-visible
  data.
