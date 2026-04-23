//! Integration tests for the Solana workbench Phase 1 surface.
//!
//! These tests drive the public API in `cadence_desktop_lib::commands::solana`
//! end-to-end — they do not spawn a real `solana-test-validator`. Instead
//! they swap in an injectable launcher and account fetcher so we can run in
//! CI without a Solana toolchain.
//!
//! The three acceptance criteria from the plan we care about here:
//!
//!   1. Spin-restore-spin cycle runs three times in a row (validator
//!      supervisor is idempotent and the snapshot store is bit-identical).
//!   2. Failover: killing the primary RPC mid-session routes the next call
//!      to the next healthy endpoint.
//!   3. Missing-toolchain state renders a predictable struct shape.
//!
//! Matches the layout of `runtime_supervisor.rs` — a single top-level file
//! with focused submodule files under `tests/solana/`.

#[path = "solana/support.rs"]
mod support;

#[path = "solana/spin_restore_cycle.rs"]
mod spin_restore_cycle;

#[path = "solana/rpc_failover.rs"]
mod rpc_failover;

#[path = "solana/toolchain_shape.rs"]
mod toolchain_shape;

#[test]
fn spin_restore_cycle_runs_three_consecutive_times() {
    spin_restore_cycle::spin_restore_cycle_runs_three_consecutive_times();
}

#[test]
fn snapshot_restore_is_bit_identical_across_process_boundary() {
    spin_restore_cycle::snapshot_restore_is_bit_identical_across_process_boundary();
}

#[test]
fn starting_second_cluster_replaces_the_first() {
    spin_restore_cycle::starting_second_cluster_replaces_the_first();
}

#[test]
fn rpc_router_fails_over_when_primary_endpoint_goes_down() {
    rpc_failover::rpc_router_fails_over_when_primary_endpoint_goes_down();
}

#[test]
fn rpc_router_recovers_when_primary_endpoint_comes_back() {
    rpc_failover::rpc_router_recovers_when_primary_endpoint_comes_back();
}

#[test]
fn rpc_router_set_endpoints_replaces_default_pool() {
    rpc_failover::rpc_router_set_endpoints_replaces_default_pool();
}

#[test]
fn toolchain_probe_returns_well_shaped_struct_on_this_host() {
    toolchain_shape::toolchain_probe_returns_well_shaped_struct_on_this_host();
}

#[test]
fn toolchain_probe_serializes_to_camel_case_json() {
    toolchain_shape::toolchain_probe_serializes_to_camel_case_json();
}
