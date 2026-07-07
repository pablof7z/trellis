---
type: research-record
date: 2026-07-03
session: c7805f5d-42c5-44b6-8eaa-ecd2453ed822
transcript: /Users/pablofernandez/.claude/projects/-Users-pablofernandez-src-trellis/c7805f5d-42c5-44b6-8eaa-ecd2453ed822.jsonl
source_lines: 1253-1273
agent_attribution: finder-H
has_preregistered_criteria: true
has_method: true
has_structured_report: true
characterization: "Finder-H conventions/mechanical-gate audit of PR #2866 — all gates pass (file-size, manifest inheritance, release manifest, ownership doctrine-lint, planning discipline), no violations found"
captured_at: 2026-07-03T09:42:55Z
---

Finder-H conventions/mechanical-gate audit of PR #2866 — all gates pass (file-size, manifest inheritance, release manifest, ownership doctrine-lint, planning discipline), no violations found

---

<agent-message from="finder-H">
[]

Conventions/mechanical-gate angle for PR #2866 — no quotable violations found. Coverage and why each gate passes:

- Governing files read: repo-root AGENTS.md (canonical; CLAUDE.md defers to it), root CLAUDE.md. No CLAUDE.md/AGENTS.md exists in ancestor dirs of the changed files (crates/nmp-nip60/, release/, root) — the docs/wiki/* AGENTS.md files don't govern these paths.

- File-size gate (AGENTS.md "File size": 300 soft/500 hard for source; 800 for .toml): all changed files pass. Largest source is crates/nmp-nip60/src/nip60_wallet/nutzap_send.rs at 265 LOC; ownership.rs 117; release/nmp-release.toml 375 (<800); root Cargo.toml 159; crate Cargo.toml 34. Nothing added to .file-size-baseline.

- Manifest inheritance ratchet (ci/check-release-manifest.sh:120-135): crate Cargo.toml switches to `version = { workspace = true }`, `edition = { workspace = true }`, `license = { workspace = true }`, `repository.workspace = true`. The gate regex `^version(\.workspace = true| = \{ workspace = true \})$` accepts both forms, so all four pass; `description = ` present (line 136), no `publish = false`.

- Release-manifest completeness (check-release-manifest.sh:229): nmp-nip60 is added to [workspace].members AND classified as `[[public_crates]]` (name/path present, name matches crate manifest). Every workspace member stays classified.

- Excluded-crate standalone-build gate (ci/check-excluded-crate-standalone-builds.sh): reads the exclude list dynamically; removing nmp-nip60 from `exclude` and deleting its empty `[workspace]` table (required now that it's a member, not a standalone root) is consistent — the remaining excluded crate (nmp-nip59-decrypt-only) is untouched.

- Ownership doctrine-lint (crates/nmp-testing/bin/doctrine-lint/ownership_contract_gates.rs `workspace_ownership_audit_is_clean`): new crate declares a descriptor via `nmp_ownership::declare_crate_ownership!` in ownership.rs, wired with `pub mod ownership;` in lib.rs, non-empty summary, owner_id `nmp.nip60` (matches the `nmp.nip<digits>` convention). No NMP-OWNERSHIP-COLLISION: none of its exclusive kind claims (17375/7375/7376/7374/10019/9321/38172) or its `namespace/nmp-nip60` mechanism are claimed by any other of the ~70 descriptors.

- Planning discipline (AGENTS.md:20): no PLAN.md/TODO.md/ROADMAP.md added; no `// TODO:`/`todo!()`/`unimplemented!` in added lines. Log-safe discipline: no println!/log/tracing added.

Diff at scratchpad/pr2866.diff; checkout at scratchpad/nmp.
</agent-message>
