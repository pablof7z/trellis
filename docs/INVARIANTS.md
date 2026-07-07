# Invariants

These invariants are part of Trellis' specification. A change that weakens one
must update this document and the linked tests in the same PR.

For public-facing claims, pair these invariant rows with the proof-packet
matrix in [Testing](TESTING.md#proof-packet-validation-matrix). That matrix
states the claim boundary, executable evidence, and limitation before the claim
appears in docs, demos, release notes, essays, or marketing copy.

## Deterministic Replay

The same graph shape and same canonical input sequence must produce the same
transaction trace, resource command order, output frame order, and final state.

Evidence:

- `crates/trellis-core/tests/transaction_phases.rs::same_input_sequence_produces_same_phase_trace`
- `crates/trellis-core/tests/oracle_model.rs::generated_model_replay_is_deterministic`
- `crates/trellis-core/tests/debug_dump.rs::debug_dump_is_deterministic`

## Stable Identity

Graph-local node and scope identities are typed, stable, and independent of
debug names. Debug names are inspection labels, not identity.

Evidence:

- `crates/trellis-core/tests/identity.rs::node_identities_are_stable`
- `crates/trellis-core/tests/identity.rs::duplicate_debug_names_do_not_define_identity`
- `crates/trellis-core/tests/scopes.rs::scopes_can_be_created_and_inspected`

## Typed API Boundaries

Typed handles constrain public API use. Input, derived, collection, set, and map
handles must not be accidentally interchangeable across incompatible value
shapes.

Evidence:

- `crates/trellis-core/tests/identity.rs::typed_handles_carry_distinct_value_types`
- `crates/trellis-core/tests/transactions.rs::non_input_node_cannot_be_set_as_input`
- `crates/trellis-core/tests/collection_boundaries.rs::set_and_unit_map_shapes_are_type_distinct`
- `crates/trellis-core/tests/collection_boundaries.rs::collection_context_reports_wrong_collection_shape`

## Transaction Atomicity

Failed transactions must not partially mutate graph state, resource ownership,
output state, or revisions.

Evidence:

- `crates/trellis-core/tests/transactions.rs::failed_transaction_does_not_partially_commit`
- `crates/trellis-core/tests/transaction_phases.rs::failed_transaction_emits_no_partial_plans_or_frames`
- `crates/trellis-core/tests/error_status.rs::plan_error_does_not_emit_partial_plan_or_mutate_ownership`
- `crates/trellis-core/tests/error_status.rs::output_error_does_not_corrupt_graph_state`

## Transaction Boundaries

Graph state changes through transaction boundaries. Transactions have stable
audit ordering, cannot be reused after closure, and aborted construction must
not leak identifiers into later graph state.

Evidence:

- `crates/trellis-core/tests/transactions.rs::closed_transaction_cannot_be_reused`
- `crates/trellis-core/tests/transactions.rs::audit_log_order_is_stable_by_node_id`
- `crates/trellis-core/tests/transactions.rs::handles_from_aborted_transactions_do_not_alias_future_nodes`

## Equality Gates Propagation

Equal writes or equal recomputes are no-ops by default. They must not advance
node revisions, produce stale diffs, emit output deltas, or replace last-change
explanations unless explicitly configured otherwise.

Evidence:

- `crates/trellis-core/tests/transactions.rs::equal_input_write_is_noop_by_default`
- `crates/trellis-core/tests/transactions.rs::equal_input_write_can_be_configured_as_change`
- `crates/trellis-core/tests/derived.rs::equal_recompute_does_not_propagate_by_default`
- `crates/trellis-core/tests/collections.rs::equal_collection_result_produces_empty_diff_and_does_not_propagate`
- `crates/trellis-core/tests/materialized_outputs.rs::equal_output_emits_no_delta_unless_configured`
- `crates/trellis-core/tests/audit_causes.rs::equal_input_write_does_not_replace_last_change_explanation`

## Explicit Dependencies

Derived, collection, planner, and output work may read declared dependencies
only. Hidden dependency discovery is not part of 0.1.

Evidence:

- `crates/trellis-core/tests/dependencies.rs::dependencies_are_inspectable_and_ordered`
- `crates/trellis-core/tests/dependencies.rs::dependency_list_rejects_duplicate_nodes`
- `crates/trellis-core/tests/dependencies.rs::graph_rejects_unknown_dependency`
- `crates/trellis-core/tests/derived_failures.rs::undeclared_dependency_read_fails_transaction`
- `crates/trellis-core/tests/collection_boundaries.rs::scalar_derived_node_cannot_depend_on_collection`
- `crates/trellis-core/tests/derived.rs::derived_self_cycle_is_rejected`

## Derived Recompute Is Pure And Ordered

Derived nodes recompute from declared dependencies only. Unaffected branches must
not recompute, derived-to-derived dependencies must follow deterministic order,
and derive failures must leave the last committed value intact.

Evidence:

- `crates/trellis-core/tests/derived.rs::derived_node_recomputes_when_input_changes`
- `crates/trellis-core/tests/derived.rs::unaffected_derived_node_does_not_recompute`
- `crates/trellis-core/tests/derived.rs::derived_node_can_depend_on_another_derived_node`
- `crates/trellis-core/tests/derived.rs::derive_error_does_not_corrupt_committed_value`

## Collections Produce Structural Diffs

Collection nodes own structural diffing. Added, removed, updated, and unchanged
members must be reported in deterministic order without requiring consumers to
recompute their own old-vs-new comparison.

Evidence:

- `crates/trellis-core/tests/collections.rs::map_collection_detects_added_removed_updated_and_unchanged`
- `crates/trellis-core/tests/collections.rs::set_collection_detects_structural_diff_and_empty_source`
- `crates/trellis-core/tests/collections.rs::collection_can_depend_on_collection_in_stable_order`
- `crates/trellis-core/tests/collections.rs::large_collection_diff_is_deterministic`
- `crates/trellis-core/tests/collection_boundaries.rs::unrelated_transaction_clears_collection_diff_without_stale_previous_state`

## Empty Means Empty

An empty source collection means empty demand. It must not imply wildcard, all,
default, or fallback resources unless an explicit fallback node models that.

Evidence:

- `crates/trellis-core/tests/resource_plans.rs::empty_collection_produces_no_open_commands`
- `crates/trellis-examples/src/workspace_sync.rs::empty_workspace_opens_no_windows`
- `crates/trellis-examples/src/telemetry_dashboard.rs::filter_shrink_unsubscribes_removed_topics`

## Resource Plans Are Data

Graph propagation returns resource commands as plain data. The core must not
execute commands, spawn work, sleep, or call host callbacks.

Evidence:

- `crates/trellis-core/tests/resource_plans.rs::added_set_members_produce_open_commands_in_deterministic_order`
- `crates/trellis-core/tests/resource_plans.rs::removed_set_members_produce_close_commands`
- `crates/trellis-core/tests/resource_plans.rs::updated_map_members_produce_replace_commands`
- `crates/trellis-core/tests/resource_plans.rs::plan_debug_includes_command_payload_when_payload_supports_debug`
- `crates/trellis-core/tests/resource_plan_boundaries.rs::planner_cannot_emit_commands_for_another_scope`
- `crates/trellis-core/tests/resource_plan_boundaries.rs::replace_without_existing_owner_fails_atomically`
- `crates/trellis-core/tests/resource_plan_boundaries.rs::late_planner_registration_opens_existing_collection_members`
- `crates/trellis-adapter/tests/boundary.rs::adapter_does_not_change_full_transaction_results`

## Resource Identity Is Structural

Resource identity must be graph-visible as `ResourceKey`, separate from
application command payload. Transition policy must be structural transaction
trace data.

Evidence:

- `crates/trellis-core/tests/resource_plans.rs::updated_map_members_produce_replace_commands`
- `crates/trellis-testing/tests/release_gate.rs::resource_ledger_detects_lifecycle_and_status_classes`
- `docs/ADRS/0002-resource-identity-separate-from-payload.md`

## Scope Owns Lifecycle

Every live resource and materialized output must be associated with a scope.
Closing a scope closes owned resources, clears owned outputs, and leaves no
orphan resources.

Evidence:

- `crates/trellis-core/tests/resource_plans.rs::scope_close_closes_owned_resources`
- `crates/trellis-core/tests/scope_teardown.rs::closing_parent_closes_child_resources_without_orphans`
- `crates/trellis-core/tests/materialized_outputs.rs::scope_close_emits_clear_frame`
- `crates/trellis-core/tests/audit_observability.rs::scope_resource_inventory_is_deterministic_and_empty_after_close`

## Closed Scopes Reject Mutation

A closed scope is terminal. Later graph construction, planner registration, or
collection changes must not attach new nodes, create child scopes, recreate
resource ownership, or run scoped planners for that closed scope.

Evidence:

- `crates/trellis-core/tests/scope_teardown.rs::closed_scope_rejects_new_children_nodes_and_resources`
- `crates/trellis-core/tests/resource_plan_boundaries.rs::closed_scope_planner_does_not_run_on_later_collection_diffs`
- `crates/trellis-core/tests/resource_plan_boundaries.rs::closing_scope_twice_is_idempotent_for_resource_plans`

## Shared Resources Close On Last Owner

If multiple scopes own the same resource key, closing one owner must not close
the resource while another owner remains.

Evidence:

- `crates/trellis-core/tests/resource_plan_boundaries.rs::shared_resource_closes_only_after_last_owner`
- `crates/trellis-core/tests/scope_teardown.rs::shared_parent_child_resource_closes_once_after_last_owner`
- `crates/trellis-examples/src/telemetry_dashboard.rs::shared_topic_closes_after_last_panel`

## Output Frames Are Revisioned

Every output frame must carry output key, scope, transaction id, revision, frame
kind, and payload. Output revisions must be coherent and monotonic per output.

Evidence:

- `crates/trellis-core/tests/materialized_outputs.rs::input_change_emits_baseline_and_delta_with_revisions`
- `crates/trellis-core/tests/materialized_outputs.rs::deltas_reconstruct_final_baseline_state`
- `crates/trellis-core/tests/materialized_outputs.rs::rebaseline_emits_coherent_current_state`
- `crates/trellis-core/tests/materialized_outputs.rs::output_frame_ordering_is_deterministic_by_key`
- `crates/trellis-core/tests/oracle_model.rs::output_delta_sequence_matches_later_rebaseline`

## Incremental Equals Full Recompute

For supported graph shapes, incremental propagation must be checkable against a
full recompute from canonical inputs and graph structure.

Evidence:

- `crates/trellis-core/tests/derived.rs::full_recompute_matches_incremental_state`
- `crates/trellis-core/tests/collections.rs::full_recompute_includes_collections`
- `crates/trellis-core/tests/oracle_model.rs::full_recompute_includes_resources_and_outputs`
- `crates/trellis-examples/src/workspace_sync.rs`
- `crates/trellis-examples/src/mini_language_server.rs`
- `crates/trellis-examples/src/telemetry_dashboard.rs`

## Audit Facts Are Structural

Auditability must be based on structural ids and trace facts, not string-only
logs.

Evidence:

- `crates/trellis-core/tests/audit_observability.rs::audit_explains_node_resource_and_output_changes`
- `crates/trellis-core/tests/audit_observability.rs::audit_uses_exact_planner_collection_for_resource_commands`
- `crates/trellis-core/tests/audit_observability.rs::late_planner_registration_explains_existing_collection_members`
- `crates/trellis-core/tests/audit_observability.rs::audit_debug_dump_is_deterministic`
- `crates/trellis-core/tests/audit_causes.rs::node_explanations_use_only_inputs_that_reach_the_node`

## Failure Transparency

Graph, derive, plan, output, and host-resource failures must be reported
deterministically. Host resource failure is later canonical input, not hidden
graph failure or retry policy.

Evidence:

- `crates/trellis-core/tests/error_status.rs::host_resource_failure_is_modeled_as_canonical_input`
- `crates/trellis-core/tests/error_status.rs::duplicate_host_status_is_an_unchanged_canonical_input`
- `crates/trellis-core/tests/error_status.rs::unsupported_resource_transition_is_host_status_not_graph_failure`
- `crates/trellis-testing/tests/release_gate.rs::resource_ledger_detects_lifecycle_and_status_classes`
- `crates/trellis-core/tests/error_status.rs::error_categories_and_audit_events_are_deterministic`
- `crates/trellis-core/tests/derived_failures.rs::full_recompute_check_detects_mismatch`
