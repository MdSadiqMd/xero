#[path = "bootstrap_contracts/support.rs"]
mod support;

#[path = "bootstrap_contracts/surface_and_packaging.rs"]
mod surface_and_packaging;

#[path = "bootstrap_contracts/serialization.rs"]
mod serialization;

#[path = "bootstrap_contracts/malformed_inputs.rs"]
mod malformed_inputs;

#[test]
fn builder_boots_and_registered_commands_return_expected_contract_shapes() {
    surface_and_packaging::builder_boots_and_registered_commands_return_expected_contract_shapes();
}

#[test]
fn config_and_capability_files_lock_the_packaged_vite_shell_and_auth_opener_permissions() {
    surface_and_packaging::config_and_capability_files_lock_the_packaged_vite_shell_and_auth_opener_permissions();
}

#[test]
fn platform_matrix_artifact_locks_cross_platform_verification_contract() {
    surface_and_packaging::platform_matrix_artifact_locks_cross_platform_verification_contract();
}

#[test]
fn tool_result_summary_contracts_remain_tagged_and_camel_case_across_nested_payloads() {
    serialization::tool_result_summary_contracts_remain_tagged_and_camel_case_across_nested_payloads();
}

#[test]
fn skill_lifecycle_payload_contracts_remain_tagged_and_camel_case() {
    serialization::skill_lifecycle_payload_contracts_remain_tagged_and_camel_case();
}

#[test]
fn skill_lifecycle_payload_contracts_fail_closed_on_unknown_stage_or_extra_fields() {
    malformed_inputs::skill_lifecycle_payload_contracts_fail_closed_on_unknown_stage_or_extra_fields();
}

#[test]
fn serialization_stays_camel_case_for_responses_events_and_errors() {
    serialization::serialization_stays_camel_case_for_responses_events_and_errors();
}

#[test]
fn malformed_inputs_fail_fast_before_runtime_logic() {
    malformed_inputs::malformed_inputs_fail_fast_before_runtime_logic();
}
