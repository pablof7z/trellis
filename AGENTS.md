# Agent Instructions

This repository is pre-1.0. Optimize for the right long-term shape, not for
short-term compatibility or temporary fixes.

## Code Size

- Code files have a 300 line soft limit and a 500 line hard limit.
- The limit applies to source/code files only. Documentation, generated files,
  lockfiles, fixtures, vendored code, and data snapshots are exempt unless they
  are handwritten executable source.
- Treat 300+ lines as a design signal: split responsibilities before adding more
  behavior.
- Do not exceed 500 lines in a code file without first creating and linking a
  GitHub issue that explains why the boundary cannot be improved immediately.

## GitHub Issues

- Track all work as GitHub issues.
- Keep plans in GitHub issues, not local-only planning files.
- Prefer epic issues with focused subissues over one large issue that mixes
  unrelated design, implementation, and validation work.
- Every non-trivial change must reference an issue before implementation starts.
- If work discovered during implementation is meaningful, create or update the
  relevant issue instead of leaving it as an inline TODO.

## Pull Requests

- Everything except trivial changes must be delivered through a pull request
  against a GitHub issue.
- PR descriptions should link the issue, state the intended behavior change, and
  list the validation performed.
- Trivial changes are limited to typo fixes, formatting-only edits, or tiny
  documentation corrections that do not alter project direction or behavior.
- Do not bundle unrelated issues into a single PR. Use separate PRs or an epic
  issue with subissue-linked PRs.

## Pre-1.0 Compatibility Policy

- Do not preserve backwards compatibility until v1.0.
- Prefer clean API, data model, and architecture changes over compatibility
  shims, migrations, aliases, adapter layers, or "for now" fixes.
- The default question is not "what is easiest or fastest?" The default question
  is "what is the right shape of this fix?"
- When the right fix is larger, create or update the issue plan so the larger
  fix is explicit instead of landing a temporary workaround.
- Remove obsolete paths rather than keeping dead compatibility code.

## Working Standard

- Start by checking current repository state and relevant GitHub issue context.
- Keep changes focused on the issue being addressed.
- Validate the behavior or policy touched by the change before reporting done.
- Leave the repository cleaner than you found it, without rewriting unrelated
  user work.
