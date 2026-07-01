# Adapter Boundary

Trellis adapters consume transaction-result data after graph propagation.

The core graph remains owned by the host actor. It does not spawn tasks, poll
futures, call callbacks, or apply resource commands during propagation.

The required boundary is:

```text
host commits graph transaction
graph returns TransactionResult
adapter applies result.resource_plan commands
adapter emits result.output_frames
host reports resource status later as canonical input
```

## Current Crate

`trellis-adapter` is runtime-neutral. It defines:

```text
ResourceCommandSink
OutputFrameSink
AdapterBoundary
RecordingResourceSink
RecordingOutputSink
```

This crate has no async runtime dependency. It is useful for tests, examples,
and future runtime-specific adapters.

## Hard Rules

- Adapters may execute returned plans.
- Adapters may emit returned frames.
- Adapters may not change graph propagation semantics.
- Adapters may not introduce hidden scheduling inside `trellis-core`.
- Adapters may not call back into graph propagation.
- Runtime-specific dependencies belong outside `trellis-core`.

## Future Runtime Crates

Runtime-specific crates such as `trellis-adapter-tokio`, tracing support, or
wasm helpers should be built on the same data boundary:

```rust
let result = tx.commit()?;
adapter.apply_transaction(result)?;
```

Async application can happen inside adapter crates, but resource statuses must
come back to the graph as later canonical inputs.
