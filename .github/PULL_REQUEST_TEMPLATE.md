## Issue

Closes #

## Change

Describe the intended behavior or policy change.

## Validation

List the commands, tests, examples, or manual checks performed.

## Invariants

Confirm any touched invariants:

- [ ] Deterministic replay is preserved.
- [ ] Explicit dependency identity is preserved.
- [ ] Transaction boundaries are preserved.
- [ ] Graph propagation performs no external side effects.
- [ ] Resource commands remain plain data.
- [ ] Resource and output lifecycle is scoped.
- [ ] Full-recompute testability is preserved.
- [ ] Core remains domain-neutral.
- [ ] Output remains revisioned where applicable.
- [ ] Empty-source behavior is explicit where applicable.
- [ ] Error handling avoids partial commits.

## Compatibility

- [ ] This does not add pre-1.0 backwards-compatibility shims.
- [ ] This chooses the right long-term shape over a temporary workaround.

## File Size

- [ ] Code files are under the 300 line soft limit, or the issue explains why
      they are temporarily larger.
- [ ] No code file exceeds the 500 line hard limit.
