# Examples

Examples live outside `trellis-core` in `crates/trellis-examples`.

They are normative design pressure: if an example requires domain vocabulary in
the core crate, the abstraction is wrong or the example belongs outside Trellis.

## Workspace Sync

File: `crates/trellis-examples/src/workspace_sync.rs`

Shape:

```text
active workspace
 -> permitted project set
 -> sync window collection
 -> resource plan
 -> issue board output
```

Covered behavior:

- workspace switch closes old sync windows;
- permission revoke clears forbidden rows;
- empty workspace opens no windows;
- incremental result is checked against full recompute.

## Mini Language Server

File: `crates/trellis-examples/src/mini_language_server.rs`

Shape:

```text
file contents
 -> module graph
 -> affected file set
 -> watcher resource plan
 -> diagnostics output
```

Covered behavior:

- deleting the open/root file clears diagnostics and closes watchers;
- import edge changes move affected files, watcher demand, and diagnostics;
- incremental result is checked against full recompute.

## Telemetry Dashboard

File: `crates/trellis-examples/src/telemetry_dashboard.rs`

Shape:

```text
selected customer
 -> visible device set
 -> topic subscription set
 -> resource plan
 -> telemetry card output
```

Covered behavior:

- filter shrink unsubscribes removed topics;
- empty customer/device set subscribes to nothing;
- shared topic remains live while another panel needs it;
- incremental result is checked against full recompute.
