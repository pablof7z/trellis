# Trellis 0.2.1 Release Notes

Trellis 0.2.1 is a resource-reconciliation patch release for the first NMP
authority-promotion consumer.

## Highlights

- Shared-key `Open` now compares command payloads instead of silently dropping
  the joining payload.
- Equal payloads coalesce explicitly: owner state grows, no duplicate host
  `Open` is emitted, and transaction trace/audit data records the join.
- Divergent payloads now fail the transaction with a typed
  `ResourcePayloadConflict`.
- Scope teardown now emits final close commands in reverse acquisition order
  inside each scope.
- `trellis-testing::ResourceLedger` applies coalesced Opens as ownership joins
  and exposes `assert_no_unexplained_coalescing`.

## API Notes

- `Transaction::commit` now requires `C: Clone + PartialEq` for resource
  command payloads.
- `TransactionResult` and `TransactionTrace` include
  `resource_coalescences`.
- `AuditEvent` is no longer `Copy` because resource coalescing audit entries
  carry a `ResourceKey`.

## Validation

- `cargo test -p trellis-core`
- `cargo test -p trellis-testing`
- `cargo check --workspace`
