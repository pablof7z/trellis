# Contributing

Trellis is pre-1.0. Prefer the right semantic shape over compatibility shims or
temporary fixes.

## Workflow

- Track all non-trivial work in GitHub issues.
- Keep plans in GitHub issues.
- Prefer epic issues with focused subissues over large mixed issues.
- Open a pull request against the issue.
- Merge ready pull requests unless explicitly marked hold.

## Review Checklist

Every PR should answer:

- Does this preserve deterministic replay?
- Does this preserve explicit dependency identity?
- Does this preserve transaction boundaries?
- Does this avoid external side effects during propagation?
- Does this keep resource commands as data?
- Does this require a scope for lifecycle?
- Does this preserve full-recompute testability?
- Does this avoid domain-specific core concepts?
- Does this avoid hidden async scheduling?
- Does this avoid global state?
- Does this keep output revisioned?
- Does this make empty-source behavior explicit?
- Does this avoid partial commit on error?
- Does this update semantics docs?
- Does this add tests for the invariant it touches?

## Code Size

Code files have a 300 line soft limit and a 500 line hard limit. Documentation
and generated files are exempt.

## Compatibility

Before v1.0, do not preserve backwards compatibility by default. Remove or
reshape APIs when that is the cleaner long-term design.
